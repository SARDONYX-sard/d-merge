//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
#![allow(clippy::mem_forget)]
use crate::{
    collect_path::collect_nemesis_paths,
    error::{FailedIoSnafu, JsonSnafu, NemesisXmlErrSnafu, Result},
    output_path::parse_input_nemesis_path,
};
use dashmap::DashMap;
use json_patch::merger::PatchJson;
use nemesis_xml::patch::parse_nemesis_patch;
use serde_hkx_features::ClassMap;
use simd_json::BorrowedValue;
use snafu::ResultExt;
use std::{collections::HashSet, path::PathBuf, sync::Arc};
use tokio::sync::mpsc::Sender;

use super::tables::{FIRST_PERSON_BEHAVIORS, THIRD_PERSON_BEHAVIORS};

/// - key: template file stem(e.g. `0_master`)
/// - value: (`template xml`, `template xml converted to json with reference`)
type TemplateMap = DashMap<String, TemplateJson>;

#[ouroboros::self_referencing]
#[derive(Debug, PartialEq)]
struct TemplateJson {
    xml: String,
    #[borrows(xml)]
    #[covariant]
    template: BorrowedValue<'this>,
}

/// - key: template file stem(e.g. `0_master`)
/// - value: (`nemesis xml`, `nemesis xml converted to json with reference`)
type PatchMap = DashMap<PathBuf, JsonPatches>;

#[ouroboros::self_referencing]
#[derive(Debug, PartialEq)]
struct JsonPatches {
    xml: String,
    #[borrows(xml)]
    #[covariant]
    patches: Vec<PatchJson<'this>>,
}

type ChannelArgs = (String, bool);

#[derive(Debug)]
pub struct BehaviorGenerator {
    resource_dir: PathBuf,
    sender: Option<Sender<ChannelArgs>>,
}

impl BehaviorGenerator {
    pub const fn new(resource_dir: PathBuf) -> Self {
        Self {
            resource_dir,
            sender: None,
        }
    }

    async fn get_template(
        self: Arc<Self>,
        template_info: ChannelArgs,
    ) -> Result<(String, TemplateJson)> {
        let template_name = &template_info.0;
        let is_1st_person = template_info.1;

        let template_path = self.resource_dir.join(
            match is_1st_person {
                true => FIRST_PERSON_BEHAVIORS.get(template_name),
                false => THIRD_PERSON_BEHAVIORS.get(template_name),
            }
            .unwrap_or(&""),
        );

        let template_xml =
            tokio::fs::read_to_string(&template_path)
                .await
                .context(FailedIoSnafu {
                    path: template_path.clone(),
                })?;

        let value = TemplateJsonTryBuilder {
            xml: template_xml,
            template_builder: |template_xml| {
                let ast: ClassMap = serde_hkx::from_str(template_xml)?;
                simd_json::serde::to_borrowed_value(ast).context(JsonSnafu {
                    path: template_path,
                })
            },
        };

        Ok(if is_1st_person {
            (template_info.0, value.try_build()?)
        } else {
            (format!("{template_name}/_1st_person"), value.try_build()?)
        })
    }

    async fn create_nemesis_patch_json(
        self: Arc<Self>,
        patch_xml_path: PathBuf,
    ) -> Result<(PathBuf, JsonPatches)> {
        let xml = tokio::fs::read_to_string(&patch_xml_path)
            .await
            .context(FailedIoSnafu {
                path: patch_xml_path.clone(),
            })?;
        let path = patch_xml_path.clone();
        let value = JsonPatchesTryBuilder {
            xml,
            patches_builder: |xml| {
                parse_nemesis_patch(xml).context(NemesisXmlErrSnafu {
                    xml: xml.to_string(),
                    path,
                })
            },
        };

        if let Some(nemesis_path) = parse_input_nemesis_path(patch_xml_path.as_path()) {
            if let Some(sender) = &self.sender {
                sender
                    .send((nemesis_path.template_name, nemesis_path.is_1st_person))
                    .await
                    .unwrap();
            };
        }
        Ok((patch_xml_path, value.try_build()?))
    }
}

/// - nemesis_paths: `e.g. vec!["../../dummy/Data/Nemesis_Engine/mod/aaaaa"]`
/// - `resource_dir`: Path of the template from which the patch was applied.(e.g. `../templates/` => `../templates/meshes`)
///
/// # Errors
/// Returns an error if file parsing, I/O operations, or JSON serialization fails.
pub async fn behavior_gen(
    nemesis_paths: Vec<PathBuf>,
    mut generator: BehaviorGenerator,
) -> Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<ChannelArgs>(500);

    generator.sender = Some(tx);
    let generator = Arc::new(generator);

    // 1. Parallel read XML templates + Create json patches.
    // captures: nemesis_paths, templates_map, patches_map

    for nemesis_dir in nemesis_paths {
        let mut tasks = vec![];
        for patch_xml_path in collect_nemesis_paths(&nemesis_dir) {
            let generator = Arc::clone(&generator);
            let task =
                tokio::spawn(
                    async move { generator.create_nemesis_patch_json(patch_xml_path).await },
                );

            tasks.push(task);
        }

        let patches = PatchMap::new();
        for task in tasks {
            let (patch_xml_path, value) = task.await??;
            tracing::debug!(?patch_xml_path, ?value);
            patches.insert(patch_xml_path, value);
        }
    }

    // main thread
    let mut tasks = vec![];

    let mut templates_set = HashSet::new();
    while let Some(template_info) = rx.recv().await {
        if templates_set.contains(&template_info.0) {
            continue;
        }
        templates_set.insert(template_info.0.clone());

        let generator = Arc::clone(&generator);
        let task = tokio::spawn(generator.get_template(template_info));
        tasks.push(task);
    }

    let templates = TemplateMap::new();
    for task in tasks {
        let (template_name, value) = task.await??;
        templates.insert(template_name, value);
    }

    // 2. Resolve conflicts
    // captures: patches_map

    // 3. merge json patches into the templates json
    // captures: templates_map, patches_map

    // 4. write the final json(To hkx) to the output_dir
    // captures: templates_map

    // 5. errors are reported
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "unimplemented yet"]
    #[quick_tracing::init]
    async fn merge_test() {
        let ids = [
            "../../dummy\\Data\\Nemesis_Engine\\mod\\aaaaa",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\bcbi",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\cbbi",
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
        let output_dir = "../../dummy/patches/output";
        std::fs::create_dir_all(output_dir).unwrap();

        behavior_gen(
            ids,
            BehaviorGenerator::new("../../assets/templates/".into()),
        )
        .await
        .unwrap();
    }
}
