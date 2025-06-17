//! Animation data from asdsf(animationsetdatasinglefile.txt)
//!
//! This module provides structures and parsers for reading animation data
//! from a file formatted in a specific way. The primary structure is [`Asdsf`],
//! which contains a list of projects and their corresponding animation data.
mod alt_key;
pub mod de;
pub mod ser;

use indexmap::IndexMap;

use crate::lines::Str;

/// Represents the entire animation data structure.
///
/// Before merging the `animationsetdatasinglefile.txt` file, it exists in `meshes/animationsetdata` in Animation.bsa.
///
/// However, please note that there are no txt references such as Vampire in the `animationsetdatasinglefile.txt` file.
#[derive(Debug, Default, Clone)]
pub struct Asdsf<'a> {
    /// A list of project names parsed from the input.
    pub txt_projects: TxtProjects<'a>,
}

/// - key: project data file names: e.g. `ChickenProjectData\\ChickenProject.txt`
#[derive(Debug, Default, Clone)]
pub struct TxtProjects<'a>(IndexMap<Str<'a>, AnimSetList<'a>>);

/// A list of animation data corresponding to each project.
/// - key: file_name(e.g. `full_body.txt`)
#[derive(Debug, Default, Clone)]
pub struct AnimSetList<'a>(IndexMap<Str<'a>, AnimSetData<'a>>);

/// Represents individual animation data.
///
/// This structure holds the header information for the animation and the
/// associated clip animation and motion blocks.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AnimSetData<'a> {
    /// always `V3`
    pub version: Str<'a>,
    pub triggers_len: usize,
    pub triggers: Vec<Str<'a>>,
    pub conditions_len: usize,
    pub conditions: Vec<Condition<'a>>,
    pub attacks_len: usize,
    pub attacks: Vec<Attack<'a>>,
    pub anim_infos_len: usize,
    pub anim_infos: Vec<AnimInfo>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Condition<'a> {
    pub variable_name: Str<'a>,
    pub value_a: i32,
    pub value_b: i32,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Attack<'a> {
    pub attack_trigger: Str<'a>,
    pub unknown: bool,
    pub clip_names_len: usize,
    pub clip_names: Vec<Str<'a>>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AnimInfo {
    /// CRC32 representation path
    pub hashed_path: u32,
    /// CRC32 representation file name
    pub hashed_file_name: u32,
    /// u32 (le_bytes ASCII) representation extension
    ///
    /// Always `7891816`
    /// ```
    /// assert_eq!(core::str::from_utf8(&u32::to_le_bytes(7891816)), Ok("hkx\0"));
    /// assert_eq!(core::str::from_utf8(&[0x78, 0x6b, 0x68]), Ok("xkh"));
    /// ```
    pub ascii_extension: u32,
}
