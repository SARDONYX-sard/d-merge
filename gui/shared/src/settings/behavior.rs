//! Behavior-generation options.
//!
//! [`BehaviorSettings`] groups every toggle that controls *what* the patch
//! runner does, as opposed to *where* it reads/writes files or how the UI
//! looks.  All fields are persisted to the settings JSON.

use std::borrow::Cow;

use skyrim_data_dir::Runtime;

/// Options that control patch-generation behavior.
///
/// # JSON key
/// Serialized under the `"behavior"` key in `settings.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BehaviorSettings {
    /// Execution mode: VFS (MO2) or Manual directory.
    ///
    /// Determines which [`super::ModeSettings`] block is active and how mod
    /// IDs are assigned (bare Nemesis ID vs. full path).
    pub mode: DataMode,

    /// Target Skyrim runtime for behavior generation.
    ///
    /// Controls which HKX format is emitted:
    /// - [`Runtime::Le`] — 32-bit legacy format
    /// - [`Runtime::Se`] / [`Runtime::Vr`] — 64-bit SE/AE/VR format
    pub target_runtime: Runtime,

    /// Enable all mods and run the patch automatically after every mod-list
    /// refresh.
    ///
    /// Intended for CI / automated workflows.  In interactive use this
    /// option can be surprising because it triggers a patch even after a
    /// directory-change that yields a different mod list.
    pub auto_run: bool,

    /// Reports behavior-generation progress to the GUI.
    ///
    /// When enabled, progress updates are sent through the
    /// `status_report` callback and displayed in the notification area.
    ///
    /// Disabling this removes nearly all progress-reporting overhead and
    /// only reports the final success or failure result.
    pub report_status: bool,

    /// Delete `<output_dir>/meshes` immediately before each patch run.
    ///
    /// Skipped with a warning when `output_dir` equals the Skyrim data
    /// directory, to prevent accidental destruction of installed mods.
    /// See [`nemesis_merge::cache_remover::is_dangerous_remove`].
    pub auto_remove_meshes: bool,

    /// Write intermediate patch JSON and merged XML files to
    /// `<output_dir>/.d_merge/patches/.debug`.
    ///
    /// Useful when diagnosing incorrect merge output.  Has no effect on the
    /// final HKX files.
    pub enable_debug_output: bool,

    /// Generate a `FNIS.esp` stub with correct version and author metadata.
    ///
    /// Required by some mods that detect FNIS via the ESP rather than the
    /// behavior files.
    pub generate_fnis_esp: bool,

    /// Directory containing the HKX template files to patch.
    ///
    /// Typically `./assets/templates`.  The actual merge target is the
    /// `meshes/` subdirectory within this directory.
    pub template_dir: Cow<'static, str>,
}

impl Default for BehaviorSettings {
    fn default() -> Self {
        Self {
            mode: DataMode::Vfs,
            target_runtime: Runtime::Se,
            auto_run: false,
            report_status: true,
            auto_remove_meshes: false,
            enable_debug_output: false,
            generate_fnis_esp: false,
            template_dir: "./assets/templates".into(),
        }
    }
}

/// Selects how mod directories are discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataMode {
    /// Virtual File System mode (MO2, Vortex, etc.).
    ///
    /// The mod list is derived from the VFS root; mod IDs are the bare
    /// Nemesis IDs (e.g. `aaaa`), which are stable across machines.
    Vfs,

    /// Manual directory mode.
    ///
    /// The user points directly at a mods folder.  Because sibling
    /// directories can share a Nemesis ID, the full path up to the ID
    /// segment is used as the key to avoid collisions.
    Manual,
}
