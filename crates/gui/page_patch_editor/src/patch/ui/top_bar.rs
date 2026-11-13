//! The bar above everything: what is being made, and exporting it. Its details fold away, as they
//! are set once.
//!
//! ```text
//! [File v] > mymod  D:/mods/out                [Open work data] | [Nemesis v] [Export]
//! ```
//! Unfolded: where it is exported to, the mod it was read from, the mods it depends on, and the mod's
//! `info.ini`. The list of behaviors is in [`super::behaviors`].
//!
//! Each part is a function that sees only what it shows and returns what was asked ([`BarAsk`]);
//! [`PatchTool::project_bar`] does it.

use std::path::{Path, PathBuf};

use egui_widgets::{
    PathActions, PathRow,
    notify::Notification,
    shadcn::{Button, ComboBox, Icon, TextEdit},
};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::pages::patch_editor::RecentWork,
};

use crate::patch::{
    PatchTool,
    new_work::NewWork,
    tool::rename::rename_problem,
    workspace::{ExportFormat, ModInfo, Workspace},
};

/// The width of the export format box.
const EXPORT_FORMAT_WIDTH: f32 = 112.0;

/// See [`PathRow`].
pub(super) const PATH_ACTIONS: PathActions =
    PathActions { open: gui_shared::fs::open_dir_of, pick: gui_shared::fs::pick_folder_from };

/// What the bar asked for.
#[derive(Debug)]
enum BarAsk {
    /// `File > New`: a new work, its output next to the open one's.
    NewWork,
    OpenMod,
    OpenRecent(RecentWork),
    CloseWork,
    Export,
    ExportFormat(ExportFormat),
    OpenCache,
    /// The output or the `info.ini` was edited.
    Edited,
    Rename(String),
    ReloadFromMod,
    RemoveRef(usize),
    AddRef(PathBuf),
}

