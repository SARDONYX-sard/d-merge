//! # hkanno_lsp
//!
//! Editor language features for the `hkanno` text format
//! (the human-editable form of Havok animation annotations, see `serde_hkx_for_gui::hkanno`).
//!
//! This crate is **GUI-agnostic**: it only takes text (+ a cursor) and returns plain data.
//! The egui editor (`gui/egui`) turns that data into colors, popups and underlines.
//!
//! It is a Rust port of the former Monaco (TypeScript) implementation used by the Tauri GUI.
//!
//! # Coordinates
//! Every position is **0-based** and counted in **`char`s**, not bytes.
//! e.g., in `"0.5 ␀"` the `␀` (U+2400) is column 4 even though it is 3 bytes long.
//! This matches egui's `CCursor`, so the GUI can use the values without conversion.
//!
//! # Layout
//! - [`parser`]: one-line parser producing [`parser::LineNode`] (all language features start here)
//! - [`features`]: diagnostics / completion / hover / signature help / semantic tokens / format / inlay hints / code actions
//! - [`pie`]: Payload Interpreter (`PIE.@...`) instruction table shared by several features
//! - [`snippet`]: `${1:placeholder}` snippet expansion used by completion items

// Snippets use `${1:placeholder}` which clippy mistakes for `format!` arguments.
#![allow(clippy::literal_string_with_formatting_args)]

pub mod features;
pub mod parser;
pub mod pie;
pub mod snippet;
mod span;

pub use self::span::{LineSpan, Span};

/// Splits a document into lines the same way every feature does.
///
/// `\r` is stripped so that CRLF documents produce the same columns as LF ones.
///
/// # Example
/// ```
/// let lines: Vec<&str> = hkanno_lsp::lines("a\r\nb").collect();
/// assert_eq!(lines, ["a", "b"]);
/// ```
pub fn lines(text: &str) -> impl Iterator<Item = &str> {
    text.split('\n').map(|line| line.strip_suffix('\r').unwrap_or(line))
}
