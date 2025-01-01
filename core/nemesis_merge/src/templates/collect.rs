use crate::errors::{Error, FailedIoSnafu, JsonSnafu, NotFoundTemplateSnafu, Result};
use crate::{aliases::BorrowedTemplateMap, tables::TEMPLATE_BEHAVIORS};
use dashmap::DashSet;
use rayon::{iter::Either, prelude::*};
use serde_hkx_features::ClassMap;
use simd_json::{serde::to_borrowed_value, BorrowedValue};
use snafu::{OptionExt, ResultExt as _};
use std::{fs, path::Path};

pub fn collect_templates<'a>(
    template_names: DashSet<String>,
    resource_dir: &Path,
) -> (BorrowedTemplateMap<'a>, Vec<Error>) {
    let results: Vec<Result<(String, (&'static str, BorrowedValue<'static>))>> = template_names
        .into_par_iter()
        .map(|name| {
            let value = template_xml_to_value(name.as_str(), resource_dir)?;
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
) -> Result<(&'static str, BorrowedValue<'static>)> {
    let inner_path = TEMPLATE_BEHAVIORS
        .get(template_name)
        .with_context(|| NotFoundTemplateSnafu { template_name })?;

    let path = resource_dir.join(inner_path);
    let template_xml = fs::read_to_string(path).context(FailedIoSnafu { path: inner_path })?;
    let ast: ClassMap = serde_hkx::from_str(&template_xml)?;
    let value = to_borrowed_value(ast).with_context(|_| JsonSnafu { path: inner_path })?;
    Ok((inner_path, value))
}
