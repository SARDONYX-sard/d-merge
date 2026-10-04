//! Pandora Behaviour Engine settings import/export support.
//!
//! Only VFS-mode d-merge settings are supported.
//!
//! Pandora's `ActiveMods.json` contains Nemesis mod IDs only. NemesisExt and
//! FNIS entries are intentionally excluded.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use mod_info::ModType;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::settings::Settings;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PandoraSupportSettings {
    pub import_path: String,
    pub export_path: String,
    pub version: PandoraVersion,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PandoraVersion {
    /// v4.4.0-beta
    #[serde(rename = "v4.4.0-beta")]
    #[default]
    V4_4_0Beta,
}

impl PandoraVersion {
    pub const fn to_static_str(&self) -> &'static str {
        match self {
            Self::V4_4_0Beta => "v4.4.0-beta",
        }
    }
}

const SETTINGS_FILE_NAME: &str = "Settings.json";
const ACTIVE_MODS_FILE_NAME: &str = "ActiveMods.json";
const PANDORA_ENGINE_DIR_NAME: &str = "Pandora_Engine";

/// Pandora settings stored in `Settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PandoraSettings {
    pub app: PandoraAppSettings,
    pub games: PandoraGamesSettings,
}

/// Pandora application settings.
///
/// The theme has no corresponding d-merge setting and is therefore not
/// imported into [`Settings`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PandoraAppSettings {
    pub theme: i32,
}

/// Pandora game settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PandoraGamesSettings {
    #[serde(rename = "SkyrimSE")]
    pub skyrim_se: PandoraGameSettings,
}

/// Pandora settings for Skyrim Special Edition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PandoraGameSettings {
    #[serde(rename = "gameDataPath")]
    pub game_data_path: String,

    #[serde(rename = "outputPath")]
    pub output_path: String,
}

/// One entry from Pandora's `ActiveMods.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PandoraActiveMod {
    pub code: String,
    pub active: bool,
    pub priority: usize,
}

/// Pandora configuration loaded from its two configuration files.
#[derive(Debug, Clone)]
pub struct Pandora {
    pub settings: PandoraSettings,
    pub active_mods: Vec<PandoraActiveMod>,
}

impl Pandora {
    /// Imports Pandora configuration into d-merge VFS settings.
    ///
    /// # Errors
    ///
    /// Returns an error if Pandora files cannot be read or parsed, if the
    /// selected Pandora version is unsupported, or if duplicate Pandora mod
    /// codes are found.
    pub fn import_to<P>(
        root: P,
        settings: &mut Settings,
        version: PandoraVersion,
    ) -> Result<(), String>
    where
        P: AsRef<Path>,
    {
        let pandora = Self::load(root, version)?;
        pandora.apply_to(settings)
    }

    /// Exports the d-merge VFS settings as Pandora configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the Pandora configuration cannot be generated or
    /// written, or if the selected Pandora version is unsupported.
    pub fn export_from<P>(
        root: P,
        settings: &Settings,
        version: PandoraVersion,
    ) -> Result<(), String>
    where
        P: AsRef<Path>,
    {
        let pandora = Self::from_settings(settings, version)?;
        pandora.save(root, version)
    }

    fn load(root: impl AsRef<Path>, version: PandoraVersion) -> Result<Self, String> {
        match version {
            PandoraVersion::V4_4_0Beta => Self::load_v4_4_0_beta(root),
        }
    }

    fn load_v4_4_0_beta(root: impl AsRef<Path>) -> Result<Self, String> {
        let root = root.as_ref();

        let settings_path = root.join(SETTINGS_FILE_NAME);
        let active_mods_path = root.join(PANDORA_ENGINE_DIR_NAME).join(ACTIVE_MODS_FILE_NAME);

        let settings_text = std::fs::read_to_string(&settings_path)
            .map_err(|e| format!("Failed to read {}: {e}", settings_path.display()))?;

        let active_mods_text = std::fs::read_to_string(&active_mods_path)
            .map_err(|e| format!("Failed to read {}: {e}", active_mods_path.display()))?;

        let settings = sonic_rs::from_str(&settings_text)
            .map_err(|e| format!("Failed to parse {}: {e}", settings_path.display()))?;

        let active_mods = sonic_rs::from_str(&active_mods_text)
            .map_err(|e| format!("Failed to parse {}: {e}", active_mods_path.display()))?;

        Ok(Self { settings, active_mods })
    }

