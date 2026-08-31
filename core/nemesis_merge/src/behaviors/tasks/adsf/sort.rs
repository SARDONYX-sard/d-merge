use rapidhash::fast::RapidHashMap as HashMap;
use rayon::prelude::*;

use super::{AdsfPatch, PatchKind};

/// A unique identifier for deduplicating patches based on their target,
/// mod ID, and the associated animation or motion entry.
#[derive(Hash, Eq, PartialEq)]
enum PatchKey<'a> {
    ProjectNamesHeader { target: &'a str, id: &'a str },
    AnimDataHeader { target: &'a str, id: &'a str },
    EditAnim { target: &'a str, id: &'a str, name_clip: &'a str },
    SpecifiedAnim { target: &'a str, id: &'a str, name_clip: &'a str },
    EditMotion { target: &'a str, id: &'a str, clip_id: &'a str },
    SpecifiedMotion { target: &'a str, id: &'a str, index: &'a str },
    AddAnim { target: &'a str, id: &'a str, index: usize },
    AddMotion { target: &'a str, id: &'a str, index: usize },
}

/// Deduplicates ADSF patches by target and entry.
///
/// `Edit*` and `Specified*` patches targeting the same entry are resolved by
/// priority. Normal `Add*` patches remain independent because their clip IDs
/// are assigned later.
pub(super) fn dedup_patches_by_priority_parallel<'a>(
    patches: Vec<AdsfPatch<'a>>,
) -> Vec<AdsfPatch<'a>> {
    patches
        .into_par_iter()
        .enumerate()
        .fold(HashMap::default, |mut map: HashMap<PatchKey<'_>, AdsfPatch<'a>>, (idx, patch)| {
            let key = match &patch.patch {
                PatchKind::ProjectNamesHeader(_) => {
                    PatchKey::ProjectNamesHeader { target: patch.target, id: patch.id }
                }

                PatchKind::AnimDataHeader(_) => {
                    PatchKey::AnimDataHeader { target: patch.target, id: patch.id }
                }

                PatchKind::EditAnim(edit) => PatchKey::EditAnim {
                    target: patch.target,
                    id: patch.id,
                    name_clip: edit.name_clip,
                },

                PatchKind::SpecifiedAnim { name_clip, .. } => {
                    PatchKey::SpecifiedAnim { target: patch.target, id: patch.id, name_clip }
                }

                PatchKind::EditMotion(edit) => PatchKey::EditMotion {
                    target: patch.target,
                    id: patch.id,
                    clip_id: edit.clip_id,
                },

                PatchKind::SpecifiedMotion { index, .. } => {
                    PatchKey::SpecifiedMotion { target: patch.target, id: patch.id, index }
                }

                PatchKind::AddAnim(_) => {
                    PatchKey::AddAnim { target: patch.target, id: patch.id, index: idx }
                }

                PatchKind::AddMotion(_) => {
                    PatchKey::AddMotion { target: patch.target, id: patch.id, index: idx }
                }
            };

            match map.entry(key) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(patch);
                }

                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    match (&entry.get().patch, &patch.patch) {
                        (PatchKind::EditAnim(existing), PatchKind::EditAnim(new)) => {
                            if new.priority > existing.priority {
                                entry.insert(patch);
                            }
                        }

                        (
                            PatchKind::SpecifiedAnim { priority: existing_priority, .. },
                            PatchKind::SpecifiedAnim { priority: new_priority, .. },
                        )
                        | (
                            PatchKind::SpecifiedMotion { priority: existing_priority, .. },
                            PatchKind::SpecifiedMotion { priority: new_priority, .. },
                        ) => {
                            if new_priority > existing_priority {
                                entry.insert(patch);
                            }
                        }

                        (PatchKind::EditMotion(existing), PatchKind::EditMotion(new))
                            if new.priority > existing.priority =>
                        {
                            entry.insert(patch);
                        }

                        _ => {}
                    }
                }
            }

            map
        })
        .reduce(HashMap::default, |mut old_map, new_map| {
            for (key, new_patch) in new_map {
                match old_map.entry(key) {
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        entry.insert(new_patch);
                    }

                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        match (&entry.get().patch, &new_patch.patch) {
                            (PatchKind::EditAnim(existing), PatchKind::EditAnim(new)) => {
                                if new.priority > existing.priority {
                                    entry.insert(new_patch);
                                }
                            }

                            (
                                PatchKind::SpecifiedAnim { priority: existing_priority, .. },
                                PatchKind::SpecifiedAnim { priority: new_priority, .. },
                            )
                            | (
                                PatchKind::SpecifiedMotion { priority: existing_priority, .. },
                                PatchKind::SpecifiedMotion { priority: new_priority, .. },
                            ) => {
                                if new_priority > existing_priority {
                                    entry.insert(new_patch);
                                }
                            }

                            (PatchKind::EditMotion(existing), PatchKind::EditMotion(new))
                                if new.priority > existing.priority =>
                            {
                                entry.insert(new_patch);
                            }

                            _ => {}
                        }
                    }
                }
            }

            old_map
        })
        .into_values()
        .collect()
}
