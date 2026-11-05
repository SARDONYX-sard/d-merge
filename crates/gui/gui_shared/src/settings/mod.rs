//! Application settings: persistence, migration, and mode-dispatch helpers.

mod legacy;

pub mod log;
pub mod pages;
pub mod ui;

use semver::Version;
use snafu::ResultExt;

pub use self::{
    log::LogSettings,
    pages::patch::{
        behavior::{BehaviorSettings, DataMode},
        mod_list::{ModListSettings, ViewState},
        support_pandora::PandoraSupportSettings,
    },
    ui::{UiSettings, editor::LspOptions},
};
use crate::error::{Error, *};

/// The layout of `settings.json` written by this code. It only has to change with the layout:
/// [`Settings::load`] picks the migration by its major version.
const LAYOUT_VERSION: Version = Version::new(3, 0, 0);

/// Top-level settings written to and read from `settings.json`.
///
/// Each field group is a dedicated struct so that a call-site only borrows
/// the sub-struct it needs, rather than all of `AppSettings`.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    /// [`LAYOUT_VERSION`] of the last save. The key keeps its name for older `settings.json`.
    ///
    /// Since this option was not available until version 1.8.0.
    pub app_version: Version,

    /// UI appearance and window geometry.
    pub ui: UiSettings,

    /// options for logging (directory, level).
    pub log: LogSettings,

    /// each pages settings
    pub pages: PagesSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            app_version: LAYOUT_VERSION,
            ui: UiSettings::default(),
            log: LogSettings::default(),
            pages: PagesSettings::default(),
        }
    }
}

/// each page settings
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PagesSettings {
    /// Patch page state.
    pub patch: pages::patch::PatchSettings,
    /// Convert page state.
    pub convert: pages::convert::ConvertSettings,
    /// hkanno editor state.
    pub hkanno: pages::hkanno::HkannoSettings,
    /// Patch Editor page.
    pub patch_editor: pages::patch_editor::PatchEditorSettings,
}

impl Settings {
    /// By placing settings in a fixed location within the Skyrim Data directory, you can handle switching between profiles in MO2.
    pub const PATH: &str = "./.d_merge/d_merge_settings.json";

    /// Loads settings from `settings.json`.
    ///
    /// # Errors
    /// failed to deserialize self or read json
    pub fn load() -> Result<Self, Error> {
        #[derive(serde::Deserialize)]
        struct VersionProbe {
            // A missing field becomes `None`, which means "v1 era".
            app_version: Option<Version>,
        }

        let text = std::fs::read_to_string(Self::PATH).context(ReadFileSnafu { path: Self::PATH })?;
        let text = &text;

        let probe: VersionProbe = sonic_rs::from_str(text).context(SerializeJsonSnafu { path: Self::PATH })?;
        let mut settings: Self = match probe.app_version {
            None => {
                let v1: legacy::v1::Settings =
                    sonic_rs::from_str(text).context(SerializeJsonSnafu { path: Self::PATH })?;
                legacy::v2::Settings::from(v1).into()
            }
            // Compare `major`, not `< Version::new(3, 0, 0)`:
            // `3.0.0-beta.1 < 3.0.0` in semver, so a pre-release of v3 would be
            // mis-detected as v2.
            Some(v) if v.major < 3 => {
                let v2: legacy::v2::Settings =
                    sonic_rs::from_str(text).context(SerializeJsonSnafu { path: Self::PATH })?;
                v2.into()
            }
            Some(_) => sonic_rs::from_str(text).context(SerializeJsonSnafu { path: Self::PATH })?, // v3
        };

        settings.app_version = LAYOUT_VERSION;
        Ok(settings)
    }

    /// Writes settings to `settings.json`, stamping [`AppSettings::app_version`]
    /// with the current crate version before serializing.
    ///
    ///
    /// # Errors
    /// failed to serialize self or read json
    pub fn save(&self) -> Result<(), Error> {
        let json = sonic_rs::to_string_pretty(self).context(SerializeJsonSnafu { path: Self::PATH })?;
        std::fs::write(Self::PATH, json).context(WriteFileSnafu { path: Self::PATH })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A v2.8 `settings.json` (before the Convert/hkanno pages existed) must still load,
    /// and the new sections must fall back to their defaults.
    #[test]
    fn loads_settings_without_new_sections() {
        let json = r#"{ "app_version": "2.8.0", "behavior": {}, "ui": { "theme": "dark" } }"#;
        let settings: Settings = sonic_rs::from_str(json).expect("v2.8 settings must parse");

        assert_eq!(settings.ui.page, ui::Page::Patch);
        assert!(settings.pages.convert.selected_files.is_empty());
        assert!(settings.pages.hkanno.tabs.is_empty());
        assert!(settings.ui.lsp.completion);
    }

    /// Unknown keys (e.g., written by a newer version) must not break loading either.
    #[test]
    fn ignores_unknown_sections() {
        let json = r#"{ "app_version": "9.9.9", "behavior": {}, "future_section": { "x": 1 } }"#;
        assert!(sonic_rs::from_str::<Settings>(json).is_ok());
    }
}
