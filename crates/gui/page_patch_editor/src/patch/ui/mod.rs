//! Drawing and key reading of the patch tool. The logic lives next to this directory, in plain
//! modules that do not need an `egui::Ui`, so it can be unit tested.

pub(super) mod anim_data;
pub(super) mod anim_tree;
mod behaviors;
mod object_editor;
mod object_list;
pub(super) mod preview;
mod problems;
pub(super) mod references;
mod search;
mod start_screen;
pub(super) mod template_picker;
mod top_bar;
pub(super) mod tree;
mod user_patch;
mod vars_panel;
pub(super) mod work_check;
