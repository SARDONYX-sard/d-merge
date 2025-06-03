use dashmap::DashSet;
use snafu::ResultExt as _;
use std::path::Path;

use crate::{
    errors::{Error, FailedIoSnafu},
    types::OwnedTemplateMap,
};

/// Return HashMap<template key, `meshes` inner path>
pub async fn collect_templates<P>(
    path: P,
    template_names: DashSet<&str>,
) -> (OwnedTemplateMap, Vec<Error>)
where
    P: AsRef<Path>,
{
    let iter = jwalk::WalkDir::new(path).into_iter().filter_map(|path| {
        let path = path.ok()?.path();
        if !path.is_file() {
            return None;
        }
        let file_stem = path.file_stem()?;
        if file_stem.eq_ignore_ascii_case("animationdatasinglefile") {
            return None;
        }
        if !template_names.contains(file_stem.to_str()?) {
            return None;
        }
        Some(path)
    });

    let mut task_handles: Vec<tokio::task::JoinHandle<Result<_, Error>>> = vec![];
    for needed_template_path in iter {
        task_handles.push(tokio::spawn(async move {
            let bytes = tokio::fs::read(&needed_template_path)
                .await
                .with_context(|_| FailedIoSnafu {
                    path: needed_template_path.clone(),
                })?;
            Ok((needed_template_path, bytes))
        }));
    }

    let mut map = OwnedTemplateMap::new();
    let mut errors = vec![];
    for handle in task_handles {
        let result = match handle.await {
            Ok(result) => result,
            Err(err) => {
                errors.push(Error::JoinError { source: err });
                continue;
            }
        };
        match result {
            Ok((path, bytes)) => {
                map.insert(path, bytes);
            }
            Err(err) => {
                errors.push(err);
            }
        }
    }

    (map, errors)
}
