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

/// A conditional expression used in `AnimSetData` to determine whether an animation set applies.
///
/// Each `Condition` checks whether a named variable falls within a specific integer range (`value_a..=value_b`).
///
/// These variables are often used to represent weapon types, animation flags, or combat state,
/// and are typically prefixed with:
/// - `i` for integers (e.g., `iLeftHandType`, `iRightHandType`, `iWantMountedWeaponAnims`)
/// - `b` for booleans (e.g., `bIsBlocking`)
///
/// ## Matching Behavior
/// The condition is considered `true` if the value of `variable_name` is within the closed range `value_a..=value_b`.
///
/// ## Common Use Cases
/// - Restrict animations based on equipped weapon types (`HandType`)
/// - Control mounted combat behavior (`MountedAttackPermission`)
/// - Enable or disable specific animation states under complex conditions
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Condition<'a> {
    /// The name of the variable used in the condition.
    ///
    /// Typical examples include:
    /// - `iLeftHandType`
    /// - `iRightHandType`
    /// - `iWantMountedWeaponAnims`
    ///
    /// # Prefix conventions
    /// - `i` prefix: represents an integer-type variable.
    /// - `b` prefix: represents a boolean-type variable.
    ///
    /// These variables are matched against a range defined by `value_a..=value_b`.
    pub variable_name: Str<'a>,

    /// The **start** of the allowed range (inclusive) for the condition value.
    ///
    /// When `variable_name` is:
    /// - `iLeftHandType` or `iRightHandType`, this corresponds to a [`HandType`] variant.
    /// - `iWantMountedWeaponAnims`, this corresponds to a [`MountedAttackPermission`] variant.
    ///
    /// Used together with `value_b` to define a closed range (`value_a..=value_b`).
    pub value_a: i32,

    /// The **end** of the allowed range (inclusive) for the condition value.
    ///
    /// When `variable_name` is:
    /// - `iLeftHandType` or `iRightHandType`, this corresponds to a [`HandType`] variant.
    /// - `iWantMountedWeaponAnims`, this corresponds to a [`MountedAttackPermission`] variant.
    ///
    /// If `value_a == value_b`, the condition checks for a single exact value.
    pub value_b: i32,
}

/// Condition hand type
///
/// When `iLeftHandType` | `iRightHandType`, it's `HandType` range(NOTE: contain end range)
///
/// hand file name rule:
/// - `Solo`: Unarmed
/// -  `1HM`: OneHand Sword
/// -  `MRh`: RightHand Magic
/// -  `MLh`: RightHand Magic
///
/// e.g. `MRhShield.txt`: right hand Magic, left hand shield
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandType {
    /// The conditional clause is named `Solo`.
    #[default]
    Unarmed = 0,
    OneHandDagger = 1,
    OneHandSword = 2,
    OneHandAxe = 3,
    OneHandMace = 4,
    TwoHandSword = 5,
    TwoHandAxe = 6,
    Bow = 7,
    Staff = 8,
    Magic = 9,
    Shield = 10,
    Touch = 11,
    CrossBow = 12,
}

/// Controls whether attacks are allowed while mounted, and on which creature types.
///
/// This setting is used to fine-tune mounted combat behavior in animation sets.
///
/// ## Notes
/// - When set to `Disabled`, attacks while mounted are not allowed — typically used
///   when certain weapon/animation combinations are incompatible with mounted combat.
///
/// - The value can depend on a combination of weapon types and other state:
///   for example, a one-handed sword may allow attacks on horseback,
///   but dual-handing it may disable attacks.
///
/// - This value acts as a range (`value_a`..=`value_b`) for `iWantMountedWeaponAnims`
///   in `Condition` blocks within `AnimSetData`.
///
/// - See also: `iLeftHandType`, `iRightHandType` conditions.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MountedAttackPermission {
    /// Attacks are not allowed while mounted.
    #[default]
    Disabled = 0,

    /// Attacks are allowed while riding a horse.
    Horse = 1,

    /// Attacks are allowed while riding a dragon.
    Dragon = 2,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Attack<'a> {
    /// The trigger name that activates this attack animation.
    pub attack_trigger: Str<'a>,

    /// Indicates whether this attack animation is contextual.(Possibly)
    ///
    /// This flag likely marks attacks whose animation depends on input direction
    /// or player state, causing dynamic branching.
    ///
    /// - When `true`: The attack usually has **no fixed direction** and may branch into multiple clips.
    ///   For example, the `attackStart` trigger often plays `"2HM_AttackLeft"` or `"2HM_AttackRight"` clips.
    /// - When `false`: The attack typically has a **fixed direction** or a single predefined animation clip,
    ///   such as `attackPowerStartForward` playing `"2HM_AttackPowerFwdUncropped"`.
    pub is_contextual: bool,

    /// The number of clip names in this attack.
    ///
    /// This should match the length of `clip_names`.
    pub clip_names_len: usize,

    /// A list of animation clip names associated with this attack.
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
