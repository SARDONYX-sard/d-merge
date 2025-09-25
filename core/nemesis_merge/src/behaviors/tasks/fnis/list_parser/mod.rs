//! `FNIS_<mod name>_List.txt` parser
//!
//! See FNIS for Modders pdf
//!
//!
//! 5. Syntax of AnimLists(From `FNIS for Modders_V6.2.pdf` by fore)
//!
//! The following items are each syntax specifications written on a single line. <> indicates a variable that can be a number or a string.
//!
//! ```txt
//! -        FNIS Animation: <AnimType> [-<option,option,...>] <AnimEvent> <AnimFile> [<AnimObject> ...]
//! -     Behavior Variable: AnimVar <AnimVar> [ BOOL | INT32 | REAL ] <numeric_value>
//! -           Motion Data: MD <time> <delta_x> <delta_y> <delta_z>
//! - Rotation Data Format1: RD <time> <quat_1> <quat_2> <quat_3> <quat_4>
//! - Rotation Data Format2: RD <time> <delta_z_angle>
//! -        Version of mod: Version V<n>.<m>
//! -  Alternate Animations: AAprefix <3_character_mod_abbreviation>
//! -  Alternate Animations: AAset <animation_group> <number>
//! -  Alternate Animations: T <alternate_animation> <trigger1> <time1> <trigger2> <time2> ..
//! ```
pub mod alt_anim;
pub mod anim_types;
pub mod anim_var;
pub mod comments;
pub mod flags;
pub mod motion;
pub mod rotation;
pub mod version;

/// Common test helpers
#[cfg(test)]
pub mod test_helpers {
    use winnow::error::{ContextError, ErrMode};
    use winnow::Parser;

    /// Must successfully parse or panic
    pub fn must_parse<'a, O>(
        mut parser: impl Parser<&'a str, O, ErrMode<ContextError>>,
        input: &'a str,
    ) -> O {
        parser
            .parse(input)
            .unwrap_or_else(|e| panic!("ERROR:\n{e}"))
    }

    /// Must fail to parse or panic
    pub fn must_fail<'a, O>(
        mut parser: impl Parser<&'a str, O, ErrMode<ContextError>>,
        input: &'a str,
    ) {
        if parser.parse(input).is_ok() {
            panic!("[Must fail!] expected parse to fail, but got OK");
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use winnow::error::{ContextError, ErrMode};

//     #[test]
//     fn test_parse_entry_valid() {
//         let input = r#"
// s -h,ac0 IdleStart IdleStart.hkx
// MD 1.0 0 0 0
// RD 1.0 0
// "#;
//         let parsed = must_parse(parse_entry, input);
//         assert_eq!(
//             parsed.kind,
//             FNISAnimKind::new(
//                 FNISAnimType::Sequenced,
//                 FNISAnimFlags::AnimatedCameraReset | FNISAnimFlags::HeadTracking
//             )
//         );
//         assert_eq!(parsed.event, "IdleStart");
//         assert_eq!(parsed.file, "IdleStart.hkx");
//     }

//     #[test]
//     fn test_parse_entry_missing_file() {
//         let input = r#"
// s IdleStart
// MD 1.0 0 0 0
// RD 1.0 0
// "#;
//         must_fail(parse_entry, input);
//     }

//     #[test]
//     fn test_parse_anim_list_multiple_entries() {
//         let input = r#"
// Version 1.0

// s IdleStart IdleStart.hkx
// MD 1.0 0 0 0
// RD 1.0 0

// fu -h,ac0 SitDown SitDown.hkx
// MD 2.0 0 -10 0
// RD 2.0 0 0 0 1
// "#;

//         let parsed = must_parse(parse_anim_list, input);
//         assert_eq!(parsed.version.major, 1);
//         assert_eq!(parsed.entries.len(), 2);
//         assert_eq!(
//             parsed.entries[0],
//             Entry {
//                 kind: FNISAnimKind::new(FNISAnimType::Sequenced, FNISAnimFlags::empty()),
//                 event: "IdleStart",
//                 file: "IdleStart.hkx",
//                 anim_objects: vec![],
//                 md: MotionData {
//                     time: 1.0,
//                     delta_x: 0,
//                     delta_y: 0,
//                     delta_z: 0,
//                 },
//                 rd: RotationData::Format2(RotationData2 {
//                     time: 1.0,
//                     delta_z_angle: 0,
//                 }),
//             }
//         );
//         assert_eq!(
//             parsed.entries[1],
//             Entry {
//                 kind: FNISAnimKind::new(
//                     FNISAnimType::Furniture,
//                     FNISAnimFlags::HeadTracking | FNISAnimFlags::AnimatedCameraReset
//                 ),
//                 event: "SitDown",
//                 file: "SitDown.hkx",
//                 anim_objects: vec![],
//                 md: MotionData {
//                     time: 2.0,
//                     delta_x: 0,
//                     delta_y: -10,
//                     delta_z: 0,
//                 },
//                 rd: RotationData::Format1(RotationData1 {
//                     time: 2.0,
//                     quat_1: 0,
//                     quat_2: 0,
//                     quat_3: 0,
//                     quat_4: 1,
//                 }),
//             }
//         );
//     }

//     #[test]
//     #[ignore = "local test"]
//     fn test_parse_real_file() {
//         let input = std::fs::read_to_string(
//             "../../dummy/fnis_test_mods/FNIS Flyer SE 7.0/Data/Meshes/actors/character/animations/FNISFlyer/FNIS_FNISFLyer_List.txt"
//         ).unwrap();
//         let parsed = must_parse(parse_anim_list, &input);
//         println!("{:#?}", parsed);
//     }
// }