    fn save(&self, root: impl AsRef<Path>, version: PandoraVersion) -> Result<(), String> {
        match version {
            PandoraVersion::V4_4_0Beta => self.save_v4_4_0_beta(root),
        }
    }

    fn save_v4_4_0_beta(&self, root: impl AsRef<Path>) -> Result<(), String> {
        let root = root.as_ref();
        let engine_dir = root.join(PANDORA_ENGINE_DIR_NAME);

        std::fs::create_dir_all(&engine_dir)
            .map_err(|e| format!("Failed to create {}: {e}", engine_dir.display()))?;

        let settings_path = root.join(SETTINGS_FILE_NAME);
        let active_mods_path = engine_dir.join(ACTIVE_MODS_FILE_NAME);

        let settings_text = sonic_rs::to_string_pretty(&self.settings)
            .map_err(|e| format!("Failed to serialize {}: {e}", settings_path.display()))?;

        let active_mods_text = sonic_rs::to_string_pretty(&self.active_mods)
            .map_err(|e| format!("Failed to serialize {}: {e}", active_mods_path.display()))?;

        std::fs::write(&settings_path, settings_text)
            .map_err(|e| format!("Failed to write {}: {e}", settings_path.display()))?;

        std::fs::write(&active_mods_path, active_mods_text)
            .map_err(|e| format!("Failed to write {}: {e}", active_mods_path.display()))?;

        Ok(())
    }

    fn from_settings(settings: &Settings, version: PandoraVersion) -> Result<Self, String> {
        match version {
            PandoraVersion::V4_4_0Beta => Self::from_settings_v4_4_0_beta(settings),
        }
    }

    fn from_settings_v4_4_0_beta(settings: &Settings) -> Result<Self, String> {
        let vfs = &settings.vfs;

        let pandora_settings = PandoraSettings {
            app: PandoraAppSettings { theme: 0 }, // System: 0
            games: PandoraGamesSettings {
                skyrim_se: PandoraGameSettings {
                    game_data_path: vfs.skyrim_data_dir.clone(),
                    output_path: vfs.output_dir.clone(),
                },
            },
        };

        let active_mods = vfs
            .mod_list
            .par_iter()
            .enumerate()
            .filter_map(|(index, item)| {
                matches!(item.mod_type, ModType::Nemesis).then_some(PandoraActiveMod {
                    code: item.id.clone(),
                    active: item.enabled,
                    priority: index + 1,
                })
            })
            .collect();

        Ok(Self { settings: pandora_settings, active_mods })
    }

    fn apply_to(&self, settings: &mut Settings) -> Result<(), String> {
        settings.vfs.skyrim_data_dir = self.settings.games.skyrim_se.game_data_path.clone();
        settings.vfs.output_dir = self.settings.games.skyrim_se.output_path.clone();

        let mut seen = HashSet::with_capacity(self.active_mods.len());

        for active_mod in &self.active_mods {
            if !seen.insert(active_mod.code.as_str()) {
                return Err(format!("Duplicate Pandora mod code: {}", active_mod.code));
            }
        }

        let mut pandora_order = HashMap::with_capacity(self.active_mods.len());

        for (index, active_mod) in self.active_mods.iter().enumerate() {
            pandora_order.insert(active_mod.code.as_str(), (index, active_mod.active));
        }

        let mut nemesis = Vec::new();
        let mut other = Vec::new();

        for mut item in settings.vfs.mod_list.drain(..) {
            if item.mod_type == ModType::Nemesis {
                if let Some(&(order, active)) = pandora_order.get(item.id.as_str()) {
                    item.enabled = active;
                    item.priority = order;
                } else {
                    item.enabled = false;
                    item.priority = usize::MAX;
                }

                nemesis.push(item);
            } else {
                other.push(item);
            }
        }

        nemesis.sort_unstable_by(|a, b| {
            let a_order = pandora_order.get(a.id.as_str()).map_or(usize::MAX, |(order, _)| *order);
            let b_order = pandora_order.get(b.id.as_str()).map_or(usize::MAX, |(order, _)| *order);

            a_order.cmp(&b_order).then_with(|| a.id.cmp(&b.id))
        });

        settings.vfs.mod_list = nemesis;
        settings.vfs.mod_list.extend(other);

        settings.vfs.mod_list.iter_mut().enumerate().for_each(|(priority, item)| {
            item.priority = priority;
        });

        Ok(())
    }
}
