//! Parsed (borrowed) patches per template: what `behavior_merge` and `fnis_patch_gen`
//! both produce, and `behavior_merge` applies.

mod patch_map;

use std::borrow::Cow;

use dashmap::DashMap;

pub use self::patch_map::{HkxPatchMaps, OnePatchMap, SeqPatchMap};
use crate::template_key::TemplateKey;

/// Collection of patches with metadata
#[derive(Debug)]
pub struct PatchCollection<'a> {
    /// Actual template patch map
    /// - key: template name (e.g., `"0_master"`, `"defaultmale"`)
    /// - value: `Map<jsonPath, { patch, priority }>`
    pub borrowed_patches: BehaviorPatchesMap<'a>,
    /// Map showing which index of each template contains `hkbBehaviorGraphStringData` & `hkbBehaviorGraphData`.
    /// Used to replace Nemesis variables such as `$variableID[]$` or `$eventID[]$`.
    pub behavior_index_map: BehaviorIndexMap<'a>,
}
/// A patch containing references to parsed strings.
///
/// - key: template name (e.g., `"meshes/actors/character/behavior/0_master.bin"`)
/// - value: `DashMap<jsonPath, { patch, priority }>`
///
/// # Lifetime
/// The duration during which the Nemesis patch remains active from the path.
///
/// # Intended image
/// ```json
/// "meshes/.../0_master.bin": {
///     ["#0001", "hkbProjectData", "variable"]: OneField { op, patch, priority },
///     ["#0001", "hkbProjectData", "variableNames"]: Seq {
///         [{ op, patch, priority }, { op, patch, priority }]
///     }
/// },
/// "meshes/.../_1stperson/0_master.bin": {
///     ["#0001", "hkbProjectData", "variable"]: { op, patch, priority }
/// }
/// ```
#[derive(Debug, Default, Clone)]
pub struct BehaviorPatchesMap<'a>(
    pub DashMap<TemplateKey<'static>, HkxPatchMaps<'a>, fast_hash::RandomState>,
);

impl<'a> BehaviorPatchesMap<'a> {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        use rayon::prelude::*;
        self.0
            .par_iter()
            .map(|pair| {
                let HkxPatchMaps { one, seq } = pair.value();
                one.len() + seq.0.len()
            })
            .sum()
    }

    pub fn merge(&self, other: Self) {
        for (key, other_maps) in other.0 {
            match self.0.entry(key) {
                dashmap::Entry::Vacant(v) => {
                    v.insert(other_maps);
                }
                dashmap::Entry::Occupied(mut occ) => {
                    occ.get_mut().merge(other_maps);
                }
            }
        }
    }
}

/// Indexes(e.g. `#0000`) of the classes required to replace Nemesis variables.
#[derive(Debug, Clone)]
pub struct BehaviorIndexes {
    /// Index of `hkbBehaviorGraphStringData`. Required to replace event/variable names.
    pub string_data: Cow<'static, str>,
    /// Index of `hkbBehaviorGraphData`. Used for the length check of events/variables.
    ///
    /// Known from the template table or FNIS, but not from the Nemesis patch xml.
    pub behavior_graph: Option<Cow<'static, str>>,
}

/// A concurrent map from a template key (e.g., a file name like `0_master.xml`)
/// to the indexes of `hkbBehaviorGraphStringData` & `hkbBehaviorGraphData`.
///
/// This mapping is necessary for replacing Nemesis variables such as `$variableID[]$`, `$eventID[]$`,
/// where the variable needs to be resolved to the index of the event/variable names.
///
/// If a template is not in this map, the variables are not replaced.
///
/// - key: template_name
/// - value: [`BehaviorIndexes`]
#[derive(Debug, Default, Clone)]
pub struct BehaviorIndexMap<'a>(
    pub DashMap<TemplateKey<'a>, BehaviorIndexes, fast_hash::RandomState>,
);
impl<'a> BehaviorIndexMap<'a> {
    /// Create `Self`
    #[inline]
    pub fn new() -> Self {
        Self(DashMap::default())
    }

    /// Inserts the indexes. Already inserted ones are kept, except that the one with
    /// `behavior_graph` index(more information) is preferred.
    ///
    /// Therefore, the result does not depend on the insertion order.
    pub fn insert(&self, key: TemplateKey<'a>, indexes: BehaviorIndexes) {
        match self.0.entry(key) {
            dashmap::Entry::Occupied(mut occ) => {
                if occ.get().behavior_graph.is_none() && indexes.behavior_graph.is_some() {
                    occ.insert(indexes);
                }
            }
            dashmap::Entry::Vacant(vacant) => {
                vacant.insert(indexes);
            }
        }
    }
}
