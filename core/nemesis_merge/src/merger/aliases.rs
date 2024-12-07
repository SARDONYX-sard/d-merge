use dashmap::DashMap;
use simd_json::BorrowedValue;
use std::{collections::HashMap, path::PathBuf};

/// - key: template file stem(e.g. `0_master`)
/// - value: output_path(hkx file path), borrowed json (from template xml)
pub type BorrowedTemplateMap<'a> = DashMap<String, (PathBuf, BorrowedValue<'a>)>;

/// - key: merge target template file stem (e.g. `0_master`)
/// - value: nemesis patch xml(from hkx file)
pub type OwnedPatchMap = HashMap<String, String>;
/// - key: (e.g. `0_master`) template file stem.
/// - value: (mod code: (e.g. `aaaa`), nemesis patch xml files)
pub type ModPatchMap = HashMap<String, OwnedPatchMap>;
pub type ModPatchPair = (String, OwnedPatchMap);
