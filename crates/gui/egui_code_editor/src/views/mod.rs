//! Widgets of their own beside the editor, which the host places: the status bar, the Problems
//! panel, the references, a diff, the search results, the breadcrumbs, and the history of where the
//! cursor was.

pub(crate) mod breadcrumbs;
pub(crate) mod diff_view;
pub(crate) mod nav_history;
pub mod problems;
pub(crate) mod references;
pub(crate) mod search_view;
pub(crate) mod status;
