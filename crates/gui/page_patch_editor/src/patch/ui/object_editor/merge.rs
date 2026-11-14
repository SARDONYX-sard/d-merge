//! The mods' values at a conflict, beside the editor, as git's merge editor: see
//! [`crate::patch::editor::merge`].
//!
//! ```text
//! Merge   [Accept All] ✖
//! /root/#0106/value
//! Original
//!   <hkparam name="value">1</hkparam>
//! ──────────────────────────────────
//! Flinching [Accept]
//!   + <hkparam name="value">2</hkparam>    <- tinted: the mod changes it; `+` puts it below the cursor
//! ──────────────────────────────────
//! Impact [Accept]
//! ```

use egui_code_editor::languages::Xml;
use egui_widgets::shadcn::Button;
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{
    FocusAt,
    editor::{merge, resolve},
    tool::{editing::EditorPane, work_state::Work},
};

/// What the panel asked for.
#[derive(Debug)]
enum MergePick {
    /// These lines below the cursor.
    Insert(Vec<String>),
    /// These lines in place of the conflict's: see [`merge::replace_block`].
    Accept(Vec<String>),
}

impl EditorPane {
    /// Draws the panel of `id`'s conflict, and does what it asked for to object `id` of behavior
    /// `active` of `work`.
    pub(super) fn merge_panel(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (work, active): (&mut Work, usize),
        id: &str,
    ) {
        let Some(values) = self.merge_values.take() else { return };

        let (picked, close) = {
            let panel_id = if work.user_patch { "user_patch_merge" } else { "nemesis_merge" };
            let panel = egui::Panel::right(panel_id).default_size(620.0).resizable(true);
            panel.show(ui, |ui| panel_ui(ui, i18n, id, &values)).inner
        };

        if let Some(pick) = picked {
            let merged = merged_text(&self.text, self.cursor_line, (&values, &self.merge_accepted), pick);
            if let Some(accepted) = merged.accepted {
                merge::remember(&mut work.workspace.resolutions, &values, accepted.clone());
                self.merge_accepted = accepted;
            }
            self.text = merged.text;
            work.workspace.behaviors[active].set_text(id, &self.text);
            self.preview = None;
            work.object_changed(id);
            self.focus = Some(FocusAt::Line(merged.line));
        }

        if !close {
            self.merge_values = Some(values);
        }
    }
}

/// The object's text after a pick, and where the cursor goes.
struct Merged {
    text: String,
    line: usize,
    /// `Accept` put these lines in place of the conflict's: remembered for the next same conflict.
    accepted: Option<Vec<String>>,
}

/// The text after `pick`: `+` puts lines below the cursor; `Accept` puts them in place of the
/// conflict's (`accepted`: the lines taken last), or below the cursor if they are not found.
fn merged_text(
    text: &str,
    cursor_line: usize,
    (values, accepted): (&resolve::MergeValues, &[String]),
    pick: MergePick,
) -> Merged {
    let below = |lines: &[String]| {
        let (text, line) = resolve::insert_lines(text, cursor_line, lines);
        Merged { text, line, accepted: None }
    };
    match pick {
        MergePick::Insert(lines) => below(&lines),
        MergePick::Accept(lines) => {
            let candidates = merge::candidates(values, accepted);
            match merge::replace_block(text, &candidates, &lines) {
                Some(text) => Merged { text, line: cursor_line, accepted: Some(lines) },
                None => below(&lines),
            }
        }
    }
}

/// The panel's insides. Returns `(what was picked, ✖ clicked)`.
fn panel_ui(ui: &mut egui::Ui, i18n: &I18nMap, id: &str, values: &resolve::MergeValues) -> (Option<MergePick>, bool) {
    let mut picked = None;
    let mut close = false;

    ui.horizontal(|ui| {
        ui.strong(i18n.t(I18nKey::NemesisMergeTitle)).on_hover_text(i18n.t(I18nKey::NemesisMergeHover));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            close = ui.small_button("✖").clicked();
            let all = Button::new(i18n.t(I18nKey::MergeAcceptAll)).small();
            if ui.add(all).on_hover_text(i18n.t(I18nKey::MergeAcceptAllHover)).clicked() {
                picked = Some(MergePick::Accept(merge::all_lines(values)));
            }
        });
    });
    ui.add(egui::Label::new(egui::RichText::new(&values.path).monospace().weak()).truncate());
    ui.separator();

    egui::ScrollArea::both().id_salt(("nemesis_merge_values", id)).auto_shrink(false).show(ui, |ui| {
        if let Some(pick) = values_ui(ui, i18n, values) {
            picked = Some(pick);
        }
    });

    (picked, close)
}

/// The original, then each mod below it, as wide as the panel (side by side they were cut off).
fn values_ui(ui: &mut egui::Ui, i18n: &I18nMap, values: &resolve::MergeValues) -> Option<MergePick> {
    ui.vertical(|ui| {
        ui.weak(i18n.t(I18nKey::NemesisMergeOriginal));
        for line in &values.original {
            ui.add(egui::Label::new(egui_code_editor::line_job(ui, line.trim_start(), &Xml)));
        }

        let mut picked = None;
        for side in &values.sides {
            ui.separator();
            if let Some(pick) = side_ui(ui, i18n, side, &values.original) {
                picked = Some(pick);
            }
        }
        picked
    })
    .inner
}

/// One mod: its name and `Accept`, then its lines, each with `+`; what it changes is tinted.
fn side_ui(ui: &mut egui::Ui, i18n: &I18nMap, side: &resolve::MergeSide, original: &[String]) -> Option<MergePick> {
    let mut picked = None;

    ui.horizontal(|ui| {
        ui.strong(&side.name);
        let accept = Button::new(i18n.t(I18nKey::MergeAccept)).small();
        if ui.add(accept).on_hover_text(i18n.t(I18nKey::MergeAcceptHover)).clicked() {
            picked = Some(MergePick::Accept(side.lines.clone()));
        }
    });

    if side.lines.is_empty() {
        ui.weak(i18n.t(I18nKey::NemesisMergeRemoves));
    }
    for line in &side.lines {
        ui.horizontal(|ui| {
            if ui.small_button("+").on_hover_text(i18n.t(I18nKey::NemesisMergeInsertHover)).clicked() {
                picked = Some(MergePick::Insert(vec![line.clone()]));
            }
            let label = ui.add(egui::Label::new(egui_code_editor::line_job(ui, line.trim_start(), &Xml)));

            // What this mod changes, as a diff's added line.
            let is_original = original.iter().any(|original| original.trim() == line.trim());
            if !is_original {
                let tint = egui::Color32::from_rgba_unmultiplied(60, 180, 90, 40);
                ui.painter().rect_filled(label.rect.expand(1.0), 0.0, tint);
            }
        });
    }

    picked
}
