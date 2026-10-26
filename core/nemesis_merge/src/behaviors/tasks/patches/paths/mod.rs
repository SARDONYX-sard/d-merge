use std::borrow::Cow;

use crate::behaviors::tasks::templates::key::{MasterIndex, TemplateKey};

pub(super) mod collect;
pub(super) mod parse;

/// Parsed Nemesis path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NemesisPath<'a> {
    /// Normal Nemesis template-based patch.
    Normal {
        /// Template name such as `0_master`.
        template_name: &'a str,
        /// Whether `_1stperson` prefix exists.
        is_1st_person: bool,
    },

    /// Nemesis EngineExt patch.
    ///
    /// All paths under `meshes/` are considered valid patch targets.
    ///
    /// Creating without verifying that it starts with `meshes` may result in undefined behavior.
    EngineExt {
        /// Relative path starting from `meshes/` (including `meshes/`).
        meshes_path: &'a str,
    },
}

impl<'a> NemesisPath<'a> {
    /// Attempts to convert this path into a [`TemplateKey`].
    pub(super) fn to_template_key(&self) -> Option<TemplateKey<'static>> {
        match self {
            NemesisPath::Normal { template_name, is_1st_person } => {
                TemplateKey::from_nemesis_file(template_name, *is_1st_person)
            }
            // Safety: Ext verifies that the parsing stage for creation begins with meshes.
            NemesisPath::EngineExt { meshes_path } => {
                TemplateKey::new(Cow::Owned(meshes_path.to_string()))
            }
        }
    }

    /// Returns the indexes (e.g. `#0100`) of `hkbBehaviorGraphStringData` & `hkbBehaviorGraphData` used to replace
    /// Nemesis variables(e.g., `$variableID[variableName]$`).
    ///
    /// This value is only defined for [`NemesisPath::Normal`] paths.
    ///
    /// # EngineExt behavior
    ///
    /// For [`NemesisPath::EngineExt`] paths, this method always returns `None`.
    /// EngineExt patches do not have a fixed index at parse time;
    /// instead, the index must be discovered dynamically by the patcher
    /// during graph traversal.
    ///
    /// This distinction is intentional and reflects the semantic difference
    /// between template-based Nemesis patches and free-form EngineExt patches.
    pub(super) fn get_master_index(&self) -> Option<&'static MasterIndex> {
        match self {
            NemesisPath::Normal { template_name, is_1st_person } => {
                MasterIndex::from_nemesis_file(template_name, *is_1st_person)
            }
            NemesisPath::EngineExt { .. } => None,
        }
    }
}
