//! The window of one mod's conflicts. What they are is in [`crate::conflicts`].
//!
//! ```text
//! ┌ Conflicts: Modern Combat ──────────────────────────────────┐
//! │ [filter........]  + 12  - 3                                │
//! │ 0_master                                                   │
//! │   + #0106/hkbModifierList/enable  Bool                     │
//! │   - #0241/hkbClipGenerator/mode   Enum(Int8)               │  <- a click shows it below
//! │ _1stperson/0_master                                        │
//! │   + #0106/hkbModifierList/enable  Bool                     │
//! │ ────────────────────────────────────────────────────────── │
//! │ Other Mod  In effect          [Use this] [Edit...]         │  <- the value the merge keeps
//! │ false                                                      │
//! │ Modern Combat                 [Use this] [Edit...]         │
//! │ true                                                       │
//! └────────────────────────────────────────────────────────────┘
//! ```
//! `Use this` decides: that value goes to the user patch, a mod that loads last and so wins, and the
//! window stays for the next conflict. `Edit...` also goes to the editor, to change it by hand.
//!
//! It starts as small as its rows allow. Only the user makes it bigger: what is drawn must never ask
//! for more than the window has, or the window grows a bit more each frame, down to the screen's edge.

use egui_widgets::shadcn::{Button, TextEdit};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    patch::ResolveRequest,
};

use super::PatchView;
use crate::conflicts::{ConflictScan, Point};

/// How far a conflict is to the right of its template.
const INDENT: f32 = 12.0;
/// The window's width when it opens.
const WINDOW_WIDTH: f32 = 720.0;
/// What the window has besides the rows: the title bar, the filter.
const WINDOW_CHROME: f32 = 90.0;
/// The most rows' height a window opens with.
const MAX_OPENING_LIST: f32 = 320.0;
/// The values of a picked conflict get at least this: the window grows for them if it must.
const MIN_VALUES_HEIGHT: f32 = 140.0;

/// The height of `rows` rows of `row_height`, with the space between them.
fn rows_height(ui: &egui::Ui, rows: usize, row_height: f32) -> f32 {
    #[expect(clippy::cast_precision_loss, reason = "far fewer rows than that")]
    let rows = rows as f32;
    rows * (row_height + ui.spacing().item_spacing.y)
}

impl PatchView<'_> {
    /// Draws the window if a mod is being looked at.
    pub(super) fn ui_conflict_window(&mut self, ctx: &egui::Context) {
        let i18n = self.ctx.i18n;
        let Some(mod_id) = self.page.conflicts.inspected.clone() else { return };
        let name = self.ctx.settings.mod_list().iter().find(|item| item.id == mod_id).map_or("", |item| &item.name);
        let title = format!("{}: {name}", i18n.t(I18nKey::ConflictTitle));

        let mut open = true;
        let mut request = None;
        let rows = self.page.conflicts.current.as_ref().map_or(0, |found| found.of_mod(&mod_id).len());
        #[expect(clippy::cast_precision_loss, reason = "far fewer rows than that")]
        let list = (rows as f32 * 22.0).clamp(22.0, MAX_OPENING_LIST);
        let window = egui_widgets::window_near_pointer(ctx, title).open(&mut open);
        window.default_size([WINDOW_WIDTH, list + WINDOW_CHROME]).resizable(true).show(ctx, |ui| {
            request = conflict_ui(ui, i18n, &mut self.page.conflicts, &mod_id);
        });
        if !open {
            self.page.conflicts.inspected = None;
        }
        if request.is_some() {
            self.page.resolve = request;
        }
    }
}

