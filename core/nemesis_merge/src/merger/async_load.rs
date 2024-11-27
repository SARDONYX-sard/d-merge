//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.

use crate::{
    collect_path::collect_nemesis_paths,
    error::{Error, FailedIoSnafu, JsonSnafu, NemesisXmlErrSnafu, Result},
    output_path::parse_input_nemesis_path,
};
use dashmap::DashMap;
use nemesis_xml::patch::parse_nemesis_patch;
use serde_hkx_features::ClassMap;
use simd_json::BorrowedValue;
use snafu::ResultExt;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use super::tables;

/// - key: template file stem
/// - value: (`template xml`, `template xml converted to json with reference`)
type Templates<'a> = Arc<DashMap<String, (String, BorrowedValue<'a>)>>;

/// Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
///
/// # Arguments
/// - `output_dir`: Directory where the JSON output will be saved.
/// - `ids`: A vector of strings representing paths to Nemesis XML files.
///
/// # Errors
/// Returns an error if file parsing, I/O operations, or JSON serialization fails.
pub async fn behavior_gen(output_dir: impl AsRef<Path>, ids: Vec<String>) -> Result<()> {
    let output_dir = output_dir.as_ref();
    let templates = Arc::new(DashMap::new());

    // Create async tasks for each ID
    let tasks: Vec<_> = ids
        .into_iter()
        .map(|id| {
            let templates = Arc::clone(&templates);
            tokio::spawn(process_id(output_dir.to_path_buf(), id, templates))
        })
        .collect();

    // Execute tasks concurrently
    let results = futures::future::join_all(tasks).await;

    let _ = tokio::fs::write(output_dir.join("templates.txt"), format!("{templates:?}")).await;

    // Collect and log errors, if any
    let errors: Vec<String> = results
        .into_iter()
        .map(|join_result| {
            join_result
                .map_err(|err| format!("Task failed to join: {}", err)) // Handle JoinError
                .and_then(|task_result| {
                    task_result.map_err(|err| format!("Task error: {}", err)) // Handle process_id errors
                })
        })
        .filter_map(Result::err)
        .collect();

    if !errors.is_empty() {
        write_errors_to_file(output_dir, &errors).await?;
    }

    Ok(())
}

/// Processes all XML files for a given ID.
///
/// # Arguments
/// - `output_dir`: Output directory for JSON files.
/// - `id`: Path to the directory containing Nemesis XML files.
/// - `templates`: A shared DashMap for storing and accessing templates.
///
/// # Errors
/// Returns an error if file operations or parsing fail.
async fn process_id(output_dir: PathBuf, id: String, templates: Templates<'_>) -> Result<()> {
    let paths = collect_nemesis_paths(&id);

    let results: Vec<_> = futures::future::join_all(paths.into_iter().map(|path| {
        let templates = Arc::clone(&templates);
        process_file(output_dir.clone(), path, templates)
    }))
    .await;

    // Combine all errors from file processing
    results.into_iter().collect::<Result<()>>()
}

/// Processes an individual XML file and generates a JSON output file.
///
/// # Arguments
/// - `output_dir`: Directory where the JSON output will be saved.
/// - `path`: Path to the XML file to process.
/// - `templates`: A shared DashMap for storing and accessing templates.
///
/// # Errors
/// Returns an error if file parsing, I/O operations, or JSON serialization fails.
async fn process_file(output_dir: PathBuf, path: PathBuf, templates: Templates<'_>) -> Result<()> {
    let input = tokio::fs::read_to_string(&path)
        .await
        .context(FailedIoSnafu { path: path.clone() })?;
    let json_patch = parse_nemesis_patch(&input).context(NemesisXmlErrSnafu {
        xml: input.clone(),
        path: path.clone(),
    })?;

    let output = prepare_output_path(output_dir, &path, &templates).await?;

    let json_str = simd_json::to_string_pretty(&json_patch).context(JsonSnafu {
        path: output.clone(),
    })?;
    tokio::fs::write(&output, json_str)
        .await
        .context(FailedIoSnafu { path: output })?;

    Ok(())
}

/// Prepares the output path for a file, creating necessary directories and handling templates.
///
/// # Arguments
/// - `output_dir`: The base output directory.
/// - `path`: The input XML file path.
/// - `templates`: A shared DashMap for storing and accessing templates.
///
/// # Returns
/// The full path to the output file.
///
/// # Errors
/// Returns an error if directory creation or template processing fails.
async fn prepare_output_path(
    output_dir: PathBuf,
    path: &Path,
    templates: &Templates<'_>,
) -> Result<PathBuf> {
    let output_res =
        parse_input_nemesis_path(path).ok_or_else(|| Error::FailedParseNemesisPath {
            path: path.to_path_buf(),
        })?;

    let mut output = output_dir.join(output_res.relevant_path);
    output.set_extension("json");

    if let Some(output_dir) = output.parent() {
        tokio::fs::create_dir_all(output_dir)
            .await
            .context(FailedIoSnafu {
                path: output_dir.to_path_buf(),
            })?;
    }

    // Process template if needed
    {
        let template_name = output_res.template_name;
        if !templates.contains_key(&template_name) {
            let resource_root = Path::new("../../assets/templates/");
            let template_path = resource_root.join(
                if output_res.is_1st_person {
                    tables::FIRST_PERSON_BEHAVIORS.get(&template_name)
                } else {
                    tables::THIRD_PERSON_BEHAVIORS.get(&template_name)
                }
                .unwrap_or(&""),
            );

            if let Ok(template_xml) = tokio::fs::read_to_string(template_path).await {
                let ast: ClassMap = serde_hkx::from_str(&template_xml)?;
                let json: BorrowedValue<'_> =
                    simd_json::serde::to_borrowed_value(ast).context(JsonSnafu {
                        path: output.clone(),
                    })?;
                templates.insert(template_name, (template_xml, json));
            }
        }
    }

    Ok(output)
}

/// Writes all collected errors to a text file in the output directory.
///
/// # Arguments
/// - `output_dir`: The base output directory.
/// - `errors`: A vector of error messages.
///
/// # Errors
/// Returns an error if directory creation or file writing fails.
async fn write_errors_to_file(output_dir: &Path, errors: &[String]) -> Result<()> {
    tokio::fs::create_dir_all(output_dir)
        .await
        .context(FailedIoSnafu {
            path: output_dir.to_path_buf(),
        })?;
    let output = output_dir.join("errors.txt");
    tokio::fs::write(&output, errors.join("\n"))
        .await
        .context(FailedIoSnafu { path: output })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "unimplemented yet"]
    // #[quick_tracing::init]
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
        .map(|s| s.to_string())
        .collect();
        let output_dir = "../../dummy/patches/output";
        std::fs::create_dir_all(output_dir).unwrap();

        behavior_gen(output_dir, ids).await.unwrap();
    }
}
