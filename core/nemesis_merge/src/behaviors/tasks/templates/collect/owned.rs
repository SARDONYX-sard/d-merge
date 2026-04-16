use std::path::Path;

use rayon::{iter::Either, prelude::*};

use crate::{
    behaviors::tasks::{patches::types::BehaviorPatchesMap, templates::types::OwnedTemplateMap},
    errors::Error,
};

/// Collect templates path & content map.
///
/// - `template_root`: meshes parent dir. e.g. `assets/templates`. This means search `asserts/templates/meshes/...`
pub fn collect_templates<'a>(
    template_root: &Path,
    template_names: &BehaviorPatchesMap<'a>,
) -> (OwnedTemplateMap, Vec<Error>) {
    template_names
        .0
        .par_iter()
        .partition_map(|(template_key, _)| {
            // Intended sample:
            // - `../d_merge/asserts/templates/meshes/actors/character/behaviors/0_master.bin`
            // - `../d_merge/asserts/templates/meshes/actors/character/behaviors/0_master.xml`
            let template_path = template_root.join(template_key.as_meshes_inner_path());

            if !template_path.exists() || !template_path.is_file() {
                return Either::Right(Error::NotFoundTemplate {
                    template_name: template_path.display().to_string(),
                });
            }

            match std::fs::read(&template_path) {
                Ok(bytes) => Either::Left((template_key.clone(), bytes)),
                Err(_err) => Either::Right(Error::NotFoundTemplate {
                    template_name: template_path.display().to_string(),
                }),
            }
        })
}