fn conflict_ui(ui: &mut egui::Ui, i18n: &I18nMap, scan: &mut ConflictScan, mod_id: &str) -> Option<ResolveRequest> {
    if scan.is_scanning() {
        ui.horizontal(|ui| {
            ui.add(egui::Spinner::new());
            ui.label(i18n.t(I18nKey::ConflictScanning));
        });
        return None;
    }
    let found = scan.current.clone()?;
    if scan.is_stale() {
        ui.colored_label(ui.visuals().warn_fg_color, i18n.t(I18nKey::ConflictStale));
    }
    let points = found.of_mod(mod_id);
    if points.is_empty() {
        ui.weak(i18n.t(I18nKey::ConflictNone));
        return None;
    }
    let mark = scan.mark_of(mod_id);
    ui.horizontal(|ui| {
        ui.add(
            TextEdit::singleline(&mut scan.filter).hint_text(i18n.t(I18nKey::ConflictFilterHint)).desired_width(280.0),
        );
        ui.label(format!("+ {}", mark.wins)).on_hover_text(i18n.t(I18nKey::ConflictWinsHover));
        ui.label(format!("- {}", mark.loses)).on_hover_text(i18n.t(I18nKey::ConflictLosesHover));
    });

    let filter = scan.filter.trim().to_lowercase();
    let shown: Vec<&Point> = points
        .iter()
        .copied()
        .filter(|point| filter.is_empty() || point.path.to_lowercase().contains(&filter))
        .collect();
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 4.0;
    // No more than its rows need. Until a conflict is picked, all there is; then half of it.
    let is_picked = shown.iter().any(|point| scan.is_selected(point));
    let available = if is_picked { ui.available_height() / 2.0 } else { ui.available_height() };
    let rows = list_rows(&shown);
    let list_height = available.min(rows_height(ui, rows.len(), row_height));
    egui::ScrollArea::both().id_salt("conflict_rows").max_height(list_height).auto_shrink(false).show_rows(
        ui,
        row_height,
        rows.len(),
        |ui, range| {
            for row in &rows[range] {
                ui.horizontal(|ui| {
                    ui.set_height(row_height);
                    match row {
                        ListRow::Template(target) => {
                            let name = egui::RichText::new(template_name(target)).monospace().strong();
                            ui.add(egui::Label::new(name).selectable(false));
                        }
                        ListRow::Point(point) => {
                            ui.add_space(INDENT);
                            let is_winner = scan.winner_of(&found, point).is_some_and(|winner| winner.id == mod_id);
                            let glyph = if is_winner { "+" } else { "-" };
                            let selected = scan.is_selected(point);
                            if ui.add(egui::Button::selectable(selected, row_text(ui, glyph, point))).clicked() {
                                scan.select(point);
                            }
                        }
                    }
                });
            }
        },
    );
    // After this, so that nothing is drawn below a list that took all the height.
    let point = shown.iter().find(|point| scan.is_selected(point))?;
    ui.separator();
    point_ui(ui, i18n, scan, &found, point)
}

/// A row of the list: a template, then its conflicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListRow<'a> {
    Template(&'a str),
    Point(&'a Point),
}

/// `points`: sorted by target, as [`crate::conflicts::Conflicts::read`] leaves them.
fn list_rows<'a>(points: &[&'a Point]) -> Vec<ListRow<'a>> {
    let mut rows = Vec::with_capacity(points.len() + 8);
    for group in points.chunk_by(|a, b| a.target == b.target) {
        rows.push(ListRow::Template(&group[0].target));
        rows.extend(group.iter().map(|point| ListRow::Point(point)));
    }
    rows
}

/// `+ #0106/hkbModifierList/enable  Bool`, with the type in the color of class names.
fn row_text(ui: &egui::Ui, glyph: &str, point: &Point) -> egui::text::LayoutJob {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let visuals = ui.visuals();
    let mut job = egui::text::LayoutJob::default();
    let mut add = |text: &str, color| job.append(text, 0.0, egui::TextFormat::simple(font.clone(), color));
    add(&format!("{glyph} {}  ", point.path), visuals.text_color());
    if let Some(kind) = nemesis_diff::schema::type_of_path(&point.path) {
        add(&kind, egui_widgets::markdown::class_color(visuals));
    }
    job
}

