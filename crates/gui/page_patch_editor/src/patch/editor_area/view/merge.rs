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
//!   ▼ → #flinch$2  hkbClipGenerator        <- what a value points to, as this mod has it
//!       <hkobject name="#flinch$2" ...>
//! ──────────────────────────────────
//! Impact [Accept]
//! ```
//! Two mods pointing to their own objects differ in what those are, which the ids alone do not tell:
//! each pointed object is shown under its mod, to compare them.

use egui_code_editor::languages::Xml;
use egui_widgets::{
    HoverMarkdown as _,
    shadcn::{Button, Icon},
};
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{
    FocusAt,
    editor::{merge, resolve},
    editor_area::EditorPane,
    tool::work_state::Work,
    workspace::{Behavior, behavior::class_attribute},
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
            let template = &work.workspace.behaviors[active];
            panel.show(ui, |ui| panel_ui(ui, i18n, (id, template), &values)).inner
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

/// The panel's insides. `template`: the behavior of object `id`, which has the objects no mod changes.
/// Returns `(what was picked, ✖ clicked)`.
fn panel_ui(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    (id, template): (&str, &Behavior),
    values: &resolve::MergeValues,
) -> (Option<MergePick>, bool) {
    let mut picked = None;
    let mut close = false;

    ui.horizontal(|ui| {
        ui.strong(i18n.t(I18nKey::MergeTitle)).on_hover_markdown(i18n.t(I18nKey::MergeHover));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            close = ui.add(Button::new("").icon(Icon::X).small()).clicked();
            let all = Button::new(i18n.t(I18nKey::MergeAcceptAll)).small();
            if ui.add(all).on_hover_markdown(i18n.t(I18nKey::MergeAcceptAllHover)).clicked() {
                picked = Some(MergePick::Accept(merge::all_lines(values)));
            }
        });
    });
    ui.add(egui::Label::new(egui::RichText::new(&values.path).monospace().weak()).truncate());
    ui.separator();

    // The bars always show: the values and the objects they point to are often longer and wider than
    // the panel.
    let scroll = egui::ScrollArea::both()
        .id_salt(("nemesis_merge_values", id))
        .auto_shrink(false)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible);
    scroll.show(ui, |ui| {
        if let Some(pick) = values_ui(ui, i18n, template, values) {
            picked = Some(pick);
        }
    });

    (picked, close)
}

/// The original, then each mod below it, as wide as the panel (side by side they were cut off).
fn values_ui(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    template: &Behavior,
    values: &resolve::MergeValues,
) -> Option<MergePick> {
    ui.vertical(|ui| {
        {
            ui.weak(i18n.t(I18nKey::MergeOriginal));
            for line in &values.original {
                ui.add(egui::Label::new(egui_code_editor::line_job(ui, line.trim_start(), &Xml)).extend());
            }
            pointed_ui(ui, i18n, template, ("", &values.original_pointed));
        }

        let mut picked = None;
        for side in &values.sides {
            ui.separator();
            if let Some(pick) = side_ui(ui, i18n, side, &values.original) {
                picked = Some(pick);
            }
            pointed_ui(ui, i18n, template, (&side.name, &side.pointed));
        }
        picked
    })
    .inner
}

/// The objects a side's values point to, each folded under `→ #flinch$2  hkbClipGenerator`. One the
/// mod has no file of is the template's. `salt`: the side, as two sides may point to one id.
fn pointed_ui(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    template: &Behavior,
    (salt, pointed): (&str, &[resolve::PointedObject]),
) {
    for object in pointed {
        let text = {
            let of_template = || template.template_object(&object.id).map(|template| template.text.as_str());
            object.text.as_deref().or_else(of_template)
        };
        let title = format!("→ {}  {}", object.id, text.map_or("", class_attribute));

        let header = egui::CollapsingHeader::new(egui::RichText::new(title).monospace())
            .id_salt(("nemesis_merge_pointed", salt, &object.id))
            .default_open(true)
            .show(ui, |ui| match text {
                Some(text) => object_lines_ui(ui, text),
                None => {
                    ui.weak(i18n.t(I18nKey::MergePointedMissing));
                }
            });
        header.header_response.on_hover_markdown(i18n.t(I18nKey::MergePointedHover));
    }
}

/// The lines of an object, less the indentation of its first line, so that it starts at the left.
///
/// Long lines (paths, names) are not wrapped: the panel's scroll area goes both ways. No scroll area
/// of its own: one inside the panel's takes the wheel, hides its bar and cuts off what is below it.
fn object_lines_ui(ui: &mut egui::Ui, text: &str) {
    let first = text.lines().next().unwrap_or_default();
    let indent = &first[..first.len() - first.trim_start().len()];
    for line in text.lines() {
        let line = line.strip_prefix(indent).unwrap_or(line);
        ui.add(egui::Label::new(egui_code_editor::line_job(ui, line, &Xml)).extend());
    }
}

/// One mod: its name and `Accept`, then its lines, each with `+`; what it changes is tinted.
fn side_ui(ui: &mut egui::Ui, i18n: &I18nMap, side: &resolve::MergeSide, original: &[String]) -> Option<MergePick> {
    let mut picked = None;

    ui.horizontal(|ui| {
        ui.strong(&side.name);
        let accept = Button::new(i18n.t(I18nKey::MergeAccept)).small();
        if ui.add(accept).on_hover_markdown(i18n.t(I18nKey::MergeAcceptHover)).clicked() {
            picked = Some(MergePick::Accept(side.lines.clone()));
        }
    });

    if side.lines.is_empty() {
        ui.weak(i18n.t(I18nKey::MergeRemoves));
    }
    for line in &side.lines {
        ui.horizontal(|ui| {
            let insert = ui.add(Button::new("").icon(Icon::Plus).small());
            if insert.on_hover_markdown(i18n.t(I18nKey::MergeInsertHover)).clicked() {
                picked = Some(MergePick::Insert(vec![line.clone()]));
            }
            let label = ui.add(egui::Label::new(egui_code_editor::line_job(ui, line.trim_start(), &Xml)).extend());

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
