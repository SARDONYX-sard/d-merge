use crate::aliases::BorrowedTemplateMap;
use crate::errors::{Error, FailedIoSnafu, JsonSnafu, NotFoundTemplateSnafu, Result};
use crate::templates::tables::collect_table_paths;
use dashmap::{DashMap, DashSet};
use rayon::{iter::Either, prelude::*};
use serde_hkx_features::ClassMap;
use simd_json::{serde::to_borrowed_value, BorrowedValue};
use snafu::{OptionExt, ResultExt as _};
use std::path::PathBuf;
use std::{fs, path::Path};

pub fn collect_templates<'a>(
    template_names: DashSet<String>,
    resource_dir: &Path,
) -> (BorrowedTemplateMap<'a>, Vec<Error>) {
    let template_behaviors = collect_table_paths(resource_dir);
    // #[cfg(feature = "tracing")]
    // tracing::trace!("{template_behaviors:#?}");

    let results: Vec<Result<(String, (PathBuf, BorrowedValue<'static>))>> = template_names
        .into_par_iter()
        .map(|name| {
            let value = template_xml_to_value(name.as_str(), resource_dir, &template_behaviors)?;
            Ok((name, value))
        })
        .collect();

    results.into_par_iter().partition_map(|res| match res {
        Ok(value) => Either::Left(value),
        Err(err) => Either::Right(err),
    })
}

fn template_xml_to_value(
    template_name: &str,
    resource_dir: &Path,
    template_behaviors: &DashMap<String, PathBuf>,
) -> Result<(PathBuf, BorrowedValue<'static>)> {
    let inner_path = template_behaviors
        .get(template_name)
        .with_context(|| NotFoundTemplateSnafu { template_name })?
        .value()
        .to_owned();

    let path = resource_dir.join(&inner_path);
    let template_xml = fs::read_to_string(path).context(FailedIoSnafu {
        path: inner_path.clone(),
    })?;
    let ast: ClassMap = serde_hkx::from_str(&template_xml)?;
    let value = to_borrowed_value(ast).with_context(|_| JsonSnafu {
        path: inner_path.clone(),
    })?;
    Ok((inner_path, value))
}
