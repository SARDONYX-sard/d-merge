//! App-owned UI panels.
//!
//! Each sub-module contains one or more `impl App` blocks responsible for
//! rendering a single logical region of the window.  All egui interaction
//! (widget calls, layout) is isolated here; business logic is delegated back
//! to methods defined in the sibling modules (`fetch`, `patch`, `notify`).
//!
//! # Panel render order
//! egui requires that [`egui::TopBottomPanel`]s are added before any
//! [`egui::CentralPanel`].  Within top/bottom panels, the first registered
//! becomes the outermost (visually frontmost).  [`eframe::App::update`] in
//! `app/mod.rs` calls these in the order below:
//!
//! ```text
//! update()
//!  ├─ top_panels::ui_execution_mode   (topmost)
//!  ├─ top_panels::ui_skyrim_dir
//!  ├─ top_panels::ui_output_dir
//!  ├─ top_panels::ui_search_panel
//!  ├─ bottom_panel::ui_notification   (bottommost of bottoms)
//!  ├─ bottom_panel::ui_bottom_panel
//!  ├─ mod_list::ui_mod_list           (CentralPanel — must be last)
//!  └─ windows::ui_log_window
//!     windows::ui_help_window
//!     windows::ui_show_confirm
//! ```

pub(crate) mod bottom_panel;
pub(crate) mod mod_list;
pub(crate) mod top_panels;
pub(crate) mod windows;
