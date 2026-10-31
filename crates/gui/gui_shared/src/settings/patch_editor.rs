//! Patch Editor page settings.
//!
//! Only what is needed to find the work in progress again: the work itself is kept next to the
//! mod (`<output>/.d_merge/caches/patch_editor/nemesis/<mod_code>/`), not here.

/// Persisted state of the Patch Editor page.
///
/// # JSON key
/// Serialized under the `"patch_editor"` key in `settings.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PatchEditorSettings {
    /// The mod's directory (`Nemesis_Engine/mod/<code>` is exported into it).
    pub output: String,
    /// Nemesis mod code of the work in progress (e.g. `mymod`).
    pub mod_code: String,
}
