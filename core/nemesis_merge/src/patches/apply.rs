//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
use crate::{
    aliases::{BorrowedTemplateMap, MergedPatchMap},
    errors::{Error, PatchSnafu, Result},
    results::filter_results,
};
use json_patch::apply_patch;
use rayon::prelude::*;
use snafu::ResultExt;

/// Apply to hkx with merged json patch.
pub fn apply_patches<'a, 'b: 'a>(
    templates: &BorrowedTemplateMap<'a>,
    patch_mod_map: MergedPatchMap<'b>,
    // nemesis_vars: NemesisVars,
) -> Result<(), Vec<Error>> {
    //
    let results = patch_mod_map // patches
        .into_par_iter()
        .map(|(template_name, patches)| {
            // template_name: e.g. 0_master.hkx -> 0_master
            // patches: patches for 0_master.hkx
            if let Some(mut template_pair) = templates.get_mut(&template_name) {
                let template = &mut template_pair.value_mut().1;

                // update_id_path(&patches, &mut nemesis_vars);

                // TODO: Replace variables to indexes (in advance, update id_path in template)
                for (path, patch) in patches {
                    let patch_string = format!("{patch:#?}"); // TODO: Fix redundant copy
                    apply_patch(template, path, patch).with_context(|_| PatchSnafu {
                        template_name: template_name.clone(),
                        patch: patch_string,
                    })?;
                }
                // super::replace_vars::replace_var(template, &nemesis_vars);
            }

            Ok(())
        })
        .collect();

    filter_results(results)
}
