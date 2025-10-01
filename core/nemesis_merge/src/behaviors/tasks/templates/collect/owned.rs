use rayon::{iter::Either, prelude::*};
use std::{collections::HashSet, path::Path};

use crate::{
    behaviors::tasks::templates::{key::TemplateKey, types::OwnedTemplateMap},
    errors::Error,
};

/// Collect templates path & content map.
///
/// - `template_root`: meshes parent dir. e.g. `assets/templates`. This means search `asserts/templates/meshes/...`
pub fn collect_templates(
    template_root: &Path,
    template_names: HashSet<TemplateKey<'_>>,
) -> (OwnedTemplateMap, Vec<Error>) {
    template_names
        .into_par_iter()
        .map(|template_key| {
            // Intended sample: `../d_merge/asserts/templates/meshes/actors/character/behaviors/0_master.bin`
            let template_path = template_root.join(template_key.as_meshes_inner_path());

            if !template_path.exists() || !template_path.is_file() {
                return Either::Right(Error::NotFoundTemplate {
                    template_name: template_path.display().to_string(),
                });
            }

            match std::fs::read(&template_path) {
                Ok(bytes) => Either::Left((template_path, bytes)),
                Err(_err) => Either::Right(Error::NotFoundTemplate {
                    template_name: template_path.display().to_string(),
                }),
            }
        })
        .partition_map(|either| either)
}
