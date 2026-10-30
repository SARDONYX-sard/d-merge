//! `animationsetdatasinglefile.txt` patch types (shared by Nemesis and FNIS patch generation).

use std::borrow::Cow;

use skyrim_anim_data::{asdsf::patch::de::DiffPatchAnimSetData, diff_line::DiffLines};
#[derive(serde::Serialize, Debug, Default, Clone, PartialEq)]
pub struct AsdsfPatch<'a> {
    /// e.g. `DefaultMaleData~DefaultMale`
    pub target: &'a str,
    /// e.g. `/some/Nemesis_Engine/mod/slide`
    pub id: &'a str,
    pub patch: PatchKind<'a>,
}

#[derive(serde::Serialize, Debug, Clone, PartialEq)]
pub enum PatchKind<'a> {
    /// Indicates the special `$header$/$header$.txt`override
    TxtProjectHeader(DiffLines<'a>),

    /// Indicates the special `<target>/$header$.txt`override
    SubTxtHeader(DiffLines<'a>),

    /// diff patch, priority
    EditAnimSet(Box<EditAnimSet<'a>>),

    /// add patch
    AddAnimSet {
        patch: skyrim_anim_data::asdsf::normal::AnimSetData<'a>,
        /// mod list priority
        priority: usize,
        /// Add: AnimSetData file name (e.g., `_MTSolo.txt`)
        file_name: Cow<'a, str>,
    },
}

#[derive(serde::Serialize, Debug, Default, Clone, PartialEq)]
pub struct EditAnimSet<'a> {
    pub patch: DiffPatchAnimSetData<'a>,
    /// apply ordering
    pub priority: usize,
    /// file name of each txt project data
    /// - e.g. `_MTSolo.txt`
    pub file_name: &'a str,
}

impl<'a> Default for PatchKind<'a> {
    #[inline]
    fn default() -> Self {
        Self::EditAnimSet(Box::new(EditAnimSet::default()))
    }
}
