//! Drawing and key reading of the patch tool. The logic lives next to this directory, in plain
//! modules that do not need an `egui::Ui`, so it can be unit tested.

mod object_editor;
mod object_list;
pub(super) mod preview;
pub(super) mod references;
pub(super) mod template_picker;
mod top_bar;
pub(super) mod tree;
mod vars_panel;
