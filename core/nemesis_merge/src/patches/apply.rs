//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
use crate::{
    errors::{Error, PatchSnafu, Result},
    results::filter_results,
    types::{BorrowedTemplateMap, PatchMapForEachTemplate},
};
use json_patch::apply_patch;
use rayon::prelude::*;
use snafu::ResultExt;
use std::path::Path;

/// Apply to hkx with merged json patch.
pub fn apply_patches<'a, 'b: 'a>(
    templates: &BorrowedTemplateMap<'a>,
    patch_map_foreach_template: PatchMapForEachTemplate<'b>,
    output_dir: &Path,
) -> Result<(), Vec<Error>> {
    let results = patch_map_foreach_template // patches
        .into_par_iter()
        .map(|(template_name, patches)| {
            let _output_dir = output_dir;
            #[cfg(feature = "debug")]
            write_json_patch(_output_dir, template_name, &patches)?;

            // template_name: e.g. 0_master.hkx -> 0_master
            // patches: patches for 0_master.hkx
            if let Some(mut template_pair) = templates.get_mut(&template_name) {
                let template = &mut template_pair.value_mut().1;

                for (path, patch) in patches.0 {
                    let patch_string = format!("{patch:#?}"); // TODO: Fix redundant copy
                    apply_patch(template, path, patch).with_context(|_| PatchSnafu {
                        template_name,
                        patch: patch_string,
                    })?;
                }
            }

            Ok(())
        })
        .collect();

    filter_results(results)
}

#[cfg(feature = "debug")]
fn write_json_patch(
    output_dir: &Path,
    template_name: &str,
    patches: &crate::types::PatchMap,
) -> Result<(), Error> {
    use crate::errors::FailedIoSnafu;
    use snafu::ResultExt as _;

    let output_dir = output_dir.join(".debug").join("patches");

    let output_dir_1st_person = output_dir.join("_1stperson");
    std::fs::create_dir_all(&output_dir_1st_person).context(FailedIoSnafu {
        path: output_dir_1st_person,
    })?;
    std::fs::create_dir_all(&output_dir).context(FailedIoSnafu {
        path: output_dir.clone(),
    })?;

    let output_path = output_dir.join(format!("{template_name}.patch.json"));
    let json = simd_json::to_string_pretty(patches).with_context(|_| crate::errors::JsonSnafu {
        path: output_path.clone(),
    })?;
    std::fs::write(&output_path, &json).context(FailedIoSnafu { path: output_path })?;

    Ok(())
}
