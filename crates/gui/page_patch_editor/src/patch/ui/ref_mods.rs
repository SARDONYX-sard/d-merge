//! The mods a work depends on, in the bar of the Patch Editor and of the user patch: each with the
//! versions of it that the work works with, and what is wrong with them.
//!
//! A user patch refers to a mod for each side of each conflict, so they are folded to one line,
//! and listed, open, in a box of their own height:
//!
//! ```text
//! ▸ References: 7  Flinching (flinch), Precision (colis), …  ⚠ 1   [+]   <- folded: cut to fit
//! ▾ References: 7                                                 [+]
//!   │ Flinching (flinch)  [>=1.2.0] 1.1.0: Not of the versions…  ✖ │  <- open: one per line,
//!   │ Precision (colis)   [       ]                              ✖ │     scrolled past its height
//! ```
//!
//! What it tells is only told: patching does not stop. See `patch_editor_core::workspace::deps`.

use std::path::Path;

use egui_widgets::{
    HoverMarkdown as _,
    shadcn::{Button, Icon, TextEdit},
};
use gui_shared::i18n::{I18nKey, I18nMap};

use super::top_bar::BarAsk;
use crate::patch::workspace::{VersionCheck, Workspace};

/// The width of the field of the versions.
const REQ_WIDTH: f32 = 70.0;
/// The height the open list takes at most: past it, it scrolls.
const LIST_HEIGHT: f32 = 160.0;

/// The referenced mods (`Workspace::ref_mods`), folded or open (see the module docs), and `[+]` to
/// add one. Returns what was asked.
pub(in crate::patch) fn ref_mods_row(ui: &mut egui::Ui, i18n: &I18nMap, workspace: &mut Workspace) -> Vec<BarAsk> {
    let mut asks = Vec::new();
    let id = ui.make_persistent_id("ref_mods_open");
    let mut open = ui.data_mut(|data| *data.get_persisted_mut_or_default::<bool>(id));

    ui.horizontal(|ui| {
        let count = workspace.ref_mods.len();
        let chevron = if open { Icon::ChevronDown } else { Icon::ChevronRight };
        if count > 0 && ui.add(Button::new("").icon(chevron).small()).clicked() {
            open = !open;
            ui.data_mut(|data| data.insert_persisted(id, open));
        }
        ui.label(format!("{} {count}", i18n.t(I18nKey::RefModsLabel))).on_hover_markdown(i18n.t(I18nKey::RefModsHover));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let add = ui.add(Button::new("").icon(Icon::Plus).small()).on_hover_markdown(i18n.t(I18nKey::RefModsHover));
            if add.clicked()
                && let Some(dir) = gui_shared::rfd::FileDialog::new().pick_folder()
            {
                asks.push(BarAsk::AddRef(dir));
            }
            if !open {
                folded_summary(ui, i18n, workspace);
            }
        });
    });

    if open && !workspace.ref_mods.is_empty() {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::ScrollArea::vertical().max_height(LIST_HEIGHT).auto_shrink([false, true]).show(ui, |ui| {
                for at in 0..workspace.ref_mods.len() {
                    ui.horizontal(|ui| asks.extend(ref_mod(ui, i18n, workspace, at)));
                }
            });
        });
    }
    asks
}

/// `Flinching (flinch), Precision (colis), …  ⚠ 1`, right to left: how many have a problem, then the
/// names cut to the room left (all of them on hover).
fn folded_summary(ui: &mut egui::Ui, i18n: &I18nMap, workspace: &Workspace) {
    let problems: Vec<String> = (0..workspace.ref_mods.len())
        .filter_map(|at| {
            let (check, version) = workspace.ref_check(at)?;
            let problem = version_problem(i18n, check, version)?;
            Some(format!("{}: {problem}", ref_mod_name(&workspace.ref_mods[at])))
        })
        .collect();
    if !problems.is_empty() {
        let mark = ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {}", problems.len()));
        mark.on_hover_text(problems.join("\n"));
    }

    let names: Vec<String> = workspace.ref_mods.iter().map(|dir| ref_mod_name(dir)).collect();
    let names = names.join(", ");
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.add(egui::Label::new(egui::RichText::new(&names).weak()).truncate()).on_hover_text(&names);
    });
}

/// `Flinching (flinch)  [>=1.2.0] 1.1.0: Not of the versions...  ✖`: the referenced mod `at`.
fn ref_mod(ui: &mut egui::Ui, i18n: &I18nMap, workspace: &mut Workspace, at: usize) -> Option<BarAsk> {
    let dir = &workspace.ref_mods[at];
    ui.label(ref_mod_name(dir)).on_hover_text(dir);

    let edited = workspace.ref_req_mut(at).is_some_and(|req| {
        let hint = i18n.t(I18nKey::RefModVersionHint);
        let field = ui.add(TextEdit::singleline(req).hint_text(hint).desired_width(REQ_WIDTH));
        field.on_hover_markdown(i18n.t(I18nKey::RefModVersionHover)).changed()
    });
    if let Some(problem) = workspace.ref_check(at).and_then(|(check, version)| version_problem(i18n, check, version)) {
        ui.colored_label(ui.visuals().warn_fg_color, problem);
    }
    let removed = ui.add(Button::new("").icon(Icon::X).small()).on_hover_text(&workspace.ref_mods[at]).clicked();

    if removed { Some(BarAsk::RemoveRef(at)) } else { edited.then_some(BarAsk::Edited) }
}

/// What is wrong with the version `version` of a referenced mod, or `None`.
///
/// e.g. `Unmet`, `1.1.0` -> `1.1.0: Not of the versions this mod works with`
fn version_problem(i18n: &I18nMap, check: VersionCheck, version: &str) -> Option<String> {
    let text = |key| i18n.t(key);
    Some(match check {
        VersionCheck::Met => return None,
        VersionCheck::Unmet => format!("{version}: {}", text(I18nKey::RefModVersionUnmet)),
        VersionCheck::BadVersion => format!("{version}: {}", text(I18nKey::RefModVersionInvalid)),
        VersionCheck::NoVersion => text(I18nKey::RefModVersionMissing).to_owned(),
        VersionCheck::BadReq => text(I18nKey::RefModVersionReqInvalid).to_owned(),
    })
}

/// What a referenced mod is shown as: the name of its directory in the mods, and its code. Only the
/// path is looked at: it is drawn every frame.
///
/// e.g. `D:/MO2/mods/Flinching/Nemesis_Engine/mod/flinch` -> `Flinching (flinch)`; a path not of that
/// shape -> its last name, e.g. `flinch`
fn ref_mod_name(dir: &str) -> String {
    fn name_of(path: &Path) -> Option<std::borrow::Cow<'_, str>> {
        path.file_name().map(|name| name.to_string_lossy())
    }

    let path = Path::new(dir);
    let code = name_of(path).unwrap_or_else(|| dir.into());
    let is_mod_dir = path.parent().and_then(name_of).is_some_and(|name| name.eq_ignore_ascii_case("mod"));
    match path.ancestors().nth(3).and_then(name_of) {
        Some(mod_name) if is_mod_dir => format!("{mod_name} ({code})"),
        _ => code.into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::ref_mod_name;

    /// The function docs' examples.
    #[test]
    fn a_referenced_mod_shows_its_mod_s_name_and_code() {
        assert_eq!(ref_mod_name("D:/MO2/mods/Flinching/Nemesis_Engine/mod/flinch"), "Flinching (flinch)");
        assert_eq!(ref_mod_name("flinch"), "flinch");
    }
}
