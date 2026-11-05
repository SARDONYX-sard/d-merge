//! The window of one mod's conflicts. What they are is in [`crate::conflicts`].
//!
//! ```text
//! ┌ Conflicts: Modern Combat ──────────────────────────────────┐
//! │ [filter........]  + 12  - 3                                │
//! │ + #0106/hkbModifierList/enable         0_master            │
//! │ - #0241/hkbClipGenerator/mode          0_master            │  <- a click shows it below
//! │ ────────────────────────────────────────────────────────── │
//! │ Other Mod  In effect                    [Use this]         │  <- the value the merge keeps
//! │ false                                                      │
//! │ Modern Combat                           [Use this]         │
//! │ true                                                       │
//! └────────────────────────────────────────────────────────────┘
//! ```
//! `Use this` sends that value to a user patch in the Patch Editor, which loads last and so wins.
//! The window can be resized both ways: the two lists share its height.

use egui_widgets::shadcn::{Button, TextEdit};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    patch::ResolveRequest,
};

use super::PatchView;
use crate::conflicts::{ConflictScan, Point};

impl PatchView<'_> {
    /// Draws the window if a mod is being looked at.
    pub(super) fn ui_conflict_window(&mut self, ctx: &egui::Context) {
        let i18n = self.ctx.i18n;
        let Some(mod_id) = self.page.conflicts.inspected.clone() else { return };
        let name = self.ctx.settings.mod_list().iter().find(|item| item.id == mod_id).map_or("", |item| &item.name);
        let title = format!("{}: {name}", i18n.t(I18nKey::ConflictTitle));

        let mut open = true;
        let mut request = None;
        let window = egui_widgets::window_near_pointer(ctx, title).open(&mut open);
        window.default_size([720.0, 560.0]).resizable(true).show(ctx, |ui| {
            request = conflict_ui(ui, i18n, &mut self.page.conflicts, &mod_id);
        });
        if !open {
            self.page.conflicts.inspected = None;
        }
        if let Some(mut request) = request {
            request.output = user_patch_output(self.ctx.settings.current_skyrim_data_dir());
            self.page.resolve = Some(request);
        }
    }
}

/// Where the user patch goes: where the mod list will find it.
///
/// e.g. `D:/Skyrim/Data` -> the same; `D:/mods/*` (manual mode) -> `D:/mods/d_merge_user_patch`
fn user_patch_output(data_dir: &str) -> String {
    match data_dir.trim().strip_suffix('*') {
        Some(mods_dir) => format!("{mods_dir}d_merge_user_patch"),
        None => data_dir.trim().to_owned(),
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
    // All the height for the list until a conflict is picked, then half of it.
    let is_picked = shown.iter().any(|point| scan.selected.as_deref() == Some(&*point.path));
    let list_height = if is_picked { ui.available_height() / 2.0 } else { ui.available_height() };
    egui::ScrollArea::both().id_salt("conflict_rows").max_height(list_height).auto_shrink(false).show_rows(
        ui,
        row_height,
        shown.len(),
        |ui, range| {
            for point in &shown[range] {
                let glyph =
                    if scan.winner_of(&found, point).is_some_and(|winner| winner.id == mod_id) { "+" } else { "-" };
                let label = format!("{glyph} {}   {}", point.path, short_target(&point.target));
                let selected = scan.selected.as_deref() == Some(&*point.path);
                if ui.add(egui::Button::selectable(selected, egui::RichText::new(label).monospace())).clicked() {
                    scan.selected = Some(point.path.to_string());
                }
            }
        },
    );
    ui.separator();
    let point = shown.iter().find(|point| scan.selected.as_deref() == Some(&*point.path))?;
    point_ui(ui, i18n, scan, &found, point)
}

/// `meshes/actors/character/behaviors/0_master.bin` -> `0_master`
fn short_target(target: &str) -> &str {
    let file = target.rsplit('/').next().unwrap_or(target);
    file.strip_suffix(".bin").unwrap_or(file)
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
    egui::ScrollArea::vertical().id_salt("conflict_values").auto_shrink(false).show(ui, |ui| {
        for (owner, value) in values {
            ui.horizontal_top(|ui| {
                ui.strong(&owner.name);
                if Some(owner.id.as_str()) == winner {
                    ui.colored_label(egui::Color32::from_rgb(100, 200, 120), i18n.t(I18nKey::ConflictKept))
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
                    request = owner.dir.as_ref().map(|dir| ResolveRequest {
                        target: point.target.to_string(),
                        path: point.path.to_string(),
                        mod_dir: dir.clone(),
                        output: String::new(),
                    });
                }
            });
            ui.add(egui::Label::new(egui::RichText::new(&*value.preview).monospace()).wrap());
            ui.add_space(4.0);
        }
    });
    request
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_user_patch_goes_where_the_mod_list_looks() {
        assert_eq!(user_patch_output("D:/Skyrim/Data"), "D:/Skyrim/Data");
        assert_eq!(user_patch_output(r"D:\mods\*"), r"D:\mods\d_merge_user_patch");
    }

    #[test]
    fn a_target_is_shown_by_its_file() {
        assert_eq!(short_target("meshes/actors/character/behaviors/0_master.bin"), "0_master");
        assert_eq!(short_target("x"), "x");
    }
}
