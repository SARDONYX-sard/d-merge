//! Widget size variants matching shadcn/ui's Nova size system.

/// Size variants for components like Button and Toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComponentSize {
    /// Extra small: h=24, px=8, text=12
    Xs,
    /// Small: h=28, px=10, text=12.8
    Sm,
    /// Default: h=32, px=10, text=14
    #[default]
    Default,
    /// Large: h=36, px=10, text=14
    Lg,
}

impl ComponentSize {
    /// Returns (height, horizontal_padding, font_size) in logical pixels.
    pub fn metrics(self) -> (f32, f32, f32) {
        match self {
            Self::Xs => (24.0, 8.0, 12.0),
            Self::Sm => (28.0, 10.0, 12.8),
            Self::Default => (32.0, 10.0, 14.0),
            Self::Lg => (36.0, 10.0, 14.0),
        }
    }
}
