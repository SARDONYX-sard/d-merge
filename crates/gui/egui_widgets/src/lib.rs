//! Reusable egui widgets used by the D Merge GUI.
//!
//! - [`shadcn`]: shadcn/ui-styled drop-ins for egui widgets (`Button`, `ComboBox`, ...)
//! - [`markdown`]: tiny Markdown renderer for tooltips
//! - [`notify`]: colored one-line notification state
//! - [`EditorTab`]: a tab of an editor, as VS Code draws them; [`preview_tabs`]: the tabs it keeps
//!
//! This crate depends only on `egui` (no app logic), so it stays fast to compile.

pub mod checklist;
pub mod markdown;
pub mod notify;
pub mod preview_tabs;
pub mod shadcn;

mod activity_bar;
mod cut_hover;
mod editor_tab;
mod path_row;
mod window;

pub use self::{
    activity_bar::{ActivityBar, ActivityItem, section_header, side_bar_title},
    cut_hover::{CutHover, Fitted, fit, fit_text, fitted_button},
    editor_tab::{EditorTab, EditorTabOutput, TabAction, TabMenu},
    path_row::{PathActions, PathRow, PathRowOutput},
    window::{Modal, NearPointer, quick_pick_window, show_modal, vertical_splitter, window_near_pointer},
};
