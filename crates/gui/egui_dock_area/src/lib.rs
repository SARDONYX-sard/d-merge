//! Groups of tabs to dock for egui, as VS Code's editor groups.
//!
//! ```text
//! ┌──────────────────────┬──────────────┐
//! │ [Root] [Idle]        │ [Preview]    │   drag a tab: onto another group's tabs (joins it),
//! │                      │              │   onto an edge of a group (a group beside it there),
//! │                      │              │   out of the window (a window of its own),
//! └──────────────────────┴──────────────┘   and from there back in
//! ```
//!
//! - [`node`]: a layout of groups, split side by side or one above the other
//! - [`dock`]: the host's area and the windows groups went out to
//! - [`DockArea`]: draws them, and the drag of a tab between groups
//!
//! A group is the host's: it draws it all, its own tab bar too (e.g. as VS Code's), telling its
//! tabs through [`TabDrags`]. A drop comes back as [`Dropped`]: the host moves its tab, and makes a
//! group for it if it goes beside one or out ([`Dock::put_beside`], [`Dock::open_window`]).
//!
//! A window of its own is an immediate viewport: drawn in the host's frame, with its borrows, so a
//! group needs not be `Send`.

pub mod dock;
pub mod node;
mod view;

pub use self::{
    dock::{Area, Dock, Place, Target, Window},
    node::{Axis, Half, Node, Path, Split},
    view::{DockArea, DockOutput, Dropped, TabDrags},
};

/// What the host's groups are, and how they are drawn.
pub trait GroupViewer {
    /// A group: e.g. an editor and its tabs.
    type Group;
    /// What a dragged tab takes along: e.g. which tab it is.
    type Item;

    /// Draws `group` in all the room of `ui`, its tab bar too: each tab told to `tabs`
    /// ([`TabDrags::tab`]), and the bar ([`TabDrags::bar`]).
    fn ui(&mut self, ui: &mut egui::Ui, group: &mut Self::Group, tabs: &mut TabDrags<'_, Self::Item>);

    /// The title of a window `group` is the first of: e.g. its shown tab's.
    fn title(&mut self, group: &Self::Group) -> String;
}
