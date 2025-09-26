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
pub mod fnis_animation;
pub mod motion;
pub mod rotation;
pub mod version;
