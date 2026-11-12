//! The views of the history, in the editor's place: an object compared between two revisions (as git's `diff`), and its
//! lines with the revision that last changed each (as GitLens' blame). The history is
//! [`crate::patch::workspace::history`]; the Changes view, [`super::side_bar`], opens these.
//!
//! ```text
//! ┌ #0106: #2 -> working ─────────────────┐   ┌ Blame #0106 ─────────────────────────────┐
//! │   <hkobject name="#0106" ...>         │   │ template       <hkobject name="#0106"..>  │
//! │ - <hkparam name="enable">true<..>     │   │ #2 fix speed   <hkparam name="enable">..  │
//! │ + <hkparam name="enable">false<..>    │   │ working        <hkparam name="userData".. │
//! └───────────────────────────────────────┘   └───────────────────────────────────────────┘
//! ```

use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{
    PatchTool,
    workspace::history::{ObjectKey, Origin},
};

/// A diff or a blame of an object, shown in the editor's place as VS Code opens a diff in its editor:
/// while that object is the one open. `✖`, or opening another, goes back to the editor.
#[derive(Debug)]
pub(in crate::patch) struct RevisionView {
    /// The object, e.g. `#0106`.
    id: String,
    /// e.g. `#0106: #2 -> working`
    title: String,
    body: Body,
}

#[derive(Debug)]
enum Body {
    /// The lines of the diff: ` ` the same, `-` only in the older, `+` only in the newer.
    Compare(Vec<(char, String)>),
    /// Each line with what last changed it.
    Blame(Vec<(String, String)>),
}

/// The lines of `old` and `new` as a unified diff: each hunk's old lines, then its new lines.
///
/// e.g. `a\nb\n` -> `a\nc\n`: ` a`, `-b`, `+c`
pub(in crate::patch) fn unified(old: &str, new: &str) -> Vec<(char, String)> {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let mut lines = Vec::with_capacity(new_lines.len() + 8);
    let mut at_old = 0;
    for hunk in nemesis_diff::line_changes(old, new) {
        lines.extend(
            old_lines[at_old..hunk.old.start.min(old_lines.len())].iter().map(|line| (' ', (*line).to_owned())),
        );
        lines.extend(old_lines[hunk.old.clone()].iter().map(|line| ('-', (*line).to_owned())));
        lines.extend(new_lines[hunk.new.clone()].iter().map(|line| ('+', (*line).to_owned())));
        at_old = hunk.old.end;
    }
    lines.extend(old_lines[at_old.min(old_lines.len())..].iter().map(|line| (' ', (*line).to_owned())));
    lines
}

impl PatchTool {
    /// The text of object `id` of behavior `behavior` at revision `number` (`None`: in the work now).
    /// An object no revision changed is the template's.
    pub(in crate::patch) fn object_text_at(&self, behavior: usize, id: &str, number: Option<u32>) -> String {
        let Some(of) = self.workspace.behaviors.get(behavior) else { return String::new() };
        let template = || of.template_object(id).map(|object| object.text.clone()).unwrap_or_default();
        match number {
            None => of.text(id).map_or_else(template, str::to_owned),
            Some(number) => {
                let key: ObjectKey = (of.meshes_path.clone(), id.to_owned());
                self.workspace.history.state_at(number).get(&key).map_or_else(template, |text| (*text).to_owned())
            }
        }
    }

    /// Opens the diff of object `id` from revision `from` to `to` (`None`: the work now).
    pub(in crate::patch) fn open_compare(&mut self, behavior: usize, id: &str, from: u32, to: Option<u32>) {
        let (old, new) = (self.object_text_at(behavior, id, Some(from)), self.object_text_at(behavior, id, to));
        let to_name = to.map_or_else(|| "working".to_owned(), |number| format!("#{number}"));
        let title = format!("{id}: #{from} -> {to_name}");
        self.show_in_editor(behavior, id, title, Body::Compare(unified(&old, &new)));
    }

    /// Opens object `id` with `body` in the editor's place.
    fn show_in_editor(&mut self, behavior: usize, id: &str, title: String, body: Body) {
        if behavior != self.active {
            self.active = behavior;
            self.tabs.clear();
        }
        self.selected = Some(id.into());
        self.revision_view = Some(RevisionView { id: id.to_owned(), title, body });
    }

