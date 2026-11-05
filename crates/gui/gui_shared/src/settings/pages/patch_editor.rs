//! Patch Editor page settings.
//!
//! Only what is needed to find the work in progress again: the work itself is kept next to the
//! mod (`<output>/.d_merge/caches/patch_editor/nemesis/<mod_code>/`), not here.

/// Persisted state of the Patch Editor page.
///
/// # JSON key
/// Serialized under the `"patch_editor"` key in `settings.json`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PatchEditorSettings {
    /// The mod's directory (`Nemesis_Engine/mod/<code>` is exported into it).
    pub output: String,
    /// Nemesis mod code of the work in progress (e.g. `mymod`).
    pub mod_code: String,
    /// Show the changes inside the editor: new lines tinted, old lines between them.
    pub inline_diff: bool,
    /// The patch preview is to the right of the editor (else under it).
    pub preview_side: bool,
}

impl Default for PatchEditorSettings {
    fn default() -> Self {
        Self { output: String::new(), mod_code: String::new(), inline_diff: true, preview_side: true }
    }
}
