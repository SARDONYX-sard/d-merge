//! Typography level variants.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TypographyVariant {
    H1,
    H2,
    H3,
    H4,
    #[default]
    P,
    Lead,
    Large,
    Small,
    Muted,
}

impl TypographyVariant {
    /// Returns (font_size, line_height_factor, is_bold).
    pub fn metrics(self) -> (f32, f32, bool) {
        match self {
            Self::H1 => (36.0, 1.1, true),
            Self::H2 => (30.0, 1.2, true),
            Self::H3 => (24.0, 1.3, true),
            Self::H4 => (20.0, 1.4, true),
            Self::P => (14.0, 1.5, false),
            Self::Lead => (20.0, 1.5, false),
            Self::Large => (18.0, 1.5, true),
            Self::Small => (13.0, 1.5, false),
            Self::Muted => (14.0, 1.5, false),
        }
    }
}
