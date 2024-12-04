//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
#![allow(clippy::mem_forget)]
use super::{BorrowedTemplateMap, ModPatchMap};
use crate::error::{Error, NemesisXmlErrSnafu, PatchSnafu, Result};
use json_patch::apply_patch;
use nemesis_xml::patch::parse_nemesis_patch;
use rayon::prelude::*;
use snafu::ResultExt;

pub fn apply_patches<'a, 'b: 'a>(
    templates: &BorrowedTemplateMap<'a>,
    patch_mod_map: &'b ModPatchMap,
) -> Vec<Result<(), Error>> {
    patch_mod_map
        .par_iter()
        .flat_map(|(_, patch_map)| {
            patch_map.par_iter().map(|(template_target, nemesis_xml)| {
                let patches_json =
                    parse_nemesis_patch(nemesis_xml).context(NemesisXmlErrSnafu {
                        path: template_target.clone(),
                    })?;
                if let Some(mut template_pair) = templates.get_mut(template_target) {
                    let template = &mut template_pair.value_mut().1;
                    for patch in patches_json {
                        let patch_string = format!("{patch:#?}");
                        apply_patch(template, patch).context(PatchSnafu {
                            template_name: template_target.clone(),
                            patch: patch_string,
                        })?;
                    }
                }
                Ok(())
            })
        })
        .collect()
}
