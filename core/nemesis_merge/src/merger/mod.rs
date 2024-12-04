mod apply_patches;
pub mod behavior_gen;
mod collect_templates_and_patches;
mod save_to_hkx;
pub mod tables;

use dashmap::DashMap;
use simd_json::BorrowedValue;
use std::{collections::HashMap, path::PathBuf};

/// - key: template file stem(e.g. `0_master`)
/// - value: output_path(hkx file path), borrowed json (from template xml)
pub type TemplateMap<'a> = DashMap<String, (PathBuf, BorrowedValue<'a>)>;

/// - key: merge target template file stem (e.g. `0_master`)
/// - value: nemesis patch xml(from hkx file)
pub type PatchMapOwned = HashMap<String, String>;
/// - key: (e.g. `0_master`) template file stem.
/// - key: (e.g. `aaaa`) mod code
/// - value: nemesis patch xml files
pub type PatchModMap = HashMap<String, PatchMapOwned>;

#[derive(Debug)]
pub struct Options {
    pub resource_dir: PathBuf,
    pub output_dir: PathBuf,
}
