//! Parsed (borrowed) patches per template: what `behavior_merge` and `fnis_patch_gen`
//! both produce, and `behavior_merge` applies.

mod key_index;
mod list_map;
mod patch_json;
mod patch_map;
mod priority_map;

use std::borrow::Cow;

use fast_hash::HashMap;

pub use self::{
    list_map::ListMap,
    patch_json::{PatchJson, ReadPatchJson},
    patch_map::{HkxPatchMaps, OnePatchMap, Overridden, SeqPatchMap},
    priority_map::{Prioritized, PriorityEntry, PriorityMap},
};
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
/// - value: the patches of that template, see [`HkxPatchMaps`]
///
/// Not concurrent: it has one writer at a time. `fnis_patch_gen` gives each worker a map of its own
/// (rayon's `fold`) and merges them after (`reduce`); `behavior_merge` collects one made per template.
/// A reader may still go through it in parallel (`par_iter`).
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
pub struct BehaviorPatchesMap<'a>(pub HashMap<TemplateKey<'static>, HkxPatchMaps<'a>>);

impl<'a> BehaviorPatchesMap<'a> {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// How many paths are patched, over every template.
    pub fn len(&self) -> usize {
        self.0.values().map(HkxPatchMaps::len).sum()
    }

    /// Takes in the patches of `other`: of a template both have, as [`HkxPatchMaps::merge`] does.
    pub fn merge(&mut self, other: Self) {
        for (key, other_maps) in other.0 {
            match self.0.entry(key) {
                fast_hash::hash_map::Entry::Vacant(vacant) => {
                    vacant.insert(other_maps);
                }
                fast_hash::hash_map::Entry::Occupied(mut occupied) => occupied.get_mut().merge(other_maps),
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

/// A map from a template key (e.g., a file name like `0_master.xml`)
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
pub struct BehaviorIndexMap<'a>(pub HashMap<TemplateKey<'a>, BehaviorIndexes>);
impl<'a> BehaviorIndexMap<'a> {
    /// Create `Self`
    #[inline]
    pub fn new() -> Self {
        Self(HashMap::default())
    }

    /// Inserts the indexes. Already inserted ones are kept, except that the one with
    /// `behavior_graph` index(more information) is preferred.
    ///
    /// Therefore, the result does not depend on the insertion order.
    #[inline]
    pub fn insert(&mut self, key: TemplateKey<'a>, indexes: BehaviorIndexes) {
        match self.0.entry(key) {
            fast_hash::hash_map::Entry::Occupied(mut occupied) => {
                if occupied.get().behavior_graph.is_none() && indexes.behavior_graph.is_some() {
                    occupied.insert(indexes);
                }
            }
            fast_hash::hash_map::Entry::Vacant(vacant) => {
                vacant.insert(indexes);
            }
        }
    }
}
