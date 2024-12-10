use super::aliases::ModPatchMap;
use crate::error::{NemesisXmlErrSnafu, Result};
use crate::output_path::parse_input_nemesis_path;
use json_patch::{JsonPatch, Op};
use nemesis_xml::patch::parse_nemesis_patch;
use rayon::iter::ParallelExtend;
use snafu::ResultExt as _;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;

type JsonPatchMap<'a> = HashMap<&'a String, Vec<JsonPatch<'a>>>;
pub type TemplatePatches<'a> = HashMap<String, Vec<JsonPatch<'a>>>;

pub fn paths_to_ids(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .filter_map(|path| parse_input_nemesis_path(path).map(|parsed| parsed.template_name))
        .collect()
}

pub fn merge_mod_patches(
    mod_patch_map: &ModPatchMap,
    priority_ids: Vec<String>,
) -> Result<TemplatePatches<'_>> {
    let json_patch_map = convert_mod_patch_map(mod_patch_map)?;
    #[cfg(feature = "tracing")]
    tracing::debug!("json_patch_map = {json_patch_map:#?}");
    let template_patches = merge_patches_with_priority(priority_ids, json_patch_map);
    #[cfg(feature = "tracing")]
    tracing::debug!("template_patches = {template_patches:#?}");
    Ok(template_patches)
}

fn convert_mod_patch_map(mod_patch_map: &ModPatchMap) -> Result<JsonPatchMap<'_>> {
    let mut patch_map = HashMap::new();

    for (template, owned_patches) in mod_patch_map {
        let mut json_patches = Vec::new();

        for (mode_code, patch_xml) in owned_patches {
            let parsed_patches =
                parse_nemesis_patch(patch_xml).with_context(|_| NemesisXmlErrSnafu {
                    path: format!("{mode_code}/{template}"),
                })?;
            json_patches.par_extend(parsed_patches);
        }

        patch_map.insert(template, json_patches);
    }

    Ok(patch_map)
}

pub fn merge_patches_with_priority<'a>(
    keys: Vec<String>,
    patches: JsonPatchMap<'a>,
) -> HashMap<String, Vec<JsonPatch<'a>>> {
    let mut merged_patches: HashMap<String, Vec<JsonPatch<'a>>> = HashMap::new();
    let mut path_map: HashMap<Vec<Cow<'a, str>>, JsonPatch<'a>> = HashMap::new();

    for key in keys {
        if let Some(patch_list) = patches.get(&key) {
            for patch in patch_list {
                let path = patch.path.clone();

                if let Some(existing_patch) = path_map.get_mut(&path) {
                    match (&existing_patch.op, &patch.op) {
                        (Op::Add, Op::Add) => {
                            merged_patches
                                .entry(key.clone())
                                .or_default()
                                .push(patch.clone());
                        }
                        (Op::Remove | Op::Replace, Op::Add) => continue,
                        (_, _) => {
                            *existing_patch = patch.clone();
                        }
                    }
                } else {
                    path_map.insert(path.clone(), patch.clone());
                    merged_patches
                        .entry(key.clone())
                        .or_default()
                        .push(patch.clone());
                }
            }
        }
    }

    merged_patches
}

#[cfg(test)]
mod tests {
    use super::*;
    use json_patch::{JsonPatch, Op};
    use simd_json::{BorrowedValue, ValueBuilder as _};
    use std::borrow::Cow;

    #[test]
    fn test_merge_patches_with_priority() {
        // パッチデータ
        let patch1 = JsonPatch {
            op: Op::Add,
            path: vec![Cow::Borrowed("$"), Cow::Borrowed("0029")],
            value: BorrowedValue::from("Value1"),
        };

        let patch2 = JsonPatch {
            op: Op::Remove,
            path: vec![Cow::Borrowed("$"), Cow::Borrowed("0029")],
            value: BorrowedValue::null(),
        };

        let patch3 = JsonPatch {
            op: Op::Add,
            path: vec![Cow::Borrowed("$"), Cow::Borrowed("0029")],
            value: BorrowedValue::from("Value2"),
        };

        let mut patches = HashMap::new();

        let key1 = "low_priority".to_string();
        let key2 = "high_priority".to_string();
        patches.insert(&key1, vec![patch1]);
        patches.insert(&key2, vec![patch2, patch3]);

        let keys = vec!["low_priority".to_string(), "high_priority".to_string()];

        let result = merge_patches_with_priority(keys, patches);

        assert_eq!(result.len(), 1);
        // assert!(result.iter().any(|p| p.value == *"Value2"));
        // assert!(result.iter().any(|p| p.op == Op::Remove));
    }
}
