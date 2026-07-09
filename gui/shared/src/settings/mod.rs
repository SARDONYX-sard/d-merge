//! Application settings: persistence, migration, and mode-dispatch helpers.

pub mod behavior;
mod compat_old;
pub mod log;
pub mod mod_list;
pub mod mod_list_ui;
pub mod ui;

use semver::Version;

pub use self::{
    behavior::{BehaviorSettings, DataMode},
    log::LogSettings,
    mod_list::ModListSettings,
    mod_list_ui::ModListUiSettings,
    ui::UiSettings,
};
use crate::mod_item::ModItem;

/// By placing settings in a fixed location within the Skyrim Data directory, you can handle switching between profiles in MO2.
pub const SETTINGS_PATH: &str = "./.d_merge/d_merge_settings.json";

/// Top-level settings written to and read from `settings.json`.
///
/// Each field group is a dedicated struct so that a call-site only borrows
/// the sub-struct it needs, rather than all of `AppSettings`.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    /// Schema version written by the last save.
    ///
    /// Since this option was not available until version 1.8.0, we will use this as the basis for migrating the old settings.
    /// Therefore, do not include `#[serde(default)]`
    pub app_version: Version,

    /// Patch-generation behavior toggles.
    pub behavior: BehaviorSettings,

    /// UI appearance and window geometry.
    #[serde(default)]
    pub ui: UiSettings,

    /// options for logging (directory, level).
    #[serde(default)]
    pub log: LogSettings,

    /// VFS-mode paths and mod list.
    #[serde(default)]
    pub vfs: ModListSettings,

    /// Manual-mode paths and mod list.
    #[serde(default)]
    pub manual: ModListSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            app_version: Self::default_version(),
            behavior: BehaviorSettings::default(),
            ui: UiSettings::default(),
            log: LogSettings::default(),
            vfs: ModListSettings::default(),
            manual: ModListSettings::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

impl Settings {
    /// Returns the default version for legacy settings files. `1.0.0`
    const fn default_version() -> Version {
        Version::new(1, 0, 0)
    }

    /// Loads settings from `settings.json` next to the executable.
    ///
    /// Returns [`Default`] when the file is absent.  Returns an error when
    /// the file exists but cannot be parsed, so the caller can surface a
    /// diagnostic rather than silently overwriting valid data.
    ///
    /// # Errors
    /// failed to read json
    pub fn load() -> Result<Self, String> {
        let path = std::path::Path::new(SETTINGS_PATH);

        if !path.exists() {
            return Ok(Self::default());
        }

        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {SETTINGS_PATH}: {e}"))?;

        let mut settings: Self = if let Ok(settings) = sonic_rs::from_str(&text) {
            settings
        } else {
            let old: compat_old::OldSettings = sonic_rs::from_str(&text)
                .map_err(|e| format!("Failed to parse {SETTINGS_PATH}: {e}"))?;
            old.into()
        };
        settings.app_version =
            Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or(Self::default_version());

        Ok(settings)
    }

    /// To json.
    ///
    /// # Errors
    ///
    /// Serialization can fail if `T`'s implementation of `Serialize` decides to fail, or if `T` contains a map with non-string keys.
    #[inline]
    pub fn to_string_pretty(&self) -> Result<String, sonic_rs::Error> {
        sonic_rs::to_string_pretty(self)
    }

    /// Writes settings to `settings.json`, stamping [`AppSettings::app_version`]
    /// with the current crate version before serializing.
    pub fn save(&self) {
        match sonic_rs::to_string_pretty(self) {
            Ok(text) => {
                if let Err(_e) = std::fs::write(SETTINGS_PATH, text) {
                    #[cfg(feature = "tracing")]
                    tracing::error!("Failed to save settings: {_e}");
                }
            }
            Err(_e) => {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to serialize settings: {_e}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Mode-dispatch helpers
// ---------------------------------------------------------------------------

impl Settings {
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
    pub fn create_issue_link(&self) -> String {
        use std::{borrow::Cow, path::Path};

        use gh_issue_link::{SkyrimRuntime, new_gh_issue_link, version::get_file_version};
        use skyrim_data_dir::{Runtime, get_skyrim_data_dir};

        let target_runtime = self.behavior.target_runtime;
        let skyrim_runtime = match target_runtime {
            Runtime::Le => SkyrimRuntime::Le,
            Runtime::Se => SkyrimRuntime::Se,
            Runtime::Vr => SkyrimRuntime::Vr,
        };

        let skyrim_data_dir: Option<Cow<'_, Path>> = if self.vfs.skyrim_data_dir.trim().is_empty() {
            get_skyrim_data_dir(target_runtime).ok().map(Cow::Owned)
        } else {
            Some(Path::new(&self.vfs.skyrim_data_dir).into())
        };

        let skyrim_version = skyrim_data_dir.and_then(|skyrim_data_dir| {
            let exe = match skyrim_runtime {
                SkyrimRuntime::Le => "TESV.exe",
                SkyrimRuntime::Se => "SkyrimSE.exe",
                SkyrimRuntime::Vr => "SkyrimVR.exe",
            };
            let exe_path = skyrim_data_dir.parent()?.join(exe);

            get_file_version(exe_path).map(|ver| ver.to_string()).ok()
        });
        new_gh_issue_link(env!("CARGO_PKG_VERSION"), skyrim_runtime, skyrim_version.as_deref())
    }
}
