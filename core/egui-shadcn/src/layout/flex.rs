//! Ergonomic flexbox layout builder wrapping `egui_flex::Flex`.

/// A flexbox layout container with CSS-familiar naming.
///
/// Thin wrapper over [`egui_flex::Flex`] that provides shorthand builder
/// methods (`row`, `column`, `gap`, `align_center`, `justify_end`, etc.)
/// so layouts read like CSS flexbox.
///
/// ```no_run
/// # egui::__run_test_ui(|ui| {
/// egui_shadcn::Flex::row().gap(8.0).show(ui, |f| {
///     f.add(egui::Button::new("Cancel"));
///     f.add(egui::Button::new("Save"));
/// });
/// # });
/// ```
#[must_use]
pub struct Flex(pub(crate) egui_flex::Flex);

impl Flex {
    /// Horizontal (row) layout — items flow left to right.
    pub fn row() -> Self {
        Self(egui_flex::Flex::horizontal())
    }

    /// Vertical (column) layout — items flow top to bottom.
    pub fn column() -> Self {
        Self(egui_flex::Flex::vertical())
    }

    /// Space between items on both axes (uniform gap).
    pub fn gap(self, gap: f32) -> Self {
        Self(self.0.gap(egui::vec2(gap, gap)))
    }

    /// Enable wrapping when items exceed the container width.
    pub fn wrap(self) -> Self {
        Self(self.0.wrap(true))
    }

    /// Cross-axis: align items to start.
    pub fn align_start(self) -> Self {
        Self(self.0.align_items(egui_flex::FlexAlign::Start))
    }

    /// Cross-axis: center items.
    pub fn align_center(self) -> Self {
        Self(self.0.align_items(egui_flex::FlexAlign::Center))
    }

    /// Cross-axis: align items to end.
    pub fn align_end(self) -> Self {
        Self(self.0.align_items(egui_flex::FlexAlign::End))
    }

    /// Cross-axis: stretch items to fill.
    pub fn align_stretch(self) -> Self {
        Self(self.0.align_items(egui_flex::FlexAlign::Stretch))
    }

    /// Main-axis: pack items to the start.
    pub fn justify_start(self) -> Self {
        Self(self.0.justify(egui_flex::FlexJustify::Start))
    }

    /// Main-axis: center items.
    pub fn justify_center(self) -> Self {
        Self(self.0.justify(egui_flex::FlexJustify::Center))
    }

    /// Main-axis: pack items to the end.
    pub fn justify_end(self) -> Self {
        Self(self.0.justify(egui_flex::FlexJustify::End))
    }

    /// Main-axis: distribute with equal space between items.
    pub fn justify_between(self) -> Self {
        Self(self.0.justify(egui_flex::FlexJustify::SpaceBetween))
    }

    /// Main-axis: distribute with equal space around items.
    pub fn justify_around(self) -> Self {
        Self(self.0.justify(egui_flex::FlexJustify::SpaceAround))
    }

    /// Main-axis: distribute with equal space between and around items.
    pub fn justify_evenly(self) -> Self {
        Self(self.0.justify(egui_flex::FlexJustify::SpaceEvenly))
    }

    /// Fill all available width.
    pub fn w_full(self) -> Self {
        Self(self.0.w_full())
    }

    /// Fill all available height.
    pub fn h_full(self) -> Self {
        Self(self.0.h_full())
    }

    /// Explicit width in points.
    pub fn width(self, width: f32) -> Self {
        Self(self.0.width(width))
    }

    /// Explicit height in points.
    pub fn height(self, height: f32) -> Self {
        Self(self.0.height(height))
    }

    /// Default grow factor for all children.
    pub fn grow_items(self, grow: f32) -> Self {
        Self(self.0.grow_items(grow))
    }

    /// Display this flex container and populate it via the closure.
    pub fn show<R>(
        self,
        ui: &mut egui::Ui,
        f: impl FnOnce(&mut super::flex_instance::FlexInst) -> R,
    ) -> egui::InnerResponse<R> {
        self.0.show(ui, |instance| {
            let mut inst = super::flex_instance::FlexInst(instance);
            f(&mut inst)
        })
    }
}
