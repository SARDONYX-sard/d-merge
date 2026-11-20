//! Ready-made [`crate::Language`]s for common formats (no LSP features, highlighting only).

mod xml;
mod xml_path;

pub use self::{
    xml::{Palette, Xml},
    xml_path::xml_path,
};
