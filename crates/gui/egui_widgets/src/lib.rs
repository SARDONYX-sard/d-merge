//! Reusable egui widgets used by the D Merge GUI.
//!
//! - [`shadcn`]: shadcn/ui-styled drop-ins for egui widgets (`Button`, `ComboBox`, ...)
//! - [`markdown`]: tiny Markdown renderer for tooltips
//! - [`notify`]: colored one-line notification state
//! - [`EditorTab`]: a tab of an editor, as VS Code draws them
//!
//! This crate depends only on `egui` (no app logic), so it stays fast to compile.

pub mod markdown;
pub mod notify;
pub mod shadcn;

mod cut_hover;
mod editor_tab;
mod path_row;
mod window;

pub use self::{
    cut_hover::CutHover,
    editor_tab::{EditorTab, EditorTabOutput},
    path_row::{PathActions, PathRow, PathRowOutput},
    window::{Modal, NearPointer, show_modal, vertical_splitter, window_near_pointer},
};
