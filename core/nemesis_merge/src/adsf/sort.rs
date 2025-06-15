use rayon::prelude::*;
use std::collections::HashMap;

use crate::adsf::{AdsfPatch, PatchKind};

#[derive(Hash, Eq, PartialEq)]
enum PatchKey<'a> {
    // (target, id, clip_id)
    EditAnim(&'a str, &'a str, &'a str),
    // (target, id, clip_id)
    EditMotion(&'a str, &'a str, &'a str),
    // (target, id, index)
    AddAnim(&'a str, &'a str, usize),
    // (target, id, index)
    AddMotion(&'a str, &'a str, usize),
}

pub fn dedup_patches_by_priority_parallel<'a>(patches: Vec<AdsfPatch<'a>>) -> Vec<AdsfPatch<'a>> {
    patches
        .into_par_iter()
        .enumerate()
        .fold(HashMap::new, |mut map, (idx, patch)| {
            let key = match &patch.patch {
                PatchKind::EditAnim(edit) => {
                    PatchKey::EditAnim(patch.target, patch.id, edit.name_clip)
                }
                PatchKind::EditMotion(edit) => {
                    PatchKey::EditMotion(patch.target, patch.id, edit.clip_id)
                }
                PatchKind::AddAnim(_) => PatchKey::AddAnim(patch.target, patch.id, idx),
                PatchKind::AddMotion(_) => PatchKey::AddMotion(patch.target, patch.id, idx),
            };

            map.entry(key)
                .and_modify(|existing: &mut AdsfPatch<'a>| {
                    match (&existing.patch, patch.patch.clone()) {
                        (PatchKind::EditAnim(edit_anim), PatchKind::EditAnim(edit_anim2))
                            if edit_anim2.priority > edit_anim.priority =>
                        {
                            if let PatchKind::EditAnim(edit_anim) = &mut existing.patch {
                                edit_anim.patch.merge(edit_anim2.patch);
                                edit_anim.name_clip = edit_anim2.name_clip;
                                edit_anim.priority = edit_anim2.priority;
                            }
                        }
                        (
                            PatchKind::EditMotion(edit_motion),
                            PatchKind::EditMotion(edit_motion2),
                        ) if edit_motion2.priority > edit_motion.priority => {
                            if let PatchKind::EditMotion(edit_motion) = &mut existing.patch {
                                edit_motion.patch.merge(edit_motion2.patch);
                                edit_motion.clip_id = edit_motion2.clip_id;
                                edit_motion.priority = edit_motion2.priority;
                            }
                        }
                        _ => {} // do nothing
                    }
                })
                .or_insert(patch);

            map
        })
        .reduce(HashMap::new, |mut map1, map2| {
            for (key, patch2) in map2 {
                map1.entry(key)
                    .and_modify(|patch1| match (&patch1.patch, &patch2.patch) {
                        (PatchKind::EditAnim(edit_anim), PatchKind::EditAnim(edit_anim2))
                            if edit_anim2.priority > edit_anim.priority =>
                        {
                            *patch1 = patch2.clone();
                        }
                        (
                            PatchKind::EditMotion(edit_motion),
                            PatchKind::EditMotion(edit_motion2),
                        ) if edit_motion2.priority > edit_motion.priority => {
                            *patch1 = patch2.clone();
                        }
                        _ => {}
                    })
                    .or_insert(patch2);
            }
            map1
        })
        .into_values()
        .collect()
}
