//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
#![allow(clippy::mem_forget)]
use super::tables::{FIRST_PERSON_BEHAVIORS, THIRD_PERSON_BEHAVIORS};
use crate::{
    collect_path::collect_nemesis_paths,
    error::{Error, FailedIoSnafu, JsonSnafu, NemesisXmlErrSnafu, PatchSnafu, Result},
    output_path::{parse_input_nemesis_path, NemesisPath},
};
use json_patch::apply_patch;
use nemesis_xml::patch::parse_nemesis_patch;
use serde_hkx::bytes::serde::hkx_header::HkxHeader;
use serde_hkx_features::{fs::ReadExt, ClassMap};
use simd_json::{
    serde::{from_borrowed_value, to_borrowed_value},
    BorrowedValue,
};
use snafu::ResultExt;
use std::{collections::HashMap, path::PathBuf};
use tokio::fs;

/// - key: template file stem(e.g. `0_master`)
/// - value: output_path(hkx file path), borrowed json (from template xml)
type TemplateMap<'a> = HashMap<String, (PathBuf, BorrowedValue<'a>)>;

/// - key: merge target template file stem (e.g. `0_master`)
/// - value: nemesis patch xml(from hkx file)
type PatchMapOwned = HashMap<String, String>;
/// - key: (e.g. `0_master`) template file stem.
/// - key: (e.g. `aaaa`) mod code
/// - value: nemesis patch xml files
type PatchModMap<'a> = HashMap<String, PatchMapOwned>;

#[derive(Debug)]
pub struct Options {
    resource_dir: PathBuf,
    output_dir: PathBuf,
}

/// - nemesis_paths: `e.g. vec!["../../dummy/Data/Nemesis_Engine/mod/aaaaa"]`
/// - `resource_dir`: Path of the template from which the patch was applied.(e.g. `../templates/` => `../templates/meshes`)
///
/// # Errors
/// Returns an error if file parsing, I/O operations, or JSON serialization fails.
pub async fn behavior_gen(nemesis_paths: Vec<PathBuf>, options: Options) -> Result<()> {
    let mut patch_mod_map = PatchModMap::new();
    {
        let mut templates = TemplateMap::new();

        for patch_path in &nemesis_paths {
            let mut patch_map_owned = PatchMapOwned::new();

            for txt_path in collect_nemesis_paths(patch_path) {
                let NemesisPath {
                    relevant_path: _,
                    template_name,
                    is_1stperson,
                } = parse_input_nemesis_path(&txt_path).ok_or(Error::FailedParseNemesisPath {
                    path: txt_path.clone(),
                })?;
                let template_name_key = match is_1stperson {
                    true => format!("_1stperson/{template_name}"),
                    false => template_name.clone(),
                };

                // one template
                {
                    let (output_path, template_path) = match is_1stperson {
                        true => FIRST_PERSON_BEHAVIORS.get(&template_name),
                        false => THIRD_PERSON_BEHAVIORS.get(&template_name),
                    }
                    .map(|path| {
                        let Options {
                            resource_dir,
                            output_dir,
                        } = &options;
                        let mut output_path = output_dir.join(path);
                        output_path.set_extension("hkx");
                        (output_path, resource_dir.join(path))
                    })
                    .ok_or_else(|| Error::FailedParseNemesisPath {
                        path: txt_path.clone(),
                    })?;
                    let json_value = template_xml_to_value(template_path).await?;
                    templates.insert(template_name_key.clone(), (output_path, json_value));
                };

                // one patch
                let patch_txt = fs::read_to_string(&txt_path).await.context(FailedIoSnafu {
                    path: txt_path.clone(),
                })?;
                patch_map_owned.insert(template_name_key, patch_txt);
            }

            if let Some(mod_code) = patch_path.file_name() {
                patch_mod_map.insert(mod_code.to_string_lossy().to_string(), patch_map_owned);
            }
        }

        // TODO: Priority joins between patches may allow templates to be processed in a parallel loop.
        // 2. apply patches
        for patch_map in patch_mod_map.values() {
            for (template_target, patch_txt) in patch_map {
                let patches_json = parse_nemesis_patch(patch_txt).context(NemesisXmlErrSnafu {
                    path: template_target.clone(),
                })?;
                if let Some((_, template)) = templates.get_mut(template_target) {
                    for patch in patches_json {
                        apply_patch(template, patch).context(PatchSnafu {
                            template_name: template_target.clone(),
                        })?;
                    }
                }
            }
        }

        save_templates_to_hkx(templates).await?;
    };

    Ok(())
}

async fn template_xml_to_value(path: PathBuf) -> Result<BorrowedValue<'static>> {
    let template_xml = path.read_any_string().await?;
    let ast: ClassMap = serde_hkx::from_str(&template_xml)?;
    to_borrowed_value(ast).context(JsonSnafu { path }) // TODO: fix needless realloc
}

async fn save_templates_to_hkx(templates: TemplateMap<'_>) -> Result<()> {
    for (output_path, template_json) in templates.into_values() {
        if let Some(output_dir_all) = output_path.parent() {
            fs::create_dir_all(output_dir_all)
                .await
                .context(FailedIoSnafu {
                    path: output_dir_all,
                })?;
        }

        let ast: ClassMap = from_borrowed_value(template_json).context(JsonSnafu {
            path: output_path.clone(),
        })?;
        let hkx_bytes = serde_hkx::to_bytes(&ast, &HkxHeader::new_skyrim_se())?;
        fs::write(&output_path, hkx_bytes)
            .await
            .context(FailedIoSnafu {
                path: output_path.clone(),
            })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "unimplemented yet"]
    #[quick_tracing::init]
    async fn merge_test() {
        #[allow(clippy::iter_on_single_items)]
        let ids = [
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\aaaaa",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\bcbi",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\cbbi",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\gender",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\hmce",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\momo",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\na1w",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\nemesis",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\pscd",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\rthf",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\skice",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\sscb",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\tkuc",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\tudm",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\turn",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\zcbe",
        ]
        .into_iter()
        .map(|s| s.into())
        .collect();

        behavior_gen(
            ids,
            Options {
                resource_dir: "../../assets/templates".into(),
                output_dir: "../../dummy/patches_applied/output".into(),
            },
        )
        .await
        .unwrap();
    }
}
