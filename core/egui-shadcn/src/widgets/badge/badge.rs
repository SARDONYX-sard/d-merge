//! Badge builder struct — a small pill label.

/// A badge pill: `h-5 rounded-full px-2 text-xs font-medium`.
#[must_use]
pub struct Badge {
    pub(crate) text: String,
    pub(crate) variant: crate::tokens::badge_variant::BadgeVariant,
}

impl Badge {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: crate::tokens::badge_variant::BadgeVariant::Default,
        }
    }

    pub fn variant(mut self, variant: crate::tokens::badge_variant::BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}
