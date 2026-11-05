//! Drawing and key reading. The logic lives in plain modules that do not need an `egui::Ui`, so it
//! can be unit tested.

pub(crate) mod completion;
pub(crate) mod context_menu;
pub(crate) mod find;
pub(crate) mod hover;
pub(crate) mod overlays;
pub(crate) mod palette;
pub(crate) mod quick_fix;