impl PatchTool {
    /// The one line, and the details if unfolded.
    pub(in crate::patch) fn project_bar(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        let mut asks = Vec::new();
        ui.horizontal(|ui| {
            asks.extend(file_menu(ui, i18n, &self.recent));
            let chevron = if self.panels.project_open { Icon::ChevronDown } else { Icon::ChevronRight };
            let mut toggle = ui.add(Button::new("").icon(chevron).small()).clicked();

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let ready = !self.place.output.trim().is_empty() && !self.workspace.behaviors.is_empty();
                asks.extend(export_buttons(ui, i18n, ready, self.workspace.export_format));
                if cache_button(ui, i18n, &self.cache_dir()) {
                    asks.push(BarAsk::OpenCache);
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    toggle |= summary(ui, self.place.mod_code.trim(), self.place.output.trim());
                });
            });
            if toggle {
                self.panels.project_open = !self.panels.project_open;
            }
        });

        if self.panels.project_open {
            asks.extend(self.project_form(ui, i18n, notify));
        }
        for ask in asks {
            self.apply_bar(ask, notify, i18n);
        }
    }

    /// The directories, the mod code, and the mod's `info.ini`.
    fn project_form(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) -> Vec<BarAsk> {
        let mut asks = Vec::new();

        let output = PathRow::new(i18n.t(I18nKey::OutputLabel), i18n.t(I18nKey::SelectButton), PATH_ACTIONS)
            .help(i18n.t(I18nKey::NemesisOutputHover))
            .show(ui, &mut self.place.output);
        if let Some(err) = output.open_error {
            notify.error(err);
        }
        if output.changed {
            asks.push(BarAsk::Edited);
        }

        asks.extend(source_row(ui, i18n, self.place.mod_dir.trim(), notify));
        asks.extend(ref_mods_row(ui, i18n, &self.workspace.ref_mods));

        ui.horizontal_wrapped(|ui| {
            // It is in the ids of the new objects and the names of the folders: renamed, not typed over.
            let hover = i18n.t(I18nKey::NemesisModCodeHover);
            ui.label(i18n.t(I18nKey::NemesisModCodeLabel)).on_hover_text(hover);
            ui.strong(self.place.mod_code.trim()).on_hover_text(hover);
            if !self.is_user_patch {
                let old = self.place.loaded_for.as_deref().unwrap_or_default();
                let root = &self.place.cache_root;
                asks.extend(rename_form(ui, i18n, &mut self.panels.rename_to, (old, root)));
            }
            if info_fields(ui, i18n, &mut self.workspace.info) {
                asks.push(BarAsk::Edited);
            }
        });
        asks
    }

    /// Does what the bar asked for.
    fn apply_bar(&mut self, ask: BarAsk, notify: &mut Notification, i18n: &I18nMap) {
        match ask {
            BarAsk::NewWork => {
                // Usually next to the one that was open.
                let output = self.place.output.clone();
                self.close_work(notify, i18n);
                self.panels.new_work = Some(NewWork { code: String::new(), output, ..NewWork::default() });
            }
            BarAsk::OpenMod => self.pick_mod(notify, i18n),
            BarAsk::OpenRecent(work) => self.open_recent(&work, notify, i18n),
            BarAsk::CloseWork => self.close_work(notify, i18n),

            BarAsk::Export => self.export(notify, i18n),
            BarAsk::ExportFormat(format) => {
                self.workspace.export_format = format;
                self.pane.preview = None; // it says what the export would write
                self.mark_dirty();
            }
            BarAsk::OpenCache => self.open_cache(notify, i18n),

            BarAsk::Edited => self.mark_dirty(),
            BarAsk::Rename(new) => self.start_rename(&new, notify, i18n),
            BarAsk::ReloadFromMod => self.reload_from_mod(notify, i18n),

            BarAsk::RemoveRef(at) => {
                self.workspace.ref_mods.remove(at);
                self.workspace.reload_refs();
                self.refs_changed();
            }
            BarAsk::AddRef(dir) => {
                let code = self.place.mod_code.trim().to_owned();
                self.workspace.add_ref(&dir, &code);
                self.refs_changed();
            }
        }
    }

    /// The mods depended on changed: what they lend is read again.
    fn refs_changed(&mut self) {
        self.reset_names();
        self.pane.opened = None;
        self.mark_dirty();
    }

    /// `Open work data`: the directory of the work, saved first (a new work has none until then).
    pub(super) fn open_cache(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        self.save_now(notify, i18n);
        if let Err(err) = gui_shared::fs::open_dir_of(&self.cache_dir().to_string_lossy()) {
            notify.error(err);
        }
    }

    /// The directory of the open work, or of all the works if none is open.
    pub(super) fn cache_dir(&self) -> PathBuf {
        let root = &self.place.cache_root;
        let dir =
            self.place.loaded_for.as_deref().map_or_else(|| root.clone(), |code| Workspace::cache_dir(root, code));
        std::path::absolute(&dir).unwrap_or(dir)
    }
}

/// `File ▾`: `New`, `Open`, `Open Recent`, `Close`, like VS Code's.
fn file_menu(ui: &mut egui::Ui, i18n: &I18nMap, recent: &[RecentWork]) -> Option<BarAsk> {
    let mut asked = None;
    // With a frame and an arrow: it should be seen to be a menu.
    let button = egui::Button::new(format!("{} ▾", i18n.t(I18nKey::FileLabel)));
    egui::containers::menu::MenuButton::from_button(button).ui(ui, |ui| {
        let items = [
            (I18nKey::NemesisNewWork, I18nKey::NemesisNewWorkHover, BarAsk::NewWork),
            (I18nKey::NemesisOpenMod, I18nKey::NemesisOpenModHover, BarAsk::OpenMod),
        ];
        for (label, hover, ask) in items {
            if ui.button(i18n.t(label)).on_hover_text(i18n.t(hover)).clicked() {
                asked = Some(ask);
            }
        }

        ui.menu_button(i18n.t(I18nKey::NemesisOpenRecent), |ui| {
            if let Some(work) = recent_list(ui, recent, i18n) {
                asked = Some(BarAsk::OpenRecent(work));
            }
        });

        ui.separator();
        let close = ui.button(i18n.t(I18nKey::NemesisCloseWork)).on_hover_text(i18n.t(I18nKey::NemesisCloseWorkHover));
        if close.clicked() {
            asked = Some(BarAsk::CloseWork);
        }

        if asked.is_some() {
            ui.close();
        }
    });
    asked
}

