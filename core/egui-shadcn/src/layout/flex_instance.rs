//! Ergonomic wrapper over `egui_flex::FlexInstance` for adding flex children.

/// Handle for adding children inside a [`super::flex::Flex`] layout.
///
/// Wraps [`egui_flex::FlexInstance`] to hide `FlexItem` boilerplate for
/// common cases while still exposing full control via `item()` / `item_ui()`.
pub struct FlexInst<'ui, 'inst>(pub(crate) &'inst mut egui_flex::FlexInstance<'ui>);

impl<'ui, 'inst> FlexInst<'ui, 'inst> {
    /// Add a widget at its natural size.
    pub fn add(
        &mut self,
        widget: impl egui::Widget,
    ) -> egui::InnerResponse<egui::Response> {
        self.0.add_widget(egui_flex::FlexItem::new(), widget)
    }

    /// Add a widget that grows to fill available space.
    pub fn grow(
        &mut self,
        factor: f32,
        widget: impl egui::Widget,
    ) -> egui::InnerResponse<egui::Response> {
        self.0
            .add_widget(egui_flex::FlexItem::new().grow(factor), widget)
    }

    /// Add arbitrary UI content at natural size.
    pub fn ui<R>(
        &mut self,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R> {
        self.0.add_ui(egui_flex::FlexItem::new(), content)
    }

    /// Add arbitrary UI content that grows to fill available space.
    pub fn grow_ui<R>(
        &mut self,
        factor: f32,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R> {
        self.0
            .add_ui(egui_flex::FlexItem::new().grow(factor), content)
    }

    /// Insert an empty spacer that grows with factor 1.0.
    pub fn spacer(&mut self) -> egui::Response {
        self.0.grow()
    }

    /// Add a nested flex container at natural size.
    pub fn nested<R>(
        &mut self,
        flex: super::flex::Flex,
        content: impl FnOnce(&mut FlexInst) -> R,
    ) -> egui::InnerResponse<R> {
        self.0
            .add_flex(egui_flex::FlexItem::new(), flex.0, |inner| {
                let mut inst = FlexInst(inner);
                content(&mut inst)
            })
    }

    /// Add a nested flex container that grows to fill available space.
    pub fn grow_nested<R>(
        &mut self,
        factor: f32,
        flex: super::flex::Flex,
        content: impl FnOnce(&mut FlexInst) -> R,
    ) -> egui::InnerResponse<R> {
        self.0.add_flex(
            egui_flex::FlexItem::new().grow(factor),
            flex.0,
            |inner| {
                let mut inst = FlexInst(inner);
                content(&mut inst)
            },
        )
    }

    /// Add a widget with full [`egui_flex::FlexItem`] control.
    pub fn item(
        &mut self,
        item: egui_flex::FlexItem,
        widget: impl egui::Widget,
    ) -> egui::InnerResponse<egui::Response> {
        self.0.add_widget(item, widget)
    }

    /// Add arbitrary UI content with full [`egui_flex::FlexItem`] control.
    pub fn item_ui<R>(
        &mut self,
        item: egui_flex::FlexItem,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R> {
        self.0.add_ui(item, content)
    }
}
