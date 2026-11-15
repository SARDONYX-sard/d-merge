//! Patch Editor page settings.
//!
//! Only what is needed to find the work in progress again: the work itself is kept in
//! `./.d_merge/caches/patch_editor/nemesis/<mod_code>/`, not here.

use std::collections::BTreeMap;

/// Persisted state of the Patch Editor page.
///
/// # JSON key
/// Serialized under the `"patch_editor"` key in `settings.json`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PatchEditorSettings {
    /// The mod's directory (`Nemesis_Engine/mod/<code>` is exported into it).
    pub output: String,
    /// The root of the mod the work was read from. Empty: it was made in the editor.
    pub mod_dir: String,
    /// Nemesis mod code of the work in progress (e.g. `mymod`).
    pub mod_code: String,
    /// Show the changes inside the editor: new lines tinted, old lines between them.
    pub inline_diff: bool,
    /// The patch preview scrolls to and highlights the editor's cursor line.
    pub follow_preview: bool,
    /// Show preview or not
    pub show_preview: bool,
    /// The object list only shows what changed.
    pub changed_only: bool,
    /// The object list is a tree (else a flat list).
    pub tree_view: bool,
    /// The tree nodes the user opened or closed: template path -> object id -> open.
    ///
    /// e.g. `meshes/actors/character/behaviors/0_master.bin` -> `#0091` -> `true`.
    /// Nodes not listed have their default state.
    pub tree_open: BTreeMap<String, BTreeMap<String, bool>>,
    /// The works opened lately, the last one first, like VS Code's "Open Recent".
    pub recent: Vec<RecentWork>,
    /// Where "Export as mod" of the user patch wrote last.
    pub user_patch_export: UserPatchExport,
}

/// Where the user patch is written as a mod (and read back from), and how.
///
/// e.g. `./output` -> `./output/Nemesis_Engine/mod/upatch/...`
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct UserPatchExport {
    /// The mod's root, e.g. `D:/MO2/mods/My Patch`.
    pub output: String,
    /// As Nemesis: what is done with the templates Nemesis does not know.
    pub outside_nemesis: OutsideNemesis,
}

/// What "Export as mod" does, as Nemesis, with the templates Nemesis does not know (e.g. a creature's
/// behavior that only d-merge can patch): Nemesis itself can't use their patches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OutsideNemesis {
    /// To `Nemesis_EngineExt`, which only d-merge reads.
    #[default]
    WriteExt,
    /// Left out of the mod.
    Skip,
    /// Nothing is written.
    Cancel,
}

impl OutsideNemesis {
    /// In the order of the setting's list.
    pub const ALL: [Self; 3] = [Self::WriteExt, Self::Skip, Self::Cancel];

    /// What the setting's list shows.
    pub const fn i18n_key(self) -> crate::i18n::I18nKey {
        use crate::i18n::I18nKey;
        match self {
            Self::WriteExt => I18nKey::OutsideNemesisWriteExt,
            Self::Skip => I18nKey::OutsideNemesisSkip,
            Self::Cancel => I18nKey::OutsideNemesisCancel,
        }
    }
}

/// A work of the Patch Editor. Its patches and what it had open are in its cache: these find it.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RecentWork {
    pub output: String,
    pub mod_code: String,
}

impl PatchEditorSettings {
    const fn empty() -> Self {
        Self {
            output: String::new(),
            mod_dir: String::new(),
            mod_code: String::new(),
            inline_diff: false,
            follow_preview: false,
            show_preview: false,
            changed_only: false,
            tree_view: false,
            tree_open: BTreeMap::new(),
            recent: Vec::new(),
            user_patch_export: UserPatchExport { output: String::new(), outside_nemesis: OutsideNemesis::WriteExt },
        }
    }
}

impl Default for PatchEditorSettings {
    #[inline]
    fn default() -> Self {
        Self { inline_diff: true, follow_preview: true, show_preview: true, ..Self::empty() }
    }
}
