use rayon::prelude::*;
use std::collections::HashMap;

use super::{AdsfPatch, PatchKind};

/// A unique identifier for deduplicating patches based on their
/// target, id, and an associated clip or index.
#[derive(Hash, Eq, PartialEq)]
enum PatchKey<'a> {
    /// Edit an animation by its target, id, and clip name.
    EditAnim {
        target: &'a str,
        id: &'a str,
        clip_name: &'a str,
    },
    /// Edit a motion by its target, id, and clip id.
    EditMotion {
        target: &'a str,
        id: &'a str,
        clip_id: &'a str,
    },
    /// Add an animation entry.
    AddAnim {
        target: &'a str,
        id: &'a str,
        index: usize,
    },
    /// Add a motion entry.
    AddMotion {
        target: &'a str,
        id: &'a str,
        index: usize,
    },
}

/// Deduplicates a list of `AdsfPatch` instances in parallel,
/// merging and prioritizing patches as needed.
///
/// ## Behavior
///
/// - Determines uniqueness based on the `target`, `id`, and an associated identifier
///   (`clip_name` for `EditAnim`, `clip_id` for `EditMotion`, or `index` for additions).
/// - When duplicate edits occur:
///   - The one with the higher `priority` replaces or merges into the existing one.
///   - For `EditAnim` patches:
///     - The higher priority patch merges its contents into the existing one,
///       updating its `name_clip` and `priority`.
///   - For `EditMotion` patches:
///     - The higher priority patch merges its contents into the existing one,
///       updating its `clip_id` and `priority`.
/// - `AddAnim` and `AddMotion` patches are treated uniquely based on their index,
///   making them distinct entries.
///
/// ## Returns
///
/// A `Vec<AdsfPatch<'a>>` containing only the relevant, deduplicated patches,
/// with priority-based merging applied.
pub fn dedup_patches_by_priority_parallel<'a>(patches: Vec<AdsfPatch<'a>>) -> Vec<AdsfPatch<'a>> {
    patches
        .into_par_iter()
        .enumerate()
        .fold(
            HashMap::new,
            |mut map: HashMap<PatchKey<'_>, AdsfPatch<'a>>, (idx, patch)| {
                let key = match &patch.patch {
                    PatchKind::EditAnim(edit) => PatchKey::EditAnim {
                        target: patch.target,
                        id: patch.id,
                        clip_name: edit.name_clip,
                    },
                    PatchKind::EditMotion(edit) => PatchKey::EditMotion {
                        target: patch.target,
                        id: patch.id,
                        clip_id: edit.clip_id,
                    },
                    PatchKind::AddAnim(_) => PatchKey::AddAnim {
                        target: patch.target,
                        id: patch.id,
                        index: idx,
                    },
                    PatchKind::AddMotion(_) => PatchKey::AddMotion {
                        target: patch.target,
                        id: patch.id,
                        index: idx,
                    },
                };
                match map.entry(key) {
                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        match (&mut entry.get_mut().patch, patch.patch) {
                            (PatchKind::EditAnim(existing_edit), PatchKind::EditAnim(new_edit))
                                if new_edit.priority > existing_edit.priority =>
                            {
                                existing_edit.patch.merge(new_edit.patch);
                                existing_edit.name_clip = new_edit.name_clip;
                                existing_edit.priority = new_edit.priority;
                            }
                            (
                                PatchKind::EditMotion(existing_edit),
                                PatchKind::EditMotion(new_edit),
                            ) if new_edit.priority > existing_edit.priority => {
                                existing_edit.patch.merge(new_edit.patch);
                                existing_edit.clip_id = new_edit.clip_id;
                                existing_edit.priority = new_edit.priority;
                            }
                            _ => {
                                // Do nothing if the new patch has lower or equal priority.
                            }
                        }
                    }
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        entry.insert(patch);
                    }
                }
                map
            },
        )
        .reduce(HashMap::new, |mut map1, map2| {
            for (key, patch2) in map2 {
                match map1.entry(key) {
                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        match (&mut entry.get_mut().patch, &patch2.patch) {
                            (PatchKind::EditAnim(existing_edit), PatchKind::EditAnim(new_edit))
                                if new_edit.priority > existing_edit.priority =>
                            {
                                entry.insert(patch2);
                            }
                            (
                                PatchKind::EditMotion(existing_edit),
                                PatchKind::EditMotion(new_edit),
                            ) if new_edit.priority > existing_edit.priority => {
                                entry.insert(patch2);
                            }
                            _ => {
                                // Do nothing for lower or equal priority.
                            }
                        }
                    }
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        entry.insert(patch2);
                    }
                }
            }
            map1
        })
        .into_values()
        .collect()
}
