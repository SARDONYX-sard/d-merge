//! A node graph view for egui, as a blueprint: the neighborhood of one node, in layers from left to
//! right, with its fields on the nodes and pins to connect.
//!
//! ```text
//! layer:   -1              0 (focus)                  1
//!       ┌────────┐     ┌──────────────────┐      ┌─────────┐
//!       │ Parent ●──> ○│▓Root▓▓▓▓▓▓▓▓▓▓▓▓▓│  ┌─>○│▓Idle ×2▓│  ▓ the header's color, ×2 shared
//!       └────────┘     │ name      Root   │  │   └─────────┘
//!                      │ states 1-8 / 900 ‹ ›│  │                a long list, a page at a time
//!                      │ states[0]       ●───┘
//!                      │ states    +     ●   <- drop a node here: one more
//!                      └──────────────────┘
//! ```
//!
//! The host keeps its graph (what a node is, is its own) and gives this crate only what the view
//! needs, through [`GraphSource`]: the edges, parents and [`Row`]s of a node, and how it looks. What
//! the user changes comes back as a [`GraphEdit`], for the host to carry out.
//!
//! - [`layout`]: which nodes are shown and where. A pure function of the source, computed again
//!   only when the focus or [`GraphSource::version`] changes.
//! - [`view`]: the state on screen (the layout, pan, zoom, moved nodes, the value being typed) and
//!   the drawing. Only what is on screen is drawn.

pub mod layout;
pub mod view;

pub use layout::{Layout, LayoutOptions};
pub use view::{GraphLabels, GraphOutput, GraphView};

/// An edge from a node, as [`GraphSource::edges`] gives it.
///
/// e.g. `Edge { to: 12, label: "states", key: 418 }`: the pointer at byte 418 of a text, in field
/// `states`, points to node 12.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge<'a, N> {
    pub to: N,
    /// Drawn on the edge when no [`Row`] has its `key`. Empty: none.
    pub label: &'a str,
    /// The host's own number for the edge. A [`Row`] with the same key is where it leaves from.
    pub key: usize,
}

/// How a node looks.
///
/// e.g. `Card { title: "Idle", subtitle: "#0097 hkbClipGenerator", header: Some(BLUE), .. }`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Card<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    /// The band behind the title, e.g. by the kind of node (as a blueprint's function / event).
    pub header: Option<egui::Color32>,
    /// The bar on the left, e.g. for a new or an edited node.
    pub accent: Option<egui::Color32>,
    /// Drawn faintly, e.g. a node that only an edge mentions.
    pub dimmed: bool,
    /// Its values can be typed and its pins connected.
    pub editable: bool,
    /// How many nodes point to it in the whole graph. More than one: `×N` on its title, a shared
    /// node.
    pub fan_in: usize,
}

/// One line of a node: a field.
///
/// e.g. `Row { label: "startStateId", index: None, slot: Slot::Value("0"), color: None, key: 120 }`
///
/// Rows of one label with an `index` one after another are a list: a long one is shown a page at a
/// time (see [`GraphView::list_page`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row<'a> {
    pub label: &'a str,
    /// Of an element of a list: drawn as `label[index]`.
    pub index: Option<usize>,
    pub slot: Slot<'a>,
    /// Of its value or pin, and of the edge leaving it: e.g. by type, as a blueprint's pins.
    pub color: Option<egui::Color32>,
    /// The host's own number for the row, given back in a [`GraphEdit`] or a click. An [`Edge`]
    /// with the same key leaves from its pin.
    pub key: usize,
}

/// What a row holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot<'a> {
    /// A value typed as text: a click edits it.
    Value(&'a str),
    /// A value shown only, e.g. `[3]` for a list edited elsewhere.
    Fixed(&'a str),
    /// A pointer: a pin on the right, dragged onto a node to connect it, or off to disconnect it.
    Pin { connected: bool },
    /// The end of a list of pointers: a node dropped on its pin is added to the list.
    AddPin,
}

/// What the user changed: the host carries it out (and the source shows it next frame).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphEdit<N> {
    /// The value of row `key` of `node` was typed.
    Value { node: N, key: usize, text: String },
    /// The pin of row `key` of `node` was dropped on node `to`.
    Connect { node: N, key: usize, to: N },
    /// The pin of row `key` of `node` was dropped where no node is.
    Disconnect { node: N, key: usize },
}

/// The host's graph, as the view reads it.
pub trait GraphSource {
    /// A node: a number, a handle. Copied freely.
    type Node: Copy + Eq + std::hash::Hash + std::fmt::Debug;

    /// Changes whenever an edge, a row or a card does: the layout is computed again then.
    fn version(&self) -> u64;

    /// The edges from `node`, in the order they are drawn.
    fn edges(&self, node: Self::Node) -> impl Iterator<Item = Edge<'_, Self::Node>>;

    /// The nodes with an edge to `node`.
    fn parents(&self, node: Self::Node) -> impl Iterator<Item = Self::Node>;

    fn card(&self, node: Self::Node) -> Card<'_>;

    /// The lines of `node` under its title. None by default: a node is its card only.
    fn rows(&self, node: Self::Node) -> impl Iterator<Item = Row<'_>> {
        let _ = node;
        std::iter::empty()
    }

    /// Can the pin of row `key` of `node` point to `to`? While a pin is dragged, the nodes that
    /// cannot take it are drawn faintly, and dropping it there does nothing.
    fn accepts(&self, node: Self::Node, key: usize, to: Self::Node) -> bool {
        let _ = (node, key, to);
        true
    }
}
