use rayon::prelude::*;
use std::collections::{hash_map::Entry, HashMap};

use super::{AsdsfPatch, PatchKind};

#[derive(Hash, Eq, PartialEq)]
enum PatchKey<'a> {
    // (target, id, file_name)
    EditAnim(&'a str, &'a str, &'a str),
}

pub fn dedup_patches_by_priority_parallel<'a>(patches: Vec<AsdsfPatch<'a>>) -> Vec<AsdsfPatch<'a>> {
    patches
        .into_par_iter()
        .fold(
            HashMap::new,
            |mut map: HashMap<PatchKey<'_>, AsdsfPatch<'a>>, patch| {
                let key = match &patch.patch {
                    PatchKind::EditAnimSet(edit) => {
                        PatchKey::EditAnim(patch.target, patch.id, edit.file_name)
                    }
                };

                match map.entry(key) {
                    Entry::Occupied(mut entry) => match (&mut entry.get_mut().patch, patch.patch) {
                        (
                            PatchKind::EditAnimSet(ref mut existing_edit),
                            PatchKind::EditAnimSet(new_edit),
                        ) if new_edit.priority > existing_edit.priority => {
                            existing_edit.patch.merge(new_edit.patch);
                            existing_edit.file_name = new_edit.file_name;
                            existing_edit.priority = new_edit.priority;
                        }
                        _ => {}
                    },
                    Entry::Vacant(entry) => {
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
                        match (&entry.get().patch, &patch2.patch) {
                            (
                                PatchKind::EditAnimSet(edit_anim),
                                PatchKind::EditAnimSet(edit_anim2),
                            ) if edit_anim2.priority > edit_anim.priority => {
                                entry.insert(patch2);
                            }
                            _ => {}
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
