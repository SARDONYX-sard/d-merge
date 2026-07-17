//! Typography builder struct — styled text display.

/// Styled text matching shadcn/ui typography variants.
#[must_use]
pub struct Typography {
    pub(crate) text: String,
    pub(crate) variant: crate::tokens::typography_variant::TypographyVariant,
}

impl Typography {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: crate::tokens::typography_variant::TypographyVariant::P,
        }
    }

    pub fn variant(mut self, variant: crate::tokens::typography_variant::TypographyVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn h1(text: impl Into<String>) -> Self {
        Self::new(text).variant(crate::tokens::typography_variant::TypographyVariant::H1)
    }

    pub fn h2(text: impl Into<String>) -> Self {
        Self::new(text).variant(crate::tokens::typography_variant::TypographyVariant::H2)
    }

    pub fn h3(text: impl Into<String>) -> Self {
        Self::new(text).variant(crate::tokens::typography_variant::TypographyVariant::H3)
    }

    pub fn h4(text: impl Into<String>) -> Self {
        Self::new(text).variant(crate::tokens::typography_variant::TypographyVariant::H4)
    }

    pub fn lead(text: impl Into<String>) -> Self {
        Self::new(text).variant(crate::tokens::typography_variant::TypographyVariant::Lead)
    }

    pub fn muted(text: impl Into<String>) -> Self {
        Self::new(text).variant(crate::tokens::typography_variant::TypographyVariant::Muted)
    }

    pub fn small(text: impl Into<String>) -> Self {
        Self::new(text).variant(crate::tokens::typography_variant::TypographyVariant::Small)
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}
