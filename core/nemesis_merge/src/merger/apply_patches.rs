//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
use super::{aliases::BorrowedTemplateMap, merge_patches::JsonPatchMap, results::filter_results};
use crate::error::{Error, PatchSnafu, Result};
use json_patch::apply_patch;
use rayon::prelude::*;
use snafu::ResultExt;

pub fn apply_patches<'a, 'b: 'a>(
    templates: &BorrowedTemplateMap<'a>,
    patch_mod_map: JsonPatchMap<'b>,
) -> Result<(), Vec<Error>> {
    let results = patch_mod_map
        .into_par_iter()
        .map(|(template_target, patches)| {
            if let Some(mut template_pair) = templates.get_mut(template_target) {
                let template = &mut template_pair.value_mut().1;
                for patch in patches {
                    let patch_string = format!("{patch:#?}"); // TODO: Fix redundant copy
                    apply_patch(template, patch).with_context(|_| PatchSnafu {
                        template_name: template_target.clone(),
                        patch: patch_string,
                    })?;
                }
            }
            Ok(())
        })
        .collect();

    filter_results(results)
}
