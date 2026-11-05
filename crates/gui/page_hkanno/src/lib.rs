//! hkanno editor page: edit the annotations embedded in Havok animation files.
//!
//! ```text
//! ┌ [attack.hkx ✖] [idle.hkx ✖]                                       [Open] ┐  tabs
//! ├ Output: [attack.modified.hkx                 ]  Format: [Amd64 ▼]         ┤  file bar
//! ├ [Save] [Revert] [Format] [⚙] Keymap: [Vim ▼]               [Hide Preview] ┤  toolbar
//! │ Annotation                              │ Preview (XML)                      │
//! │ # numOriginalFrames: 38                 │ <hkobject ...>                     │
//! │ trackName: Root                         │   <hkparam name="trackName">...    │
//! │ 0.100000 MCO_DodgeOpen                  │                                    │
//! ├ Problems ───────────────────────────────────────────────────────────────────┤
//! └ ✖ Ln 5, Col 1: Missing z value in animmotion.                    [Fix ...]  ┘
//! ```
//!
//! # Ownership
//! - persisted state: `settings.hkanno`, borrowed through [`HkannoPageCtx`]
//! - open files / editor buffers: [`HkannoPage`], owned by the app
//!
//! # Modules
//! - [`tab`]: one open file (load / save / revert / diagnostics)
//! - [`language`]: plugs `hkanno_lsp` into the generic `egui_code_editor::CodeEditor`
//! - [`preview`]: XML preview + cursor sync
//! - [`ui`]: drawing and key reading: tabs, editor, problems, toolbar, dialogs

mod language;
mod preview;
mod tab;
mod ui;

use egui_widgets::notify::Notification;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::{LspOptions, pages::hkanno::HkannoSettings},
};

use self::tab::HkannoTab;

/// What the page borrows from the app for one frame.
#[derive(Debug)]
pub struct HkannoPageCtx<'a> {
    /// Persisted page settings (`settings.hkanno`): options, preview toggle, keymap.
    pub settings: &'a mut HkannoSettings,
    pub lsp_settings: &'a mut LspOptions,
    pub i18n: &'a I18nMap,
    /// The app's notification bar.
    pub notify: &'a mut Notification,
    /// Frame (background) of the page's panels, from the app theme.
    pub panel_frame: egui::Frame,
}

/// Open files and transient dialog state. Keep one in the app.
#[derive(Default)]
pub struct HkannoPage {
    tabs: Vec<HkannoTab>,
    active: usize,
    /// "Language features" window.
    show_lsp_options: bool,
    /// "Revert?" confirmation modal.
    show_revert: bool,
}

impl std::fmt::Debug for HkannoPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HkannoPage")
            .field("tabs", &self.tabs.len())
            .field("active", &self.active)
            .finish_non_exhaustive()
    }
}

impl HkannoPage {
    /// Re-opens the tabs saved in `settings.json`. Files that can no longer be read are skipped
    /// (and reported in `notify`).
    pub fn restore(settings: &HkannoSettings, notify: &mut Notification) -> Self {
        let mut tabs = Vec::new();
        for saved in &settings.tabs {
            match HkannoTab::restore(saved) {
                Ok(tab) => tabs.push(tab),
                Err(err) => {
                    tracing::error!("Failed to restore hkanno tab: {err}");
                    notify.error(err);
                }
            }
        }
        let active = settings.active_tab.min(tabs.len().saturating_sub(1));
        Self { tabs, active, ..Self::default() }
    }

    /// Writes the open tabs into `settings` (call before saving `settings.json`).
    pub fn store(&self, settings: &mut HkannoSettings) {
        settings.tabs = self.tabs.iter().map(|tab| tab.settings.clone()).collect();
        settings.active_tab = self.active;
    }

    fn active_tab(&mut self) -> Option<&mut HkannoTab> {
        self.tabs.get_mut(self.active)
    }

    /// Opens `paths`; already open files just become active.
    fn open(&mut self, paths: impl IntoIterator<Item = String>, ctx: &mut HkannoPageCtx<'_>) {
        let failed = ctx.i18n.t(I18nKey::HkannoLoadFailed);
        for path in paths {
            if let Some(index) = self.tabs.iter().position(|t| t.settings.input_path == path) {
                self.active = index;
                continue;
            }
            match HkannoTab::open(&path) {
                Ok(tab) => {
                    self.tabs.push(tab);
                    self.active = self.tabs.len() - 1;
                }
                Err(err) => {
                    tracing::error!("{failed}: {err}");
                    ctx.notify.error(format!("{failed}: {err}"));
                }
            }
        }
    }

    fn close(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.tabs.remove(index);
            self.active = self.active.min(self.tabs.len().saturating_sub(1));
        }
    }

    /// Saves the active tab and reports the result.
    pub(crate) fn save_active(&mut self, ctx: &mut HkannoPageCtx<'_>) {
        let Some(tab) = self.active_tab() else { return };
        match tab.save() {
            Ok(()) => ctx.notify.success(format!("{}: {}", ctx.i18n.t(I18nKey::HkannoSaved), tab.settings.output_path)),
            Err(err) => {
                let msg = format!("{}: {err}", ctx.i18n.t(I18nKey::HkannoSaveFailed));
                tracing::error!("{msg}");
                ctx.notify.error(msg);
            }
        }
    }

    /// Replaces the active tab's annotations with the `.txt` at `path`, and reports the result.
    pub(crate) fn import_txt(&mut self, path: &std::path::Path, ctx: &mut HkannoPageCtx<'_>) {
        let Some(tab) = self.active_tab() else { return };
        match tab.import_txt(path) {
            Ok(()) => ctx.notify.success(format!("{}: {}", ctx.i18n.t(I18nKey::HkannoImported), path.display())),
            Err(err) => {
                let msg = format!("{}: {err}", ctx.i18n.t(I18nKey::HkannoLoadFailed));
                tracing::error!("{msg}");
                ctx.notify.error(msg);
            }
        }
    }
}

/// Dropped paths split into `(annotation texts, files to open)`.
///
/// e.g., `[a.hkx, notes.TXT, b.xml]` -> `([notes.TXT], [a.hkx, b.xml])`
fn split_dropped(paths: Vec<std::path::PathBuf>) -> (Vec<std::path::PathBuf>, Vec<std::path::PathBuf>) {
    paths.into_iter().partition(|path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("txt")))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn dropped_texts_are_kept_apart() {
        let dropped = ["a.hkx", "notes.TXT", "b.xml"].map(PathBuf::from).to_vec();
        let (texts, files) = split_dropped(dropped);
        assert_eq!(texts, [PathBuf::from("notes.TXT")]);
        assert_eq!(files, [PathBuf::from("a.hkx"), PathBuf::from("b.xml")]);
    }
}
