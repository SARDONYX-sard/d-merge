use crate::behaviors::tasks::templates::collect::path::template_name_and_inner_path;
use crate::behaviors::tasks::templates::types::{
    BorrowedTemplateMap, OwnedTemplateMap, TemplateKey,
};
use crate::errors::{
    Error, FailedToGetInnerPathFromTemplateSnafu, JsonSnafu, Result, TemplateSnafu,
};
use rayon::{iter::Either, prelude::*};
use simd_json::{serde::to_borrowed_value, BorrowedValue};
use snafu::ResultExt as _;
use std::path::Path;

/// Return  Map<name, (inner_path, value)>
pub fn collect_templates(templates: &OwnedTemplateMap) -> (BorrowedTemplateMap<'_>, Vec<Error>) {
    templates.into_par_iter().partition_map(|(path, bytes)| {
        let parse_template = || -> Result<(TemplateKey, (&str, BorrowedValue<'_>))> {
            fn is_value_bin(path: &Path) -> bool {
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("bin"))
            }
            fn is_xml(path: &Path) -> bool {
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"))
            }

            let value = match path {
                path if is_value_bin(path) => template_bin_to_value(bytes, path),
                path if is_xml(path) => template_xml_to_value(bytes, path),
                other => {
                    return Err(Error::UnsupportedTemplatePath {
                        path: other.clone(),
                    })
                }
            }?;
            let (name, inner_path) = template_name_and_inner_path(path)
                .with_context(|_| FailedToGetInnerPathFromTemplateSnafu { path: path.clone() })?;

            Ok((name, (inner_path, value)))
        };

        match parse_template() {
            Ok(v) => Either::Left(v),
            Err(e) => Either::Right(e),
        }
    })
}

pub(crate) fn template_xml_to_value(bytes: &[u8], path: &Path) -> Result<BorrowedValue<'static>> {
    let template_xml = core::str::from_utf8(bytes).map_err(|_| Error::NonUtf8Path {
        path: path.to_path_buf(),
    })?;
    let ast: serde_hkx_features::ClassMap = serde_hkx::from_str(template_xml)?;
    let value = to_borrowed_value(ast).with_context(|_| JsonSnafu { path })?;
    Ok(value)
}

pub(super) fn template_bin_to_value<'a>(
    template_bytes: &'a [u8],
    path: &Path,
) -> Result<BorrowedValue<'a>> {
    rmp_serde::from_slice(template_bytes).with_context(|_| TemplateSnafu { path })
}
