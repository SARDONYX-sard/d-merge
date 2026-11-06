//! Convert page: hkx <-> xml/json converter.
//!
//! ```text
//! Inputs                                     [Files & Directories] [Directory Tree]
//! [+ Add Files…] [+ Add Directories…]                                  [Clear]
//! ┌──────────────────────────────────────────────────────────────────────┐
//! │ 📄 C:/mod/a.hkx                                              ✔    ✖ │
//! │ 📁 C:/mod/meshes                                                  ✖ │
//! │        (empty: "Drop .hkx / .xml / .json files or directories here")    │
//! └──────────────────────────────────────────────────────────────────────┘
//! ──────────────────────────────────────────────────────────────────────────
//! Output format: [HKX SE (64-bit)] [HKX LE (32-bit)] [XML] [JSON]
//! Output:        [ Next to each input                    ] [Select] [📂]
//! [                       Convert (2) → XML                              ]
//! ```
//!
//! - Inputs are **added** (buttons, drag & drop), never replaced; `✖` removes one, `Clear` all.
//! - "Files & Directories" takes files and directories in one list (a directory converts everything inside).
//! - "Directory Tree" shows the files under root directories with checkboxes, and keeps the directory
//!   structure in the output.
//!
//! # Ownership
//! - persisted inputs: `settings.convert`, borrowed through [`ConvertPageCtx`]
//! - transient state (loaded tree, progress): [`ConvertPage`], owned by the app
//!
//! The page never touches the rest of the app; everything it needs comes in [`ConvertPageCtx`].

mod task;
mod tree;
mod ui;

use std::path::{Path, PathBuf};

use egui_widgets::notify::Notification;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::pages::convert::{ConvertSettings, SelectionType},
};

use self::{task::ConvertTask, tree::TreeView};

/// Extensions the converter accepts as input.
const INPUT_EXTENSIONS: [&str; 3] = ["hkx", "xml", "json"];

/// What the page borrows from the app for one frame.
#[derive(Debug)]
pub struct ConvertPageCtx<'a> {
    /// Persisted page settings (`settings.convert`).
    pub settings: &'a mut ConvertSettings,
    pub i18n: &'a I18nMap,
    /// The app's notification bar.
    pub notify: &'a mut Notification,
    /// Frame (background) of the page's panels, from the app theme.
    pub panel_frame: egui::Frame,
    /// Runtime the conversion runs on.
    pub runtime: &'a tokio::runtime::Runtime,
}

/// Transient Convert page state (see the crate docs). Keep one in the app.
#[derive(Default)]
pub struct ConvertPage {
    tree: TreeView,
    task: ConvertTask,
}

impl std::fmt::Debug for ConvertPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConvertPage").field("running", &self.task.is_running()).finish_non_exhaustive()
    }
}

impl ConvertPage {
    /// Call every frame (even when the page is hidden): reports a finished conversion.
    pub fn poll(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        match self.task.take_finished() {
            Some(Ok(())) => notify.success(i18n.t(I18nKey::ConvertDone)),
            Some(Err(err)) => {
                tracing::error!("{err}");
                notify.error(err);
            }
            None => {}
        }
    }

    /// Number of inputs of the current mode (checked files in the tree).
    fn input_count(&self, settings: &ConvertSettings) -> usize {
        match settings.selection_type {
            SelectionType::Tree => self.tree.checked_count(),
            SelectionType::Files | SelectionType::Dir => settings.selected_files.len(),
        }
    }

    /// `Clear`: forget the inputs of the current mode and the statuses (not the output).
    fn clear_inputs(&mut self, settings: &mut ConvertSettings) {
        match settings.selection_type {
            SelectionType::Files | SelectionType::Dir => settings.selected_files.clear(),
            SelectionType::Tree => {
                settings.tree_roots.clear();
                self.tree.clear();
            }
        }
        self.task.clear_statuses();
    }

    fn start_convert(&mut self, egui_ctx: &egui::Context, ctx: &mut ConvertPageCtx<'_>) {
        let settings = &*ctx.settings;
        let (inputs, roots) = match settings.selection_type {
            SelectionType::Files | SelectionType::Dir => (settings.selected_files.clone(), None),
            SelectionType::Tree => (self.tree.checked_files(), Some(settings.tree_roots.clone())),
        };

        if inputs.is_empty() {
            ctx.notify.warn(ctx.i18n.t(I18nKey::ConvertNoInput));
            return;
        }

        self.task.start(ctx.runtime, egui_ctx, inputs, settings.output.clone(), settings.format, roots);
    }
}

/// Appends `paths` that are not in `list` yet.
fn add_paths<'p>(list: &mut Vec<String>, paths: impl Iterator<Item = &'p Path>) {
    for path in paths {
        let path = path.display().to_string();
        if !list.contains(&path) {
            list.push(path);
        }
    }
}

fn is_input_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| INPUT_EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e)))
}

/// Settings from v2.9 had a separate "Directories" mode; its directories now live in the
/// "Files & Directories" list.
fn migrate_dir_mode(settings: &mut ConvertSettings) {
    if settings.selection_type == SelectionType::Dir {
        settings.selection_type = SelectionType::Files;
    }
    if !settings.selected_dirs.is_empty() {
        let dirs = std::mem::take(&mut settings.selected_dirs);
        add_paths(&mut settings.selected_files, dirs.iter().map(Path::new));
    }
}

/// Adds paths dropped onto the window to the inputs of the current mode.
///
/// In the directory tree, directories become roots (files have no tree). In the list, directories
/// and the files the converter can read are added. e.g., `[C:/mod, a.hkx, b.png]` -> `C:/mod`, `a.hkx`
fn add_dropped(settings: &mut ConvertSettings, dropped: &[PathBuf]) {
    let paths = dropped.iter().map(PathBuf::as_path);
    match settings.selection_type {
        SelectionType::Tree => add_paths(&mut settings.tree_roots, paths.filter(|p| p.is_dir())),
        SelectionType::Files | SelectionType::Dir => {
            add_paths(&mut settings.selected_files, paths.filter(|p| p.is_dir() || is_input_file(p)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_new_paths_only() {
        let mut list = vec!["a".to_owned()];
        add_paths(&mut list, ["a", "b", "b"].iter().map(Path::new));
        assert_eq!(list, ["a", "b"]);
    }

    #[test]
    fn knows_input_files() {
        assert!(is_input_file(Path::new("a.HKX")));
        assert!(is_input_file(Path::new("a.json")));
        assert!(!is_input_file(Path::new("a.png")));
        assert!(!is_input_file(Path::new("no_extension")));
    }

    #[test]
    fn the_old_directory_mode_moves_to_the_list() {
        let mut settings = ConvertSettings {
            selection_type: SelectionType::Dir,
            selected_dirs: vec!["C:/mod".to_owned()],
            ..ConvertSettings::default()
        };
        migrate_dir_mode(&mut settings);
        assert!(settings.selection_type == SelectionType::Files);
        assert!(settings.selected_dirs.is_empty());
        assert_eq!(settings.selected_files, ["C:/mod"]);
    }

    #[test]
    fn dropped_files_are_filtered() {
        let mut settings = ConvertSettings { selection_type: SelectionType::Files, ..ConvertSettings::default() };
        add_dropped(&mut settings, &[PathBuf::from("a.hkx"), PathBuf::from("b.png")]);
        assert_eq!(settings.selected_files, ["a.hkx"]);
    }
}
