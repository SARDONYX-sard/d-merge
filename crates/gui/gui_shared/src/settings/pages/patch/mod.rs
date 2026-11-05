pub mod behavior;
pub mod mod_item;
pub mod mod_list;
pub mod support_pandora;

use self::{behavior::DataMode, mod_item::ModItem, mod_list::ModListSettings};

/// Patch page settings
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PatchSettings {
    /// Patch-generation behavior toggles.
    pub behavior: behavior::BehaviorSettings,

    /// Mod-list filter and sort state.
    pub view_state: mod_list::ViewState,

    pub pandora: crate::settings::PandoraSupportSettings,

    /// VFS-mode paths and mod list.
    pub vfs: mod_list::ModListSettings,
    /// Manual-mode paths and mod list.
    pub manual: mod_list::ModListSettings,
}

impl PatchSettings {
    /// Returns an immutable reference to the active [`ModeSettings`].
    ///
    /// Centralizes the `match self.behavior.mode { … }` branch so
    /// call-sites never repeat it.
    #[inline]
    pub const fn current_mod_list_settings(&self) -> &ModListSettings {
        match self.behavior.mode {
            DataMode::Vfs => &self.vfs,
            DataMode::Manual => &self.manual,
        }
    }

    /// Returns a mutable reference to the active [`ModeSettings`].
    #[inline]
    pub const fn current_mod_list_settings_mut(&mut self) -> &mut ModListSettings {
        match self.behavior.mode {
            DataMode::Vfs => &mut self.vfs,
            DataMode::Manual => &mut self.manual,
        }
    }

    /// Skyrim data directory for the active mode.
    #[inline]
    pub fn current_skyrim_data_dir(&self) -> &str {
        &self.current_mod_list_settings().skyrim_data_dir
    }

    /// Mutable Skyrim data directory for the active mode.
    #[inline]
    pub const fn current_skyrim_data_dir_mut(&mut self) -> &mut String {
        &mut self.current_mod_list_settings_mut().skyrim_data_dir
    }

    /// Output directory for the active mode.
    #[inline]
    pub fn current_output_dir(&self) -> &str {
        &self.current_mod_list_settings().output_dir
    }

    /// Mutable output directory for the active mode.
    #[inline]
    pub const fn current_output_dir_mut(&mut self) -> &mut String {
        &mut self.current_mod_list_settings_mut().output_dir
    }

    /// Mod list for the active mode (immutable).
    #[inline]
    pub fn mod_list(&self) -> &[ModItem] {
        &self.current_mod_list_settings().mod_list
    }

    /// Mod list for the active mode (mutable).
    #[inline]
    pub const fn mod_list_mut(&mut self) -> &mut Vec<ModItem> {
        &mut self.current_mod_list_settings_mut().mod_list
    }

    /// Constructs a GitHub issue URL pre-filled with system information.
    ///
    /// Included in the help window's bug-report section.
    ///
    /// `app_version`: e.g. `2.8.0`. From the app, so that this crate does not depend on the version
    /// and a release does not rebuild everything that uses it.
    pub fn create_issue_link(&self, app_version: &str) -> String {
        use std::{borrow::Cow, path::Path};

        let runtime = self.behavior.target_runtime;
        let skyrim_data_dir: Option<Cow<'_, Path>> = if self.vfs.skyrim_data_dir.trim().is_empty() {
            skyrim_data_dir::get_skyrim_data_dir(runtime).ok().map(Cow::Owned)
        } else {
            Some(Path::new(&self.vfs.skyrim_data_dir).into())
        };

        let skyrim_version =
            skyrim_data_dir.and_then(|skyrim_data_dir| runtime.get_version(skyrim_data_dir).map(|ver| ver.to_string()));

        gh_issue_link::new_gh_issue_link(app_version, runtime, skyrim_version.as_deref())
    }
}