/// A template as Nemesis names it. The 1st person's has the same file name as the 3rd person's.
///
/// e.g. `meshes/actors/character/behaviors/0_master.bin` -> `0_master`,
/// `meshes/actors/character/_1stperson/behaviors/0_master.bin` -> `_1stperson/0_master`
fn template_name(target: &str) -> String {
    let file = target.rsplit('/').next().unwrap_or(target);
    let name = file.strip_suffix(".bin").unwrap_or(file);
    if target.contains("/_1stperson/") { format!("_1stperson/{name}") } else { name.to_owned() }
}

/// Each mod's value, the one the merge keeps first.
fn point_ui(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    scan: &ConflictScan,
    found: &crate::conflicts::Conflicts,
    point: &Point,
) -> Option<ResolveRequest> {
    let winner = scan.winner_of(found, point).map(|winner| winner.id.as_str());
    let mut values: Vec<_> = point.values.iter().map(|value| (&found.mods[value.owner as usize], value)).collect();
    values.sort_by_key(|(owner, _)| Some(owner.id.as_str()) != winner);

    let mut request = None;
    let height = ui.available_height().max(MIN_VALUES_HEIGHT);
    egui::ScrollArea::vertical().id_salt("conflict_values").max_height(height).auto_shrink([false, true]).show(
        ui,
        |ui| {
            for (owner, value) in values {
                ui.horizontal_top(|ui| {
                    ui.strong(&owner.name);
                    if Some(owner.id.as_str()) == winner {
                        ui.colored_label(egui::Color32::from_rgb(100, 200, 120), i18n.t(I18nKey::ConflictKept))
                            .on_hover_text(i18n.t(I18nKey::ConflictKeptHover));
                    }
                    let hover =
                        if owner.dir.is_some() { I18nKey::ConflictUseHover } else { I18nKey::ConflictFnisHover };
                    let usable = owner.dir.is_some();
                    let mut ask = |edit: bool| {
                        request = owner.dir.as_ref().map(|dir| ResolveRequest {
                            target: point.target.to_string(),
                            path: point.path.to_string(),
                            mod_dir: dir.clone(),
                            edit,
                        });
                    };
                    if ui
                        .add_enabled(usable, Button::new(i18n.t(I18nKey::ConflictUse)).small())
                        .on_hover_text(i18n.t(hover))
                        .on_disabled_hover_text(i18n.t(hover))
                        .clicked()
                    {
                        ask(false);
                    }
                    if ui
                        .add_enabled(usable, Button::new(i18n.t(I18nKey::ConflictEdit)).small())
                        .on_hover_text(i18n.t(I18nKey::ConflictEditHover))
                        .on_disabled_hover_text(i18n.t(hover))
                        .clicked()
                    {
                        ask(true);
                    }
                });
                ui.add(egui::Label::new(egui::RichText::new(&*value.preview).monospace()).wrap());
                ui.add_space(4.0);
            }
        },
    );
    request
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_template_is_shown_by_its_name() {
        assert_eq!(template_name("meshes/actors/character/behaviors/0_master.bin"), "0_master");
        assert_eq!(template_name("meshes/actors/character/_1stperson/behaviors/0_master.bin"), "_1stperson/0_master");
        assert_eq!(template_name("x"), "x");
    }

    #[test]
    fn conflicts_are_listed_under_their_template() {
        let point = |target: &str, path: &str| Point { target: target.into(), path: path.into(), values: Vec::new() };
        let (a, b, c) = (point("t1", "#1"), point("t1", "#2"), point("t2", "#1"));
        let rows = list_rows(&[&a, &b, &c]);
        assert_eq!(
            rows,
            [
                ListRow::Template("t1"),
                ListRow::Point(&a),
                ListRow::Point(&b),
                ListRow::Template("t2"),
                ListRow::Point(&c),
            ]
        );
    }
}
