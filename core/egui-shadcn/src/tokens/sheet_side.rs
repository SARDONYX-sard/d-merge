//! Sheet slide-in side variants.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SheetSide {
    #[default]
    Right,
    Left,
    Top,
    Bottom,
}
