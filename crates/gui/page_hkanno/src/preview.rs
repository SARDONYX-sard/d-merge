//! XML preview of the edited file + "jump to the same annotation" sync.
//!
//! # Why a worker thread + debounce?
//! Generating the preview re-serializes the whole hkx as XML, which is too slow to do on
//! every keystroke. After an edit we wait [`DEBOUNCE`] and then build it on a thread.
//!
//! # Sync jump
//! The n-th `trackName:` line of the editor corresponds to the n-th `<hkparam name="trackName">`
//! of the XML, and the n-th `<time> ...` line to the n-th `<hkparam name="time">`.
//! When the editor cursor moves to such a line, the preview scrolls there.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use egui_code_editor::languages::Xml;
use fast_hash::HashMap;
use parking_lot::Mutex;
use serde_hkx_for_gui::hkanno::{Format, Hkanno};

/// Quiet time after the last edit before the preview is rebuilt.
const DEBOUNCE: Duration = Duration::from_millis(300);

type Outcome = Result<String, String>;

#[derive(Default)]
pub(crate) struct Preview {
    /// Preview XML split into lines (for fast, virtualized drawing).
    lines: Vec<String>,

    /// Error of the last build (shown instead of the title).
    pub(crate) error: Option<String>,

    /// Set by an edit; the build starts once [`DEBOUNCE`] has passed.
    requested_at: Option<Instant>,

    /// `Some` while a worker is running.
    pending: Option<Arc<Mutex<Option<Outcome>>>>,

    /// editor line -> preview line (both 0-based).
    jump_map: HashMap<usize, usize>,

    /// Preview line to scroll to on the next draw.
    scroll_to: Option<usize>,

    /// Preview line highlighted as the jump target.
    highlighted: Option<usize>,
}

impl Preview {
    /// Marks the preview as outdated (call after each edit).
    pub(crate) fn request(&mut self) {
        self.requested_at = Some(Instant::now());
    }

    /// Starts/finishes builds. Call every frame while the preview is visible.
    ///
    /// `hkanno`/`bytes`/`input` describe the current document; `editor_text` is used for the jump map.
    pub(crate) fn poll(
        &mut self,
        ctx: &egui::Context,
        hkanno: impl FnOnce() -> Hkanno<'static>,
        bytes: &Arc<Vec<u8>>,
        input: &str,
        editor_text: &str,
    ) {
        // ── Finished? ──────────────────────────────────────────────────────────
        let finished = self.pending.as_ref().and_then(|slot| slot.lock().take());
        if let Some(outcome) = finished {
            self.pending = None;
            match outcome {
                Ok(xml) => {
                    self.lines = xml.lines().map(str::to_owned).collect();
                    self.jump_map = build_jump_map(editor_text, &self.lines);
                    self.error = None;
                }
                Err(err) => self.error = Some(err),
            }
        }

        // ── Start a new build? ──────────────────────────────────────────────────
        let Some(requested_at) = self.requested_at else { return };
        if self.pending.is_some() {
            return; // One build at a time; the request stays and runs after this one.
        }
        let waited = requested_at.elapsed();
        if waited < DEBOUNCE {
            ctx.request_repaint_after(DEBOUNCE - waited);
            return;
        }

        self.requested_at = None;
        let slot = Arc::new(Mutex::new(None));
        self.pending = Some(Arc::clone(&slot));

        let hkanno = hkanno();
        let bytes = Arc::clone(bytes);
        let input = input.to_string();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let outcome = hkanno
                .update_hkx_bytes(&bytes, Format::Xml, std::path::Path::new(&input))
                .map_err(|e| e.to_string())
                .and_then(|xml| String::from_utf8(xml).map_err(|e| e.to_string()));
            *slot.lock() = Some(outcome);
            ctx.request_repaint();
        });
    }

    /// Scrolls to the XML line matching `editor_line`, if there is one.
    pub(crate) fn jump_from_editor_line(&mut self, editor_line: usize) {
        if let Some(&target) = self.jump_map.get(&editor_line)
            && self.highlighted != Some(target)
        {
            self.highlighted = Some(target);
            self.scroll_to = Some(target);
        }
    }

    /// Draws the XML (only the visible rows are laid out, so huge files stay fast).
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui) {
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let row_height = ui.fonts_mut(|f| f.row_height(&font));
        let row_step = row_height + ui.spacing().item_spacing.y;

        let mut area = egui::ScrollArea::both().id_salt("hkanno_preview").auto_shrink(false);
        if let Some(target) = self.scroll_to.take() {
            // Put the target line in the middle of the view.
            let offset = (target as f32).mul_add(row_step, -ui.available_height() / 2.0);
            area = area.vertical_scroll_offset(offset.max(0.0));
        }

        let highlight = ui.visuals().selection.bg_fill.gamma_multiply(0.5);
        area.show_rows(ui, row_height, self.lines.len(), |ui, rows| {
            for i in rows {
                // Colored per visible row only, so huge files stay fast.
                let mut job = egui_code_editor::line_job(ui, &self.lines[i], &Xml);
                if self.highlighted == Some(i) {
                    for section in &mut job.sections {
                        section.format.background = highlight;
                    }
                }
                ui.add(egui::Label::new(job).extend());
            }
        });
    }
}

/// editor line -> XML line, matched by order of appearance (see the module docs).
fn build_jump_map(editor_text: &str, xml_lines: &[String]) -> HashMap<usize, usize> {
    let (mut editor_tracks, mut editor_times) = (Vec::new(), Vec::new());
    for (i, line) in editor_text.lines().enumerate() {
        let line = line.trim();
        if line.get(..9).is_some_and(|head| head.eq_ignore_ascii_case("trackName")) {
            editor_tracks.push(i);
        } else if line.split_whitespace().next().is_some_and(|t| t.parse::<f32>().is_ok()) {
            editor_times.push(i);
        }
    }

    let (mut xml_tracks, mut xml_times) = (Vec::new(), Vec::new());
    for (i, line) in xml_lines.iter().enumerate() {
        if line.contains(r#"<hkparam name="trackName">"#) {
            xml_tracks.push(i);
        } else if line.contains(r#"<hkparam name="time">"#) {
            xml_times.push(i);
        }
    }

    editor_tracks
        .into_iter()
        .zip(xml_tracks)
        .chain(editor_times.into_iter().zip(xml_times))
        .collect()
}
