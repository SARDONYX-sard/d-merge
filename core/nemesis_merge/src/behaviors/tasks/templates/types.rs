use std::{collections::HashMap, path::PathBuf};

use dashmap::DashMap;
use simd_json::BorrowedValue;

/// Name of the template that needs to be read.
///
/// - format: template_name, is_1st_person
/// - e.g. (`0_master`, false)
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemplateKey<'a> {
    pub template_name: &'a str,
    pub is_1st_person: bool,
}

impl<'a> TemplateKey<'a> {
    #[inline]
    pub const fn new(template_name: &'a str, is_1st_person: bool) -> Self {
        Self {
            template_name,
            is_1st_person,
        }
    }
}

impl core::fmt::Display for TemplateKey<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_1st_person {
            write!(f, "_1stperson/{}", self.template_name)
        } else {
            write!(f, "{}", self.template_name)
        }
    }
}

/// - key: template file path
/// - value: Content bytes
pub type OwnedTemplateMap = HashMap<PathBuf, Vec<u8>>;

/// - key: template file stem(e.g. `0_master`)
/// - value: output_path(hkx file path), borrowed json (from template xml)
pub type BorrowedTemplateMap<'a> = DashMap<TemplateKey<'a>, (&'a str, BorrowedValue<'a>)>;
