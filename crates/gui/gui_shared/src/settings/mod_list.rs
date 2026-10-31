//! Per-execution-mode paths and mod list.
//!
//! [`ModeSettings`] is instantiated twice in [`AppSettings`]: once for VFS
//! mode and once for Manual mode.  The two instances are structurally
//! identical but carry different semantics (see field docs).

use serde::{Deserialize, Serialize};

use crate::mod_item::ModItem;

/// Paths and mod list for one execution mode (VFS or Manual).
///
/// # VFS instance (`AppSettings::vfs`)
/// - `skyrim_data_dir`: resolved via the MO2 virtual file system; on
///   Windows auto-detected from the Steam registry.
/// - `mod_list`: each entry's ID is the bare Nemesis mod ID (e.g. `aaaa`),
///   making the list portable across machines.
///
/// # Manual instance (`AppSettings::manual`)
/// - `skyrim_data_dir`: must point to the directory that directly contains
///   `meshes/`, `scripts/`, etc.
/// - `mod_list`: each entry's ID is the absolute path up to the Nemesis mod
///   ID directory, so entries may not be portable across machines with
///   different drive layouts.
///
/// # JSON keys
/// Serialized under `"vfs"` and `"manual"` in `settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModListSettings {
    /// Skyrim data directory for this mode.
    ///
    /// Must be set before a patch can run.  An empty string is treated as
    /// "not configured" and triggers an error notification.
    pub skyrim_data_dir: String,

    /// Directory where generated behavior files are written.
    ///
    /// The patch runner refuses to write here when this path equals
    /// `skyrim_data_dir` and `auto_remove_meshes` is enabled, to avoid
    /// accidental destruction of installed mods.
    pub output_dir: String,

    /// Ordered list of mods to include in the next patch run.
    ///
    /// Order determines merge priority when two mods affect the same
    /// behavior file.  Each entry carries an `enabled` flag; disabled
    /// entries are passed to the merger but marked inactive.
    pub mod_list: Vec<ModItem>,
}

impl Default for ModListSettings {
    fn default() -> Self {
        Self {
            skyrim_data_dir: String::new(),
            output_dir: "./d_merge_output".into(),
            mod_list: Vec::new(),
        }
    }
}
