//! Patch maps of this crate + the ones shared with `fnis_patch_gen`.

use std::path::PathBuf;

pub(crate) use behavior_merge_types::patches::*;
use indexmap::IndexMap;

use crate::behaviors::tasks::{adsf::types::OwnedAdsfPatchMap, asdsf::types::OwnedAsdsfPatchMap};

#[derive(Debug, Default)]
pub(crate) struct OwnedPatches {
    /// Name of the template that needs to be read.
    ///
    /// - format: template_name, is_1st_person
    /// - e.g. (`0_master`, false)
    pub owned_patches: OwnedPatchMap,

    /// - key: template name (e.g., `"0_master"`, `"defaultmale"`)
    /// - value: `Map<jsonPath, { patch, priority }>`
    pub adsf_patches: OwnedAdsfPatchMap,

    /// HashMap showing which index (e.g. `#0000`) of each template (e.g. `0_master.xml`)
    /// contains `hkbBehaviorGraphStringData
    ///
    /// This information exists because it is needed to replace variables
    /// such as the Nemesis variable `$variableID[]$`, `$eventID[]$`.
    pub asdsf_patches: OwnedAsdsfPatchMap,
    pub errors: Vec<crate::errors::Error>,
}

impl OwnedPatches {
    /// Merges `other` into `self`. (For rayon `reduce`)
    pub(crate) fn merge(mut self, other: Self) -> Self {
        self.owned_patches.extend(other.owned_patches);
        self.adsf_patches.0.extend(other.adsf_patches.0);
        self.asdsf_patches.0.extend(other.asdsf_patches.0);
        self.errors.extend(other.errors);
        self
    }
}

/// - key: full path
/// - value: nemesis xml
pub(crate) type OwnedPatchMap = IndexMap<PathBuf, (String, usize), fast_hash::RandomState>;