/// `[Nemesis ▾] [Export]`, right to left. `ready`: there is an output and something to export.
fn export_buttons(ui: &mut egui::Ui, i18n: &I18nMap, ready: bool, format: ExportFormat) -> Option<BarAsk> {
    let export = ui
        .add_enabled(ready, Button::new(i18n.t(I18nKey::NemesisExport)).primary())
        .on_disabled_hover_text(i18n.t(I18nKey::NemesisExportDisabledHover))
        .clicked();

    // What `Export` writes: beside it, as it belongs to it and not to what opens something.
    let mut selected = ExportFormat::ALL.iter().position(|(known, _)| *known == format).unwrap_or(0);
    let format_box = ComboBox::from_id_salt("nemesis_export_format").width(EXPORT_FORMAT_WIDTH).show_index(
        ui,
        &mut selected,
        ExportFormat::ALL.len(),
        |index| ExportFormat::ALL[index].1,
    );
    let changed = format_box.on_hover_text(i18n.t(I18nKey::NemesisExportFormatHover)).changed();
    ui.separator();

    if export { Some(BarAsk::Export) } else { changed.then(|| BarAsk::ExportFormat(ExportFormat::ALL[selected].0)) }
}

/// `Open work data`, always in reach. Its hover says where that is (`dir`). Returns whether it was
/// clicked.
pub(super) fn cache_button(ui: &mut egui::Ui, i18n: &I18nMap, dir: &Path) -> bool {
    let button = Button::new(i18n.t(I18nKey::NemesisOpenCacheDir)).icon(Icon::FolderOpen);
    let response = ui.add(button).on_hover_ui(|ui| {
        ui.label(i18n.t(I18nKey::NemesisCacheDirHover));
        ui.monospace(dir.to_string_lossy());
    });
    response.clicked()
}

/// `mymod  D:/mods/out`. Returns whether it was clicked (to fold / unfold the details).
fn summary(ui: &mut egui::Ui, code: &str, output: &str) -> bool {
    let name = ui.add(egui::Label::new(egui::RichText::new(code).strong()).sense(egui::Sense::click()));
    let path = ui.add(egui::Label::new(egui::RichText::new(output).weak()).truncate().sense(egui::Sense::click()));
    name.clicked() || path.clicked()
}

/// `[Read from] D:/mods/Paraglider  [Reload]`, if the work was read from a mod (`mod_dir`).
fn source_row(ui: &mut egui::Ui, i18n: &I18nMap, mod_dir: &str, notify: &mut Notification) -> Option<BarAsk> {
    if mod_dir.is_empty() {
        return None;
    }

    let mut reload = false;
    let error = PathRow::new(i18n.t(I18nKey::NemesisModDirLabel), "", PATH_ACTIONS)
        .hover(i18n.t(I18nKey::NemesisSourceHover))
        .show_read_only(ui, mod_dir, |ui| {
            let button = Button::new(i18n.t(I18nKey::NemesisReload)).icon(Icon::RefreshCcw);
            reload = ui.add(button).on_hover_text(i18n.t(I18nKey::NemesisReloadHover)).clicked();
        });
    if let Some(err) = error {
        notify.error(err);
    }
    reload.then_some(BarAsk::ReloadFromMod)
}

