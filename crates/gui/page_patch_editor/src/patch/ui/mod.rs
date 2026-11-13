//! Drawing and key reading of the patch tool. The logic lives next to this directory, in plain
//! modules that do not need an `egui::Ui`, so it can be unit tested.

pub(super) mod anim_data;
pub(super) mod anim_tree;
mod behaviors;
mod changes_view;
pub(super) mod code_choice;
mod object_editor;
mod object_list;
pub(super) mod preview;
pub(super) mod problems;
pub(super) mod repeat;
pub(super) mod revision_view;
mod search;
pub(super) mod side_bar;
mod start_screen;
mod tab_bar;
pub(super) mod template_picker;
mod top_bar;
pub(super) mod tree;
mod user_patch;
mod vars_panel;
