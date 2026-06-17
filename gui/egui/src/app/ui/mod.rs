//! App-owned UI panels.
//!
//! Each sub-module contains one or more `impl App` blocks responsible for
//! rendering a single logical region of the window.  All egui interaction
//! (widget calls, layout) is isolated here; business logic is delegated back
//! to methods defined in the sibling modules (`fetch`, `patch`, `notify`).

pub(crate) mod bottom_panel;
pub(crate) mod mod_list;
pub(crate) mod top_panels;
pub(crate) mod windows;
