mod patch_map;

use dashmap::{DashMap, DashSet};
use indexmap::IndexMap;
use std::path::PathBuf;

pub use self::patch_map::{HkxPatches, OnePatchMap, SeqPatchMap};
use crate::behaviors::tasks::{
    adsf::types::OwnedAdsfPatchMap, asdsf::types::OwnedAsdsfPatchMap, templates::types::TemplateKey,
};

pub struct OwnedPatches {
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

/// - key: full path
/// - value: nemesis xml
pub type OwnedPatchMap = IndexMap<PathBuf, (String, usize)>;

pub struct BorrowedPatches<'a> {
    /// Name of the template that needs to be read.
    ///
    /// - format: template_name, is_1st_person
    /// - e.g. (`0_master`, false)
    pub template_names: DashSet<TemplateKey<'a>>,
    /// - key: template name (e.g., `"0_master"`, `"defaultmale"`)
    /// - value: `Map<jsonPath, { patch, priority }>`
    pub borrowed_patches: RawBorrowedPatches<'a>,
    /// HashMap showing which index (e.g. `#0000`) of each template (e.g. `0_master.xml`)
    /// contains `hkbBehaviorGraphStringData
    ///
    /// This information exists because it is needed to replace variables
    /// such as the Nemesis variable `$variableID[]$`, `$eventID[]$`.
    pub behavior_string_data_map: BehaviorStringDataMap<'a>,
}

/// - key: template name (e.g., `"0_master"`, `"defaultmale"`)
/// - value: `Map<jsonPath, { patch, priority }>`
///
/// # Intended
/// ```json
/// "0_master": {
///     ["#0001", "hkbProjectData", "variable"]: OneField { op, patch, priority },
///     ["#0001", "hkbProjectData", "variableNames"]: Seq {
///         [{ op, patch, priority }, { op, patch, priority }]
///     }
/// },
/// "_1stperson/0_master": {
///     ["#0001", "hkbProjectData", "variable"]: { op, patch, priority }
/// }
/// ```
#[derive(Debug, Default, Clone)]
pub(crate) struct RawBorrowedPatches<'a>(
    pub DashMap<TemplateKey<'a>, (OnePatchMap<'a>, SeqPatchMap<'a>)>,
);

impl RawBorrowedPatches<'_> {
    pub(crate) fn len(&self) -> usize {
        use rayon::prelude::*;
        self.0
            .par_iter()
            .map(|pair| {
                let (one, seq) = pair.value();
                one.0.len() + seq.0.len()
            })
            .sum()
    }
}

/// A concurrent map from a template key (e.g., a file name like `0_master.xml`)
/// to the identifier string (e.g., `#0000`) of the contained `hkbBehaviorGraphStringData`.
///
/// This mapping is necessary for replacing Nemesis variables such as `$variableID[]$`, `$eventID[]$`,
/// where the variable needs to be resolved to the corresponding behavior string data name.
///
/// - key: template_name
/// - value: index(e.g. `#0000`) of `hkbBehaviorGraphStringData`
#[derive(Debug, Default, Clone)]
pub struct BehaviorStringDataMap<'a>(pub DashMap<TemplateKey<'a>, &'a str>);
impl BehaviorStringDataMap<'_> {
    /// Create `Self`
    #[inline]
    pub fn new() -> Self {
        Self(DashMap::new())
    }
}
