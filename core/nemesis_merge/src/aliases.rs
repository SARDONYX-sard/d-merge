use dashmap::DashMap;
use indexmap::IndexMap;
use json_patch::{JsonPatch, JsonPath};
use simd_json::BorrowedValue;
use std::{collections::HashMap, path::PathBuf};

/// - key: template file stem(e.g. `0_master`)
/// - value: output_path(hkx file path), borrowed json (from template xml)
pub type BorrowedTemplateMap<'a> = DashMap<String, (PathBuf, BorrowedValue<'a>)>;
/// - key: full path
/// - value: nemesis xml
pub type OwnedPatchMap = IndexMap<PathBuf, String>;

// Parallel
// - "0_master": {
//      #0029", {
//          "aaaa", patch,
//          "bbbb", patch
//      }
//   }
// - "_1stperson/0_master": {
//      #0029", {
//          "aaaa", patch,
//          "bbbb", patch
//      }
//   }

/// - key: template name (e.g., "0_master", "defaultmale")
/// - value: patch_idx_map (target -> mod patches)
pub type TemplatePatchMap<'a> = DashMap<String, PatchIdxMap<'a>>;

/// - key: target identifier (e.g.: "#0029")
/// - value: mod_patch_map (mod_code -> patch content)
pub type PatchIdxMap<'a> = DashMap<String, ModPatchMap<'a>>;

/// - key: mod_code (e.g.: "aaaa", "bbbb")
/// - value: patches
pub type ModPatchMap<'a> = HashMap<String, SortedPatchMap<'a>>;

/// - key : path
/// - value: json patch
pub type SortedPatchMap<'a> = HashMap<JsonPath<'a>, JsonPatch<'a>>;

/// - key: template name
/// - value: json patches
pub type MergedPatchMap<'a> = DashMap<String, SortedPatchMap<'a>>;

/// - key: template name
/// - value: ptr name (e.g.: "#0029", "$aaaa$10", etc.)
#[derive(Debug, Default, Clone)]
pub struct PtrMap<'a>(pub DashMap<String, Option<&'a str>>);
impl PtrMap<'_> {
    pub fn new() -> Self {
        Self(DashMap::new())
    }
}
