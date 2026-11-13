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
    patch::{ResolveRequest, ResolveSide},
};
use stdx::text::contains_ignore_case;

use super::PatchView;
use crate::conflicts::{ConflictScan, Point};

/// How far a conflict is to the right of its template.
const INDENT: f32 = 12.0;
/// What wins: `+`, and the mod whose value the merge keeps.
const WINNER_COLOR: egui::Color32 = egui::Color32::from_rgb(100, 200, 120);

/// The values of a picked conflict get at least this.
const MIN_VALUES_HEIGHT: f32 = 140.0;
/// How much of the window the list takes while a conflict is picked, until the bar between them is
/// dragged: there are usually many conflicts, and only the few values of one.
const DEFAULT_LIST_SHARE: f32 = 0.7;
/// The list never gets less than this, so that a picked conflict's row can still be seen.
const MIN_LIST_HEIGHT: f32 = 60.0;
/// The bar between the list and the values.
const SPLITTER_HEIGHT: f32 = 8.0;

/// The most height the list can have beside the bar and the values.
fn max_list_height(available: f32) -> f32 {
    (available - SPLITTER_HEIGHT - MIN_VALUES_HEIGHT).max(MIN_LIST_HEIGHT)
}

/// The height of the list while a conflict is picked: `share` of `available`, as far as both parts
/// keep their minimum.
fn list_height(available: f32, share: f32) -> f32 {
    (available * share).clamp(MIN_LIST_HEIGHT, max_list_height(available))
}

