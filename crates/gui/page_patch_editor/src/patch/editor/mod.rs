//! What the object editor needs, apart from drawing it: its language, its tabs and what it shows
//! beside the text. The drawing is in [`crate::patch::ui`].

pub(in crate::patch) mod diagnostics;
pub(in crate::patch) mod inline_diff;
pub(in crate::patch) mod language;
pub(in crate::patch) mod names;
pub(in crate::patch) mod navigate;
pub(in crate::patch) mod preview;
pub(in crate::patch) mod resolve;
pub(in crate::patch) mod tabs;
