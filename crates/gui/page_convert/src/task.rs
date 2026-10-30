//! Background conversion and per-file progress.
//!
//! # Flow
//! 1. [`ConvertTask::start`] spawns `serde_hkx_for_gui::convert` on the app's tokio runtime.
//! 2. The converter reports each file's [`Status`] through a callback -> stored in `statuses`
//!    (keyed by `hash_djb2(path)`, the id the converter uses) and a repaint is requested.
//! 3. [`ConvertTask::take_finished`] hands the final result to the UI thread exactly once.

use std::sync::Arc;

use fast_hash::HashMap;
use gui_shared::settings::convert::OutFormat;
use parking_lot::Mutex;
use serde_hkx_for_gui::{hash::hash_djb2, hkanno::Format, status::Status};

/// Final result: `Err` holds every failed file's message, one per line.
type Outcome = Result<(), String>;

/// Converter state shared with the worker.
#[derive(Default)]
pub(crate) struct ConvertTask {
    /// `hash_djb2(path)` -> latest status.
    statuses: Arc<Mutex<HashMap<u32, Status>>>,

    /// `Some` while a conversion runs. The worker writes the outcome into it when done.
    running: Option<Arc<Mutex<Option<Outcome>>>>,
}

impl ConvertTask {
    pub(crate) const fn is_running(&self) -> bool {
        self.running.is_some()
    }

    /// Status of one input path (`None` = not converted yet).
    pub(crate) fn status_of(&self, path: &str) -> Option<Status> {
        self.statuses.lock().get(&hash_djb2(path)).copied()
    }

    /// Forgets all statuses, e.g., when the selection changes.
    pub(crate) fn clear_statuses(&self) {
        self.statuses.lock().clear();
    }

    /// Starts converting `inputs` (files or directories).
    ///
    /// - `output`: output directory; empty = next to each input
    /// - `roots`: tree-mode roots, stripped from the output path to keep the directory structure
    ///   e.g., root `C:/mod`, input `C:/mod/meshes/a.hkx`, output `D:/out` -> `D:/out/meshes/a.xml`
    pub(crate) fn start(
        &mut self,
        rt: &tokio::runtime::Runtime,
        ctx: &egui::Context,
        inputs: Vec<String>,
        output: String,
        format: OutFormat,
        roots: Option<Vec<String>>,
    ) {
        self.clear_statuses();

        let slot = Arc::new(Mutex::new(None));
        self.running = Some(Arc::clone(&slot));

        let statuses = Arc::clone(&self.statuses);
        let repaint_ctx = ctx.clone();
        let on_status = move |payload: serde_hkx_for_gui::status::Payload| {
            statuses.lock().insert(payload.path_id, payload.status);
            repaint_ctx.request_repaint();
        };

        let ctx = ctx.clone();
        rt.spawn(async move {
            let result = serde_hkx_for_gui::convert(
                inputs,
                Some(output),
                to_format(format),
                roots,
                on_status,
            )
            .await
            .map_err(|err| err.to_string());
            *slot.lock() = Some(result);
            ctx.request_repaint();
        });
    }

    /// Returns the outcome once the conversion has finished (then `is_running()` becomes `false`).
    pub(crate) fn take_finished(&mut self) -> Option<Outcome> {
        let outcome = self.running.as_ref()?.lock().take()?;
        self.running = None;
        Some(outcome)
    }
}

/// Settings enum -> converter enum.
pub(crate) const fn to_format(format: OutFormat) -> Format {
    match format {
        OutFormat::Amd64 => Format::Amd64,
        OutFormat::Win32 => Format::Win32,
        OutFormat::Xml => Format::Xml,
        OutFormat::Json => Format::Json,
    }
}

/// Small icon for a file's progress (nothing while pending).
pub(crate) fn status_icon(ui: &mut egui::Ui, status: Option<Status>) {
    match status {
        Some(Status::Processing) => {
            ui.add(egui::Spinner::new().size(14.0));
        }
        Some(Status::Done) => {
            ui.colored_label(egui::Color32::from_rgb(100, 200, 120), "✔");
        }
        Some(Status::Error) => {
            ui.colored_label(ui.visuals().error_fg_color, "✖");
        }
        Some(Status::Pending) | None => {}
    }
}