/// The share of the list after the bar was dragged by `delta`, from where it is shown: a share beyond
/// what is shown would have to be dragged back for nothing.
fn dragged_share(available: f32, share: f32, delta: f32) -> f32 {
    (list_height(available, share) + delta).clamp(MIN_LIST_HEIGHT, max_list_height(available)) / available
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
        egui_widgets::show_modal(ctx, title, &mut open, |ui| {
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
    if scan.is_scanning() && scan.current.is_none() {
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
    if let Some(err) = &scan.hidden_error {
        ui.colored_label(ui.visuals().error_fg_color, err.as_str());
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
        ui.colored_label(WINNER_COLOR, format!("+ {}", mark.wins)).on_hover_text(i18n.t(I18nKey::ConflictWinsHover));
        ui.colored_label(ui.visuals().error_fg_color, format!("- {}", mark.loses))
            .on_hover_text(i18n.t(I18nKey::ConflictLosesHover));
        if scan.has_hidden()
            && ui
                .add(Button::new(i18n.t(I18nKey::ConflictUnhideAll)).small())
                .on_hover_text(i18n.t(I18nKey::ConflictUnhideAllHover))
                .clicked()
        {
            scan.show_all_hidden();
        }
        // The list stays while it follows what changed.
        if scan.is_scanning() {
            ui.add(egui::Spinner::new()).on_hover_text(i18n.t(I18nKey::ConflictScanning));
        }
    });

    let filter = scan.filter.trim();
    let shown: Vec<&Point> = points.iter().copied().filter(|point| contains_ignore_case(&point.path, filter)).collect();
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 4.0;
    let is_picked = shown.iter().any(|point| scan.is_selected(point));
    let rows = list_rows(&shown);
    let available = ui.available_height();
    let share = scan.list_share.unwrap_or(DEFAULT_LIST_SHARE);
    let height = if is_picked { list_height(available, share) } else { available };
    egui::ScrollArea::both().id_salt("conflict_rows").max_height(height).auto_shrink(false).show_rows(
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
                            let glyph = point_glyph(scan, &found, point, mod_id);
                            let selected = scan.is_selected(point);
                            let winner = scan.winner_of(&found, point).map(|winner| winner.name.as_str());
                            let text = row_text(ui, glyph, point, winner);
                            if ui.add(egui::Button::selectable(selected, text)).clicked() {
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
    let delta = egui_widgets::vertical_splitter(ui, SPLITTER_HEIGHT);
    if delta != 0.0 {
        scan.list_share = Some(dragged_share(available, share, delta));
    }
    point_ui(ui, i18n, scan, &found, point)
}

/// What the mod `mod_id` is at `point`:
/// - `x`: its operation is hidden
/// - `=`: no conflict any more, as another mod's is
/// - `+`: it wins, `-`: it loses
fn point_glyph(scan: &ConflictScan, found: &crate::conflicts::Conflicts, point: &Point, mod_id: &str) -> &'static str {
    let mine = found.mods.iter().find(|scanned| scanned.id == mod_id);
    if mine.is_some_and(|mine| scan.is_hidden(mine, point)) {
        "x"
    } else if !scan.is_contested(found, point) {
        "="
    } else if scan.winner_of(found, point).is_some_and(|winner| winner.id == mod_id) {
        "+"
    } else {
        "-"
    }
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

/// The color of what [`point_glyph`] gives.
fn glyph_color(visuals: &egui::Visuals, glyph: &str) -> egui::Color32 {
    match glyph {
        "+" => WINNER_COLOR,
        "-" => visuals.error_fg_color,
        _ => visuals.weak_text_color(),
    }
}

/// `+ #0106/hkbModifierList/enable  Bool  -> Mod name`: the glyph in the color of what it says, the
/// type in the color of class names, and the mod that wins in the end.
fn row_text(ui: &egui::Ui, glyph: &str, point: &Point, winner: Option<&str>) -> egui::text::LayoutJob {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let visuals = ui.visuals();
    let mut job = egui::text::LayoutJob::default();
    let mut add = |text: &str, color| job.append(text, 0.0, egui::TextFormat::simple(font.clone(), color));
    add(glyph, glyph_color(visuals, glyph));
    add(" ", visuals.text_color());
    add(&point.path, visuals.text_color());
    add("  ", visuals.text_color());
    if let Some(kind) = nemesis_diff::schema::type_of_path(&point.path) {
        add(&kind, egui_widgets::markdown::class_color(visuals));
    }
    if let Some(winner) = winner {
        add("  -> ", WINNER_COLOR);
        add(winner, WINNER_COLOR);
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

/// The mods with a value at `point`, to make the result from. The user patch's is what is being edited,
/// and FNIS has no patch file.
fn resolve_sides(found: &crate::conflicts::Conflicts, point: &Point) -> Vec<ResolveSide> {
    let owners = point.values.iter().map(|value| &found.mods[value.owner as usize]);
    owners
        .filter(|owner| owner.id != gui_shared::patch::USER_PATCH_ID)
        .filter_map(|owner| Some(ResolveSide { name: owner.name.clone(), mod_dir: owner.dir.clone()? }))
        .collect()
}

/// Takes the value of the mod at `mod_dir` for `point` into the user patch. `sides`: `Some` to resolve
/// it by hand, with each mod's value beside it (see [`resolve_sides`]); `None` to use it as it is.
fn resolve_request(point: &Point, mod_dir: &str, sides: Option<Vec<ResolveSide>>) -> ResolveRequest {
    ResolveRequest {
        target: point.target.to_string(),
        path: point.path.to_string(),
        mod_dir: mod_dir.to_owned(),
        edit: sides.is_some(),
        sides: sides.unwrap_or_default(),
        keep_all_events: false,
    }
}

/// `[Resolve by hand…]`, then each mod's value, the one the merge keeps first.
fn point_ui(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    scan: &mut ConflictScan,
    found: &crate::conflicts::Conflicts,
    point: &Point,
) -> Option<ResolveRequest> {
    let winner = scan.winner_of(found, point).map(|winner| winner.id.as_str());
    let mut values: Vec<_> = point.values.iter().map(|value| (&found.mods[value.owner as usize], value)).collect();
    values.sort_by_key(|(owner, _)| Some(owner.id.as_str()) != winner);

    // One way to resolve by hand, from the value kept now: not one per mod.
    let mut request = None;
    let start = values.iter().find_map(|(owner, _)| owner.dir.as_ref());
    let by_hand = Button::new(i18n.t(I18nKey::ConflictEdit)).small();
    ui.horizontal(|ui| {
        if ui.add_enabled(start.is_some(), by_hand).on_hover_text(i18n.t(I18nKey::ConflictEditHover)).clicked()
            && let Some(dir) = start
        {
            request = Some(resolve_request(point, dir, Some(resolve_sides(found, point))));
        }
        // `enterNotifyEvents` / `exitNotifyEvents`: the events of every mod can be kept.
        if gui_shared::patch::is_event_array_conflict(&point.path)
            && let Some(dir) = start
            && ui
                .add(Button::new(i18n.t(I18nKey::ConflictKeepAllEvents)).small())
                .on_hover_text(i18n.t(I18nKey::ConflictKeepAllEventsHover))
                .clicked()
        {
            let sides = resolve_sides(found, point);
            request = Some(ResolveRequest { keep_all_events: true, sides, ..resolve_request(point, dir, None) });
        }
    });

    let mut toggle = None;
    let height = ui.available_height().max(MIN_VALUES_HEIGHT);
    egui::ScrollArea::vertical().id_salt("conflict_values").max_height(height).auto_shrink(false).show(ui, |ui| {
        for (owner, value) in values {
            ui.horizontal_top(|ui| {
                let is_hidden = scan.is_hidden(owner, point);
                if is_hidden {
                    ui.label(egui::RichText::new(&owner.name).strikethrough().weak());
                    ui.weak(i18n.t(I18nKey::ConflictHidden));
                } else if Some(owner.id.as_str()) == winner {
                    ui.label(egui::RichText::new(&owner.name).strong().color(WINNER_COLOR));
                } else {
                    ui.strong(&owner.name);
                }
                if Some(owner.id.as_str()) == winner {
                    ui.colored_label(WINNER_COLOR, i18n.t(I18nKey::ConflictKept))
                        .on_hover_text(i18n.t(I18nKey::ConflictKeptHover));
                }
                let hover = if owner.dir.is_some() { I18nKey::ConflictUseHover } else { I18nKey::ConflictFnisHover };
                let usable = owner.dir.is_some();
                if ui
                    .add_enabled(usable, Button::new(i18n.t(I18nKey::ConflictUse)).small())
                    .on_hover_text(i18n.t(hover))
                    .on_disabled_hover_text(i18n.t(hover))
                    .clicked()
                {
                    request = owner.dir.as_ref().map(|dir| resolve_request(point, dir, None));
                }
                let (label, hide_hover) = if is_hidden {
                    (I18nKey::ConflictUnhide, I18nKey::ConflictUnhideHover)
                } else {
                    (I18nKey::ConflictHide, I18nKey::ConflictHideHover)
                };
                let hide_hover = if usable { hide_hover } else { I18nKey::ConflictFnisHover };
                if ui
                    .add_enabled(usable, Button::new(i18n.t(label)).small())
                    .on_hover_text(i18n.t(hide_hover))
                    .on_disabled_hover_text(i18n.t(hide_hover))
                    .clicked()
                {
                    toggle = owner.code();
                }
            });
            // What is not kept in the end steps back.
            let preview = egui::RichText::new(&*value.preview).monospace();
            let preview = if Some(owner.id.as_str()) == winner { preview } else { preview.weak() };
            ui.add(egui::Label::new(preview).wrap());
            ui.add_space(4.0);
        }
    });
    if let Some(code) = toggle {
        scan.toggle_hidden(code, point);
    }
    request
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: egui::Rect = egui::Rect { min: egui::Pos2::ZERO, max: egui::pos2(1200.0, 900.0) };

    /// Draws the real window for a few frames. Returns what is shown, and where its contents end.
    fn draw_the_window(count: usize, pick: bool, share: Option<f32>) -> (egui::Rect, f32) {
        use gui_shared::settings::pages::patch::mod_item::ModItem;

        use crate::conflicts::{Conflicts, ScannedMod, Value};
        let scanned =
            |id: &str, priority| ScannedMod { id: id.into(), name: id.into(), priority, dir: Some(id.into()) };
        let value = |owner, text: &str, hash| Value { owner, hash, preview: text.into() };
        let points: Vec<Point> = (0..count)
            .map(|at| Point {
                target: "meshes/x/0_master.bin".into(),
                path: format!("#{at:04}/hkbModifierList/enable").into(),
                values: vec![value(0, "true", 1), value(1, "false", 2)],
            })
            .collect();
        let mut scan = ConflictScan::default();
        scan.list_share = share;
        if pick {
            scan.select(&points[0]);
        }
        scan.current = Some(std::sync::Arc::new(Conflicts {
            mods: vec![scanned("a", 0), scanned("b", 1)],
            points,
            useful: Vec::new(),
        }));
        let item = |id: &str, priority| ModItem { id: id.into(), enabled: true, priority, ..Default::default() };
        scan.refresh_marks(&[item("a", 0), item("b", 1)], None, true);
        let i18n = I18nMap::new();

        let ctx = egui::Context::default();
        let (mut shown, mut bottom) = (egui::Rect::NOTHING, 0.0);
        for _ in 0..6 {
            let input = egui::RawInput { screen_rect: Some(SCREEN), ..Default::default() };
            let mut output = ctx.run_ui(input, |ui| {
                let ctx = ui.ctx().clone();
                let mut open = true;
                let drawn = egui_widgets::show_modal(&ctx, "Conflicts", &mut open, |ui| {
                    let _request = conflict_ui(ui, &i18n, &mut scan, "a");
                    (ui.max_rect(), ui.min_rect().bottom())
                });
                (shown, bottom) = drawn.unwrap();
            });
            output.textures_delta.clear(); // egui insists on them being taken
        }
        (shown, bottom)
    }

    /// However few the conflicts, and at any place of the bar, it fills the window and stays in it.
    #[test]
    fn the_contents_fill_the_window_and_stay_in_it() {
        for (count, pick, share) in [
            (40, false, None),
            (3, false, None),
            (40, true, None),
            (3, true, None),
            (40, true, Some(0.0)),
            (40, true, Some(1.0)),
        ] {
            let (shown, bottom) = draw_the_window(count, pick, share);
            let case = format!("{count} rows, picked: {pick}, share: {share:?}: {shown:?} to {bottom}");
            assert!(bottom <= shown.bottom() + 1.0, "{case}: left the window");
            assert!(bottom >= shown.bottom() - 1.0, "{case}: does not fill it");
        }
    }

    #[test]
    fn the_bar_moves_within_what_both_parts_need() {
        // Its share, with both minimums.
        assert!((list_height(800.0, 0.7) - 560.0).abs() < 0.5);
        assert!((list_height(800.0, 0.0) - MIN_LIST_HEIGHT).abs() < 0.5);
        assert!((list_height(800.0, 1.0) - (800.0 - SPLITTER_HEIGHT - MIN_VALUES_HEIGHT)).abs() < 0.5);
        // A window too small for both: the list keeps its own.
        assert!((list_height(100.0, 0.7) - MIN_LIST_HEIGHT).abs() < 0.5);

        // A drag moves it by as much.
        let share = dragged_share(800.0, 0.5, 80.0);
        assert!((list_height(800.0, share) - 480.0).abs() < 0.5);
        // From the end, the other way at once.
        let at_the_end = dragged_share(800.0, 0.5, 9999.0);
        let back = dragged_share(800.0, at_the_end, -10.0);
        assert!((list_height(800.0, back) - (800.0 - SPLITTER_HEIGHT - MIN_VALUES_HEIGHT - 10.0)).abs() < 0.5);
    }

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
