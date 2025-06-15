use rayon::prelude::*;
use std::collections::HashMap;

use crate::adsf::{AdsfPatch, PatchKind};

#[derive(Hash, Eq, PartialEq)]
enum PatchKey<'a> {
    // (target, id)
    EditAnim(&'a str, &'a str),
    // (target, id)
    EditMotion(&'a str, &'a str),
    AddAnim(&'a str),
    AddMotion(&'a str),
}

pub fn dedup_patches_by_priority_parallel<'a>(patches: Vec<AdsfPatch<'a>>) -> Vec<AdsfPatch<'a>> {
    patches
        .into_par_iter()
        .fold(HashMap::new, |mut map, patch| {
            let key = match &patch.patch {
                PatchKind::EditAnim(_) => PatchKey::EditAnim(patch.target, patch.id),
                PatchKind::EditMotion(_) => PatchKey::EditMotion(patch.target, patch.id),
                PatchKind::AddAnim(_) => PatchKey::AddAnim(patch.target),
                PatchKind::AddMotion(_) => PatchKey::AddMotion(patch.target),
            };

            map.entry(key)
                .and_modify(|existing: &mut AdsfPatch<'a>| {
                    match (&existing.patch, &patch.patch) {
                        (PatchKind::EditAnim(edit_anim), PatchKind::EditAnim(edit_anim2))
                            if edit_anim2.priority > edit_anim.priority =>
                        {
                            if let PatchKind::EditAnim(edit_anim) = &mut existing.patch {
                                edit_anim.patch.merge(edit_anim2.patch.clone());
                                edit_anim.index = edit_anim2.index;
                                edit_anim.priority = edit_anim2.priority;
                            }
                        }
                        (
                            PatchKind::EditMotion(edit_motion),
                            PatchKind::EditMotion(edit_motion2),
                        ) if edit_motion2.priority > edit_motion.priority => {
                            if let PatchKind::EditMotion(edit_motion) = &mut existing.patch {
                                edit_motion.patch.merge(edit_motion2.patch.clone());
                                edit_motion.index = edit_motion2.index;
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
