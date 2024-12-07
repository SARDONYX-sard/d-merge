//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
#![allow(clippy::mem_forget)]
use super::{
    aliases::{BorrowedTemplateMap, ModPatchMap},
    results::filter_results,
};
use crate::error::{Error, NemesisXmlErrSnafu, PatchSnafu, Result};
use json_patch::apply_patch;
use nemesis_xml::patch::parse_nemesis_patch;
use rayon::prelude::*;
use snafu::ResultExt;

pub fn apply_patches<'a, 'b: 'a>(
    templates: &BorrowedTemplateMap<'a>,
    patch_mod_map: &'b ModPatchMap,
) -> Result<(), Vec<Error>> {
    let results = patch_mod_map
        .par_iter()
        .flat_map(|(_mode_code, patch_map)| {
            #[cfg(feature = "tracing")]
            tracing::debug!(_mode_code);
            patch_map.par_iter().map(|(template_target, nemesis_xml)| {
                let patches_json =
                    parse_nemesis_patch(nemesis_xml).with_context(|_| NemesisXmlErrSnafu {
                        path: template_target.clone(),
                    })?;
                #[cfg(feature = "tracing")]
                tracing::debug!(template_target);
                #[cfg(feature = "tracing")]
                tracing::debug!("patches_json = {patches_json:#?}");

                if let Some(mut template_pair) = templates.get_mut(template_target) {
                    let template = &mut template_pair.value_mut().1;
                    for patch in patches_json {
                        let patch_string = format!("{patch:#?}"); // TODO: Fix redundant copy
                        apply_patch(template, patch).with_context(|_| PatchSnafu {
                            template_name: template_target.clone(),
                            patch: patch_string,
                        })?;
                    }
                }
                Ok(())
            })
        })
        .collect();

    filter_results(results)
}