/// `References: [amco ✖] [+]`: the mods this one depends on, see `Workspace::ref_mods`.
fn ref_mods_row(ui: &mut egui::Ui, i18n: &I18nMap, ref_mods: &[String]) -> Option<BarAsk> {
    let mut asked = None;
    ui.horizontal_wrapped(|ui| {
        ui.label(i18n.t(I18nKey::NemesisRefMods)).on_hover_text(i18n.t(I18nKey::NemesisRefModsHover));

        for (at, dir) in ref_mods.iter().enumerate() {
            let code = Path::new(dir).file_name().map_or_else(|| dir.as_str().into(), |name| name.to_string_lossy());
            if ui.add(Button::new(format!("{code} ✖")).small()).on_hover_text(dir).clicked() {
                asked = Some(BarAsk::RemoveRef(at));
            }
        }

        let add = ui.add(Button::new("").icon(Icon::Plus).small()).on_hover_text(i18n.t(I18nKey::NemesisRefModsHover));
        if add.clicked()
            && let Some(dir) = gui_shared::rfd::FileDialog::new().pick_folder()
        {
            asked = Some(BarAsk::AddRef(dir));
        }
    });
    asked
}

/// `[Rename…]`, then `[newc] [Rename] [Cancel] why not`. See [`crate::patch::tool::rename`].
/// `(old, cache_root)`: the code now, and where the works are (a new code must be free there).
fn rename_form(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    rename_to: &mut Option<String>,
    (old, cache_root): (&str, &Path),
) -> Option<BarAsk> {
    let Some(new) = rename_to else {
        let start = Button::new(i18n.t(I18nKey::NemesisRenameCode)).small();
        if ui.add(start).on_hover_text(i18n.t(I18nKey::NemesisRenameCodeHover)).clicked() {
            *rename_to = Some(String::new());
        }
        return None;
    };

    ui.add(TextEdit::singleline(new).hint_text(i18n.t(I18nKey::NemesisModCodeHint)).desired_width(90.0));
    let problem = rename_problem(old, new, |code| Workspace::exists(cache_root, code));
    let rename = ui.add_enabled(problem.is_none(), Button::new(i18n.t(I18nKey::NemesisRenameCode)).small().primary());
    let cancel = ui.add(Button::new(i18n.t(I18nKey::CancelButton)).small()).clicked();
    if let Some(problem) = problem.filter(|_| !new.is_empty()) {
        ui.colored_label(ui.visuals().warn_fg_color, i18n.t(problem));
    }

    if rename.on_hover_text(i18n.t(I18nKey::NemesisRenameCodeHover)).clicked() {
        let new = std::mem::take(new);
        *rename_to = None;
        return Some(BarAsk::Rename(new));
    }
    if cancel {
        *rename_to = None;
    }
    None
}

/// The mod's `info.ini`: name, author, site, version. Returns whether one was edited.
fn info_fields(ui: &mut egui::Ui, i18n: &I18nMap, info: &mut ModInfo) -> bool {
    let fields = [
        (I18nKey::NemesisNameLabel, I18nKey::NemesisNameHover, &mut info.name, 260.0),
        (I18nKey::AuthorLabel, I18nKey::NemesisAuthorHover, &mut info.author, 130.0),
        (I18nKey::NemesisSiteLabel, I18nKey::NemesisSiteHover, &mut info.site, 320.0),
        (I18nKey::NemesisVersionLabel, I18nKey::NemesisVersionHover, &mut info.version, 70.0),
    ];
    let mut changed = false;
    for (label, hover, value, width) in fields {
        ui.label(i18n.t(label)).on_hover_text(i18n.t(hover));
        let field = ui.add(TextEdit::singleline(value).desired_width(width));
        changed |= field.on_hover_text(i18n.t(hover)).changed();
    }
    changed
}

/// The works opened lately. Returns the one clicked.
pub(super) fn recent_list(ui: &mut egui::Ui, recent: &[RecentWork], i18n: &I18nMap) -> Option<RecentWork> {
    if recent.is_empty() {
        ui.weak(i18n.t(I18nKey::NemesisOpenRecentEmpty));
    }
    let mut picked = None;
    for work in recent {
        if ui.button(format!("{}  {}", work.mod_code, work.output)).clicked() {
            picked = Some(work.clone());
        }
    }
    picked
}