    /// Opens the blame of object `id` of behavior `behavior`, as the work has it now.
    pub(in crate::patch) fn open_blame(&mut self, behavior: usize, id: &str) {
        let Some(of) = self.workspace.behaviors.get(behavior) else { return };
        let key: ObjectKey = (of.meshes_path.clone(), id.to_owned());
        let now = of.text(id).unwrap_or_default();
        let template = of.template_object(id).map(|object| object.text.as_str());
        let history = &self.workspace.history;
        let origins = history.blame(&key, template, now);
        let label = |origin: Origin| match origin {
            Origin::Template => "template".to_owned(),
            Origin::Working => "working".to_owned(),
            Origin::Revision(number) => {
                let message = history.revision(number).map_or("", |revision| revision.message.as_str());
                format!("#{number} {message}")
            }
        };
        let lines = now.lines().zip(origins).map(|(line, origin)| (label(origin), line.to_owned())).collect();
        self.show_in_editor(behavior, id, format!("Blame {id}"), Body::Blame(lines));
    }

    /// Draws the diff or the blame in the editor's place, if one is open for object `id`. Returns
    /// whether it was drawn. Another object open closes it.
    pub(in crate::patch) fn revision_view_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, id: &str) -> bool {
        self.revision_view = self.revision_view.take().filter(|view| view.id == id);
        let Some(view) = &self.revision_view else { return false };
        let mut close = false;
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong(&view.title);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close = ui.small_button("✖").on_hover_text(i18n.t(I18nKey::RevisionViewClose)).clicked();
                });
            });
            ui.separator();
            let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 2.0;
            egui::ScrollArea::both().id_salt(("nemesis_revision_view", id)).auto_shrink(false).show(
                ui,
                |ui| match &view.body {
                    Body::Compare(lines) => compare_lines(ui, lines, row_height, i18n),
                    Body::Blame(lines) => blame_lines(ui, lines),
                },
            );
        });
        if close {
            self.revision_view = None;
        }
        true
    }
}

/// The diff, as git's colors: removed red, added green, the same plain.
fn compare_lines(ui: &mut egui::Ui, lines: &[(char, String)], row_height: f32, i18n: &I18nMap) {
    if lines.iter().all(|(mark, _)| *mark == ' ') {
        ui.weak(i18n.t(I18nKey::NemesisNoChange));
    }
    for (mark, line) in lines {
        let (color, fill) = match mark {
            '-' => (ui.visuals().error_fg_color, egui::Color32::from_rgba_unmultiplied(200, 60, 60, 40)),
            '+' => (egui::Color32::from_rgb(0x73, 0xc9, 0x91), egui::Color32::from_rgba_unmultiplied(60, 180, 90, 40)),
            _ => (ui.visuals().text_color(), egui::Color32::TRANSPARENT),
        };
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_height), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, fill);
        let text = format!("{mark} {line}");
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        ui.painter().text(rect.left_center(), egui::Align2::LEFT_CENTER, text, font, color);
    }
}

/// `#2 fix speed   <hkparam ...>`: the revision in a column of its own, as GitLens' blame.
fn blame_lines(ui: &mut egui::Ui, lines: &[(String, String)]) {
    egui::Grid::new("nemesis_blame").striped(true).show(ui, |ui| {
        let mut last = "";
        for (origin, line) in lines {
            // Only where it changes: a run of lines of one revision reads as a block.
            let shown = if origin == last { String::new() } else { origin.clone() };
            ui.label(egui::RichText::new(shown).small().weak());
            ui.label(egui::RichText::new(line).monospace());
            ui.end_row();
            last = origin;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_diff_has_the_old_lines_then_the_new() {
        let lines = unified("a\nb\nd\n", "a\nc\nd\ne\n");
        let marks: Vec<String> = lines.iter().map(|(mark, line)| format!("{mark}{line}")).collect();
        assert_eq!(marks, [" a", "-b", "+c", " d", "+e"]);
    }
}
