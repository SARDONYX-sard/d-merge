//! Top-level pages, shown one at a time by the router (see [`super::router`]).
//!
//! Each page is an `impl App` block with a `ui_*_page(&mut self, ui)` entry point.
//! Page state that must survive navigation lives in [`super::App`] (e.g., [`convert::ConvertPage`]),
//! because the router recreates the route object on every navigation.

pub(crate) mod convert;
pub(crate) mod hkanno;
mod patch;
