//! Reusable egui widgets used by the D Merge GUI.
//!
//! - [`shadcn`]: shadcn/ui-styled drop-ins for egui widgets (`Button`, `ComboBox`, ...)
//! - [`markdown`]: tiny Markdown renderer for tooltips
//! - [`notify`]: colored one-line notification state
//!
//! This crate depends only on `egui` (no app logic), so it stays fast to compile.

pub mod markdown;
pub mod notify;
pub mod shadcn;

mod path_row;
mod window;

pub use self::{
    path_row::{PathActions, PathRow, PathRowOutput},
    window::window_near_pointer,
};
