use std::{collections::HashMap, path::PathBuf};

use dashmap::DashMap;
use simd_json::BorrowedValue;

use crate::behaviors::tasks::templates::key::TemplateKey;

/// - key: template file path
/// - value: Content bytes
pub type OwnedTemplateMap = HashMap<PathBuf, Vec<u8>>;

/// - key: template file stem(e.g. `0_master`)
/// - value: output_path(hkx file path), borrowed json (from template xml)
pub type BorrowedTemplateMap<'a> = DashMap<TemplateKey<'a>, BorrowedValue<'a>>;
