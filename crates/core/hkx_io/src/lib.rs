//! The **only** place that turns havok classes into bytes / XML / JSON and back.
//!
//! # Why this crate exists (binary size)
//! (De)serializing a [`ClassMap`] goes through code generated for each of the ~700 havok
//! classes, and that code is generic (`from_bytes::<T>`, `process_serde_with::<F, G, T>`, ...).
//! Rust copies generic code into **every crate that uses it** (and LTO does not merge the copies
//! back), so each crate that called `serde_hkx` directly added 15-25 MB to the executable.
//!
//! Here every entry point is a plain (non-generic, `#[inline(never)]`) function, so the
//! generic code is built once, in this crate. Other crates must call these instead of
//! `serde_hkx` / `serde_hkx_features::convert::process_serde_with` directly.
//!
//! | Function             | From / to                                                   |
//! | -------------------- | ----------------------------------------------------------- |
//! | [`from_bytes`]       | `.hkx` bytes -> [`ClassMap`]                                |
//! | [`from_xml`]         | XML -> [`ClassMap`]                                         |
//! | [`from_json_value`]  | `simd_json` value -> [`ClassMap`] (patched templates)       |
//! | [`with_class_map`]   | any format, by file extension (`.hkx`, `.xml`, `.json`, ...) |
//! | [`to_bytes`] / [`to_bytes_with_maps`] | [`ClassMap`] -> `.hkx` bytes               |
//! | [`to_xml`]           | [`ClassMap`] -> XML                                         |
//! | [`convert`]          | a file or directory -> another format (the Convert page)   |

use std::{
    cell::RefCell,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};

pub use havok_types::Pointer;
pub use serde_hkx::{EventIdMap, VariableIdMap, bytes::serde::hkx_header::HkxHeader};
pub use serde_hkx_features::{ClassMap, Format};

/// Deserialization error of `serde_hkx`.
pub type DeError = serde_hkx::errors::de::Error;
/// Serialization error of `serde_hkx`.
pub type SerError = serde_hkx::errors::ser::Error;

/// `.hkx` bytes -> classes.
///
/// # Errors
/// Invalid or unsupported hkx data.
#[inline(never)]
pub fn from_bytes(bytes: &[u8]) -> Result<ClassMap<'_>, DeError> {
    serde_hkx::from_bytes(bytes)
}

/// Havok XML -> classes.
///
/// # Errors
/// Invalid XML or unknown classes.
#[inline(never)]
pub fn from_xml(xml: &str) -> Result<ClassMap<'_>, DeError> {
    serde_hkx::from_str(xml)
}

/// A `simd_json` value (e.g., a patched template) -> classes.
///
/// # Errors
/// The JSON does not describe valid classes.
#[inline(never)]
pub fn from_json_value(
    value: simd_json::BorrowedValue<'_>,
) -> Result<ClassMap<'_>, simd_json::Error> {
    simd_json::serde::from_borrowed_value(value)
}

/// Reads `bytes` in the format of `path`'s extension (`.hkx`, `.xml`, `.json`, `.yaml`, ...)
/// and passes the classes to `f`.
///
/// A callback instead of a return value, because XML is decoded into a temporary string the
/// classes borrow from.
///
/// # Errors
/// Unknown extension, a parse error, or an error of `f`.
#[inline(never)]
pub fn with_class_map(
    bytes: &[u8],
    path: &Path,
    f: &mut dyn FnMut(ClassMap<'_>) -> serde_hkx_features::Result<()>,
) -> serde_hkx_features::Result<()> {
    // `process_serde_with` takes one callback per lifetime case; only one of them runs.
    let f = RefCell::new(f);
    serde_hkx_features::convert::process_serde_with(
        bytes,
        path,
        |classes| (f.borrow_mut())(classes),
        |classes| (f.borrow_mut())(classes),
    )
}

/// Classes -> `.hkx` bytes.
///
/// # Errors
/// A class cannot be serialized.
#[inline(never)]
pub fn to_bytes(classes: &ClassMap<'_>, header: &HkxHeader) -> Result<Vec<u8>, SerError> {
    serde_hkx::to_bytes(classes, header)
}

/// Classes -> `.hkx` bytes, replacing Nemesis event / variable names with their ids.
///
/// # Errors
/// A class cannot be serialized.
#[inline(never)]
pub fn to_bytes_with_maps(
    classes: &ClassMap<'_>,
    header: &HkxHeader,
    event_ids: EventIdMap,
    variable_ids: VariableIdMap,
) -> Result<Vec<u8>, SerError> {
    serde_hkx::to_bytes_with_maps(classes, header, event_ids, variable_ids)
}

/// Classes -> Havok XML. `top_ptr`: from `HavokSort::sort_for_xml`.
///
/// # Errors
/// A class cannot be serialized.
#[inline(never)]
pub fn to_xml(classes: &ClassMap<'_>, top_ptr: &Pointer<'_>) -> Result<String, SerError> {
    serde_hkx::to_string(classes, top_ptr)
}

/// Converts the file or directory `input` into `format`, written to `output` (or next to the
/// input). Boxed, so the async body is also built here, not in the caller.
#[inline(never)]
pub fn convert(
    input: PathBuf,
    output: Option<PathBuf>,
    format: Format,
) -> Pin<Box<dyn Future<Output = serde_hkx_features::Result<()>> + Send>> {
    Box::pin(async move { serde_hkx_features::convert(&input, output.as_deref(), format).await })
}

/// Reads `path` (any supported format) for [`with_class_map`].
///
/// # Errors
/// The file cannot be read.
pub fn read_file(path: &Path) -> serde_hkx_features::Result<Vec<u8>> {
    std::fs::read(path).map_err(|source| serde_hkx_features::error::Error::FailedReadFile {
        path: path.to_path_buf(),
        source,
    })
}
