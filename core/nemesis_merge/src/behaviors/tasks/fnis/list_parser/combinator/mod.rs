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
pub mod anim_types;
pub mod anim_var;
pub mod comment;
pub mod flags;
pub mod fnis_animation;
pub mod motion;
pub mod rotation;
pub mod version;

use winnow::{token::take_till, ModalResult, Parser as _};

/// take till space, tab
pub fn take_till_space<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    take_till(1.., [' ', '\t']).parse_next(input)
}

/// take till comment start(`'`), space, tab, or line_ending
pub fn take_till_fnis_ignores<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    take_till(1.., [' ', '\t', '\r', '\n', '\'']).parse_next(input)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trigger<'a> {
    /// A new/old event name of `hkbBehaviorGraphStringData.eventNames`. (e.g. `HeadTrackingOn`)
    pub event: &'a str,
    /// e.g. 2.555,
    pub time: f32,
}
