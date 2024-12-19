use super::aliases::{MergedPatchMap, TemplatePatchMap};
use crate::error::Result;
use crate::output_path::get_nemesis_id;
use json_patch::JsonPatch;
use rayon::prelude::*;
use std::path::PathBuf;

pub fn paths_to_ids(paths: &[PathBuf]) -> Vec<String> {
    paths
        .par_iter()
        .filter_map(|path| get_nemesis_id(path).ok())
        .collect()
}

pub fn merge_patches<'p>(
    patches: TemplatePatchMap<'p>,
    ids: &[String],
) -> Result<MergedPatchMap<'p>> {
    let merged_patches = MergedPatchMap::new();

    patches.into_par_iter().for_each(|idx_map| {
        let (template_name, patch_idx_map) = idx_map;

        patch_idx_map.into_par_iter().for_each(|patch| {
            let (_index, mut patches) = patch;

            let sorted_patches: Vec<Vec<JsonPatch<'p>>> = ids
                .iter()
                .filter_map(|id| patches.remove(id))
                .collect::<Vec<_>>();

            let mut merged_result = Vec::new();
            for patch_vec in sorted_patches {
                merge_json_patches(&mut merged_result, patch_vec);
            }

            merged_patches
                .entry(template_name.clone())
                .or_default()
                .par_extend(merged_result);
        });
    });

    Ok(merged_patches)
}

fn merge_json_patches<'p>(base: &mut Vec<JsonPatch<'p>>, additional: Vec<JsonPatch<'p>>) {
    for patch in additional {
        if !base.contains(&patch) {
            base.push(patch);
        }
    }
}
