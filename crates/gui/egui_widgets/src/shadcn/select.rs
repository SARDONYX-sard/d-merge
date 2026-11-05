// SPDX-FileCopyrightText: (C) 2026 Pawel Jankiewicz, SARDONYX
// SPDX-License-Identifier: MIT
// ref:
// - https://github.com/pjankiewicz/egui-shadcn/tree/fa5ceeed623eea983765fa4f886dd610d8b39470/src/widgets/select
// - https://github.com/pjankiewicz/egui-shadcn/tree/fa5ceeed623eea983765fa4f886dd610d8b39470/src/widgets/combobox/combobox_show.rs
//! shadcn-styled drop-in for [`egui::ComboBox`].

use super::{
    button::Button,
    icon::{Icon, paint_icon},
    text::TextEdit,
    theme::{ShadcnTheme, interpolate_color, paint_focus_ring, popup_shadow, with_alpha},
};

/// What the button has beside its text: the padding, the chevron.
const TRIGGER_CHROME: f32 = 36.0;
/// What a menu row has beside its text: the padding, the check mark, a scroll bar.
const ITEM_CHROME: f32 = 40.0;
/// The button is at least this wide, however short its text.
const MIN_TRIGGER_WIDTH: f32 = 72.0;
/// The button is at most this wide unless [`ComboBox::width`] says otherwise: longer text is clipped.
const MAX_TRIGGER_WIDTH: f32 = 320.0;
/// The menu shows long items whole up to this width, however narrow the button.
const MAX_MENU_WIDTH: f32 = 480.0;
/// The height of the button.
const TRIGGER_HEIGHT: f32 = 32.0;

/// [`egui::UiStackInfo`] tag telling [`Button::selectable`] to render as a menu item.
const ITEM_STYLE_TAG: &str = "shadcn::combo_item";

/// How [`Button::selectable`] renders inside a [`ComboBox`] popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ItemStyle {
    /// Check mark on the right, highlighted on hover.
    Select,
    /// Check mark on the left, highlighted on hover or when selected.
    Search,
}

impl ItemStyle {
    /// The style of the nearest enclosing [`ComboBox`] popup, if any.
    pub(super) fn find(ui: &egui::Ui) -> Option<Self> {
        ui.stack().iter().find_map(|frame| frame.tags().get_downcast::<Self>(ITEM_STYLE_TAG)).copied()
    }

    /// Runs `add_contents` in a child ui whose [`Button::selectable`]s render in this style.
    fn scope<R>(self, ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
        let info = egui::UiStackInfo::default().with_tag_value(ITEM_STYLE_TAG, self);
        ui.scope_builder(egui::UiBuilder::new().ui_stack_info(info), add_contents).inner
    }
}

/// A drop-down menu, like [`egui::ComboBox`].
///
/// Inside [`Self::show_ui`], use `ui.add(Button::selectable(selected, text))` for items
/// (the egui 0.33 replacement of `selectable_label`); it renders as a menu item there.
///
/// Extensions (not in egui): [`Self::placeholder`], [`Self::search_hint`],
/// [`Self::no_results_text`], [`Self::show_creatable`].
#[derive(Debug)]
#[must_use = "You should call .show*"]
pub struct ComboBox {
    id_salt: egui::IdSalt,
    label: Option<egui::WidgetText>,
    selected_text: egui::WidgetText,
    width: Option<f32>,
    height: Option<f32>,
    close_behavior: Option<egui::PopupCloseBehavior>,

    placeholder: egui::WidgetText,
    search_hint: Option<egui::WidgetText>,
    no_results_text: egui::WidgetText,
    /// The width of the text of the widest item, if the items are known. See [`Self::show_index`].
    widest_item: Option<f32>,
}

impl ComboBox {
    /// Create new [`ComboBox`] with id and label
    pub fn new(id_salt: impl egui::AsIdSalt, label: impl Into<egui::WidgetText>) -> Self {
        Self { label: Some(label.into()), ..Self::from_id_salt(id_salt) }
    }

    /// Label shown next to the combo box
    pub fn from_label(label: impl Into<egui::WidgetText>) -> Self {
        let label = label.into();
        Self::new(label.text().to_owned(), label)
    }

    /// Without label.
    pub fn from_id_salt(id_salt: impl egui::AsIdSalt) -> Self {
        Self {
            id_salt: egui::IdSalt::new(id_salt),
            label: None,
            selected_text: egui::WidgetText::default(),
            width: None,
            height: None,
            close_behavior: None,
            placeholder: "Select...".into(),
            search_hint: None,
            no_results_text: "No results found".into(),
            widest_item: None,
        }
    }

    /// Set the outer width of the button and menu. Without it, they are as wide as what they show.
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the maximum outer height of the menu (scrolls beyond it).
    pub const fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// What we show as the currently selected value.
    pub fn selected_text(mut self, selected_text: impl Into<egui::WidgetText>) -> Self {
        self.selected_text = selected_text.into();
        self
    }

    /// Controls the close behavior for the popup.
    ///
    /// By default, `PopupCloseBehavior::CloseOnClick` will be used.
    pub const fn close_behavior(mut self, close_behavior: egui::PopupCloseBehavior) -> Self {
        self.close_behavior = Some(close_behavior);
        self
    }

    // ── Extensions (not in egui) ────────────────────────────────

    /// Muted text shown while `selected_text` is empty. Default: `"Select..."`.
    pub fn placeholder(mut self, placeholder: impl Into<egui::WidgetText>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Makes [`Self::show_index`] searchable, with a search field showing this hint.
    pub fn search_hint(mut self, search_hint: impl Into<egui::WidgetText>) -> Self {
        self.search_hint = Some(search_hint.into());
        self
    }

    /// Text shown when a search matches nothing. Default: `"No results found"`.
    pub fn no_results_text(mut self, no_results_text: impl Into<egui::WidgetText>) -> Self {
        self.no_results_text = no_results_text.into();
        self
    }

    /// The outer width of the button: [`Self::width`], or what it shows.
    fn trigger_width(&self, ui: &egui::Ui, theme: &ShadcnTheme) -> f32 {
        let text = self
            .widest_item
            .unwrap_or_else(|| selected_galley(ui, theme, &self.selected_text, &self.placeholder).size().x);
        let fitting = (text + TRIGGER_CHROME).clamp(MIN_TRIGGER_WIDTH, MAX_TRIGGER_WIDTH);
        self.width.unwrap_or(fitting).min(ui.available_width())
    }

    /// The inner width of the menu: at least the button's and `min`, and what its items need.
    fn menu_width(&self, trigger: f32, min: f32) -> f32 {
        let items = self.widest_item.map_or(0.0, |text| (text + ITEM_CHROME).min(MAX_MENU_WIDTH));
        trigger.max(min).max(items)
    }

    // ── Show ────────────────────────────────────────────────────

    /// Show the combo box, with the given ui code for the menu contents.
    ///
    /// Returns `InnerResponse { inner: None }` if the combo box is closed.
    pub fn show_ui<R>(
        self,
        ui: &mut egui::Ui,
        menu_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<Option<R>> {
        let label = self.label.clone();
        with_label(ui, label, |ui| self.show_ui_impl(ui, menu_contents))
    }

    /// Show a list of items with the given selected index.
    ///
    /// The response is marked as changed when the selection changes.
    /// Unlike egui, `get(selected)` is not called when `selected >= len`.
    pub fn show_index<Text: Into<egui::WidgetText>>(
        mut self,
        ui: &mut egui::Ui,
        selected: &mut usize,
        len: usize,
        get: impl Fn(usize) -> Text,
    ) -> egui::Response {
        if *selected < len {
            self = self.selected_text(get(*selected));
        }
        self.widest_item = Some(widest_item(ui, ui.make_persistent_id(self.id_salt), len, &get));

        let label = self.label.clone();
        if self.search_hint.is_some() {
            return with_label(ui, label, |ui| {
                let response = self.show_searchable_index(ui, selected, len, get, None);
                egui::InnerResponse::new((), response)
            })
            .response;
        }

        let mut changed = false;
        let mut response = self
            .show_ui(ui, |ui| {
                for i in 0..len {
                    if ui.add(Button::selectable(i == *selected, get(i))).clicked() && *selected != i {
                        *selected = i;
                        changed = true;
                    }
                }
            })
            .response;

        if changed {
            response.mark_changed();
        }
        response
    }

    /// [`Self::show_index`] with a search, where what is searched for can also be taken as it is: for a
    /// value that need not be in the list. While the search is no item, the first row of the menu is
    /// `create_label: <the search>`, which a click or `Enter` puts in `created`.
    ///
    /// e.g. `DefaultMale~1`, `DefaultFemale~1` are listed, and `Mine~1` is typed.
    pub fn show_creatable<Text: Into<egui::WidgetText>>(
        mut self,
        ui: &mut egui::Ui,
        selected: &mut usize,
        len: usize,
        get: impl Fn(usize) -> Text,
        (create_label, created): (&str, &mut Option<String>),
    ) -> egui::Response {
        if *selected < len {
            self = self.selected_text(get(*selected));
        }
        self.widest_item = Some(widest_item(ui, ui.make_persistent_id(self.id_salt), len, &get));
        if self.search_hint.is_none() {
            self.search_hint = Some(egui::WidgetText::default());
        }
        let label = self.label.clone();
        with_label(ui, label, |ui| {
            let response = self.show_searchable_index(ui, selected, len, get, Some((create_label, created)));
            egui::InnerResponse::new((), response)
        })
        .response
    }

    fn show_ui_impl<R>(
        self,
        ui: &mut egui::Ui,
        menu_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<Option<R>> {
        let theme = ShadcnTheme::get(ui.ctx());

        let h_padding: f32 = 10.0;
        let chevron_width: f32 = 20.0;
        let width = self.trigger_width(ui, &theme);

        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, TRIGGER_HEIGHT), egui::Sense::hover());
        let response = ui.interact(rect, ui.make_persistent_id(self.id_salt), egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let cr = theme.corner_radius();
            let pressed = response.is_pointer_button_down_on();
            let trigger_bg = if pressed {
                interpolate_color(theme.background, theme.accent, 0.85)
            } else if response.hovered() {
                theme.accent
            } else {
                theme.background
            };
            let trigger_border = if response.hovered() || pressed { theme.ring } else { theme.border };

            painter.rect_filled(rect, cr, trigger_bg);
            painter.rect_stroke(rect, cr, egui::Stroke::new(1.0_f32, trigger_border), egui::epaint::StrokeKind::Inside);

            let galley = selected_galley(ui, &theme, &self.selected_text, &self.placeholder);
            let text_pos = egui::pos2(rect.min.x + h_padding, rect.center().y - galley.size().y / 2.0);
            // Not over the chevron and out of the button.
            painter.with_clip_rect(rect.with_max_x(rect.max.x - chevron_width - 4.0)).galley(
                text_pos,
                galley,
                theme.foreground,
            );

            let icon_size: f32 = 14.0;
            let chevron_rect = egui::Rect::from_center_size(
                egui::pos2(rect.max.x - chevron_width / 2.0 - 2.0, rect.center().y),
                egui::vec2(icon_size, icon_size),
            );
            paint_icon(painter, chevron_rect, Icon::ChevronDown, theme.muted_foreground);

            if response.has_focus() {
                paint_focus_ring(painter, rect, theme.radius, theme.ring);
            }
        }

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let toggle_cmd = response.clicked().then_some(egui::SetOpenCommand::Toggle);
        let inner = egui::Popup::new(response.id.with("popup"), ui.ctx().clone(), &response, ui.layer_id())
            .open_memory(toggle_cmd)
            .close_behavior(self.close_behavior.unwrap_or_default())
            .frame(
                egui::Frame::NONE
                    .fill(theme.popover)
                    .inner_margin(egui::Margin::same(4))
                    .corner_radius(theme.corner_radius())
                    .stroke(egui::Stroke::new(1.0_f32, with_alpha(theme.foreground, 26)))
                    .shadow(popup_shadow()),
            )
            .show(|ui: &mut egui::Ui| {
                let popup_width = self.menu_width(width, 144.0);
                ui.set_min_width(popup_width);
                ui.set_max_width(popup_width);
                ItemStyle::Select.scope(ui, |ui| match self.height {
                    Some(height) => egui::ScrollArea::vertical().max_height(height).show(ui, menu_contents).inner,
                    None => menu_contents(ui),
                })
            })
            .map(|inner| inner.inner);

        egui::InnerResponse::new(inner, response)
    }

    fn show_searchable_index<Text: Into<egui::WidgetText>>(
        self,
        ui: &mut egui::Ui,
        selected: &mut usize,
        len: usize,
        get: impl Fn(usize) -> Text,
        mut creatable: Option<(&str, &mut Option<String>)>,
    ) -> egui::Response {
        let theme = ShadcnTheme::get(ui.ctx());

        let icon_size = 12.0;
        let h_padding = 10.0;
        let width = self.trigger_width(ui, &theme);
        let menu_height = self.height.unwrap_or(250.0);

        let id = ui.make_persistent_id(self.id_salt);
        let (trigger_rect, _) = ui.allocate_exact_size(egui::vec2(width, TRIGGER_HEIGHT), egui::Sense::hover());
        let mut trigger = ui.interact(trigger_rect, id, egui::Sense::click());

        if ui.is_rect_visible(trigger_rect) {
            let cr = theme.corner_radius();
            let bg = if trigger.is_pointer_button_down_on() {
                interpolate_color(theme.background, theme.accent, 0.85)
            } else if trigger.hovered() {
                theme.accent
            } else {
                theme.background
            };

            ui.painter().rect_filled(trigger_rect, cr, bg);
            ui.painter().rect_stroke(
                trigger_rect,
                cr,
                egui::Stroke::new(1.0_f32, theme.input),
                egui::epaint::StrokeKind::Inside,
            );

            let galley = selected_galley(ui, &theme, &self.selected_text, &self.placeholder);
            let text_pos = egui::pos2(trigger_rect.min.x + h_padding, trigger_rect.center().y - galley.size().y / 2.0);
            // Not over the chevron and out of the button.
            let text_clip = trigger_rect.with_max_x(trigger_rect.max.x - h_padding - icon_size - 4.0);
            ui.painter().with_clip_rect(text_clip).galley(text_pos, galley, theme.foreground);

            let icon_rect = egui::Rect::from_min_size(
                egui::pos2(trigger_rect.max.x - h_padding - icon_size, trigger_rect.center().y - icon_size / 2.0),
                egui::vec2(icon_size, icon_size),
            );
            paint_icon(ui.painter(), icon_rect, Icon::ChevronDown, theme.muted_foreground);
        }

        if trigger.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let popup_id = id.with("popup");
        let search_id = id.with("search");

        let toggle_cmd = trigger.clicked().then_some(egui::SetOpenCommand::Toggle);
        let popup = egui::Popup::new(popup_id, ui.ctx().clone(), &trigger, ui.layer_id())
            .at_position(egui::pos2(trigger_rect.min.x, trigger_rect.max.y))
            .open_memory(toggle_cmd)
            .close_behavior(self.close_behavior.unwrap_or_default())
            .frame(
                egui::Frame::NONE
                    .fill(theme.popover)
                    .inner_margin(egui::Margin::same(8))
                    .corner_radius(egui::CornerRadius::same((theme.radius + 2.0).round() as u8))
                    .stroke(egui::Stroke::new(1.0_f32, theme.border))
                    .shadow(popup_shadow()),
            );

        let mut close = false;
        let mut changed = false;

        let mut search_text = ui.memory_mut(|mem| mem.data.get_temp::<String>(search_id).unwrap_or_default());

        popup.show(|ui| {
            let popup_width = self.menu_width(trigger_rect.width(), 180.0);
            ui.set_min_width(popup_width);
            ui.set_max_width(popup_width);

            let input_width = ui.available_width();
            let mut input = TextEdit::singleline(&mut search_text).desired_width(input_width);
            if let Some(hint) = self.search_hint {
                input = input.hint_text(hint);
            }
            let input_resp = ui.add(input);
            if trigger.clicked() {
                input_resp.request_focus();
            }

            ui.add_space(4.0);

            let query = search_text.to_lowercase();
            let filtered: Vec<(usize, egui::WidgetText)> = (0..len)
                .map(|i| (i, get(i).into()))
                .filter(|(_, text)| query.is_empty() || text.text().to_lowercase().contains(&query))
                .collect();

            let typed = search_text.trim().to_owned();
            let can_create =
                !typed.is_empty() && !filtered.iter().any(|(_, text)| text.text().eq_ignore_ascii_case(&typed));
            if let Some((create_label, created)) = creatable.as_mut().filter(|_| can_create) {
                let row = ItemStyle::Search
                    .scope(ui, |ui| ui.add(Button::selectable(false, format!("{create_label}: {typed}"))).clicked());
                if row || (input_resp.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))) {
                    **created = Some(typed);
                    search_text.clear();
                    close = true;
                    changed = true;
                    return;
                }
            } else if filtered.is_empty() {
                let text = egui::RichText::new(self.no_results_text.text()).color(theme.muted_foreground).size(13.0);
                ui.label(text);
                return;
            }

            egui::ScrollArea::vertical().max_height(menu_height).show(ui, |ui| {
                ItemStyle::Search.scope(ui, |ui| {
                    for (i, text) in filtered {
                        if ui.add(Button::selectable(i == *selected, text)).clicked() {
                            changed |= *selected != i;
                            *selected = i;
                            search_text.clear();
                            close = true;
                        }
                    }
                });
            });
        });

        ui.memory_mut(|mem| {
            if search_text.is_empty() {
                mem.data.remove::<String>(search_id);
            } else {
                mem.data.insert_temp(search_id, search_text);
            }
        });

        if close {
            egui::Popup::close_id(ui.ctx(), popup_id);
            ui.ctx().request_repaint();
        }

        if changed {
            trigger.mark_changed();
        }
        trigger
    }
}

/// The width of the text of the widest of the items. Measured when their number changes, not on each
/// frame: a list of fonts has hundreds.
fn widest_item<Text: Into<egui::WidgetText>>(
    ui: &egui::Ui,
    id: egui::Id,
    len: usize,
    get: &impl Fn(usize) -> Text,
) -> f32 {
    let key = id.with("widest_item");
    if let Some((for_len, widest)) = ui.data(|data| data.get_temp::<(usize, f32)>(key))
        && for_len == len
    {
        return widest;
    }
    let width_of = |i: usize| {
        let text: egui::WidgetText = get(i).into();
        let font = egui::FontId::proportional(14.0);
        ui.painter().layout_no_wrap(text.text().to_owned(), font, egui::Color32::PLACEHOLDER).size().x
    };
    let widest = (0..len).map(width_of).fold(0.0, f32::max);
    ui.data_mut(|data| data.insert_temp(key, (len, widest)));
    widest
}

/// Lays the combo box out horizontally with its label, like egui does (only when labelled).
fn with_label<R>(
    ui: &mut egui::Ui,
    label: Option<egui::WidgetText>,
    add_combo: impl FnOnce(&mut egui::Ui) -> egui::InnerResponse<R>,
) -> egui::InnerResponse<R> {
    let Some(label) = label else { return add_combo(ui) };

    ui.horizontal(|ui| {
        let mut ir = add_combo(ui);
        let label_response = ui.label(label);
        ir.response = ir.response.labelled_by(label_response.id);
        ir.response |= label_response;
        ir
    })
    .inner
}

/// Lays out the trigger text; the muted placeholder while `selected_text` is empty.
fn selected_galley(
    ui: &egui::Ui,
    theme: &ShadcnTheme,
    selected_text: &egui::WidgetText,
    placeholder: &egui::WidgetText,
) -> std::sync::Arc<egui::Galley> {
    let (text, color) = if selected_text.is_empty() {
        (placeholder.text(), theme.muted_foreground)
    } else {
        (selected_text.text(), theme.foreground)
    };
    ui.painter().layout_no_wrap(text.to_owned(), egui::FontId::proportional(14.0), color)
}

/// Paints a [`Button::selectable`] as a menu item of `style`.
pub(super) fn paint_item(
    ui: &mut egui::Ui,
    style: ItemStyle,
    selected: bool,
    text: &egui::WidgetText,
) -> egui::Response {
    let theme = ShadcnTheme::get(ui.ctx());
    let check_icon_size: f32 = 12.0;

    let galley =
        ui.painter().layout_no_wrap(text.text().to_owned(), egui::FontId::proportional(14.0), theme.popover_foreground);

    let desired = egui::vec2(ui.available_width(), galley.size().y + 8.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    let (highlight, cr, check_x, text_x) = match style {
        ItemStyle::Select => (
            response.hovered(),
            (theme.radius - 2.0).max(4.0).round() as u8,
            rect.max.x - check_icon_size - 8.0,
            rect.min.x + 6.0,
        ),
        ItemStyle::Search => (response.hovered() || selected, 4, rect.min.x + 4.0, rect.min.x + check_icon_size + 6.0),
    };

    if highlight {
        ui.painter().rect_filled(rect, egui::CornerRadius::same(cr), theme.accent);
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    if ui.is_rect_visible(rect) {
        if selected {
            let check_rect = egui::Rect::from_min_size(
                egui::pos2(check_x, rect.center().y - check_icon_size / 2.0),
                egui::vec2(check_icon_size, check_icon_size),
            );
            paint_icon(ui.painter(), check_rect, Icon::Check, theme.popover_foreground);
        }

        ui.painter().galley(
            egui::pos2(text_x, rect.center().y - galley.size().y / 2.0),
            galley,
            theme.popover_foreground,
        );
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Opens the real combo box in a window. Returns the button and the menu.
    fn open(search: bool, width: Option<f32>, items: &[&str]) -> (egui::Rect, egui::Rect) {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 900.0));
        let mut selected = 0;
        let (mut trigger, mut menu) = (egui::Rect::NOTHING, None);
        for frame in 0..7 {
            let mut input = egui::RawInput { screen_rect: Some(screen), ..Default::default() };
            if frame == 2 {
                input.events.push(egui::Event::PointerMoved(trigger.center()));
            }
            if frame == 3 || frame == 4 {
                input.events.push(egui::Event::PointerButton {
                    pos: trigger.center(),
                    button: egui::PointerButton::Primary,
                    pressed: frame == 3,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            let mut output = ctx.run_ui(input, |ui| {
                let ctx = ui.ctx().clone();
                egui::Window::new("window").show(&ctx, |ui| {
                    let mut combo = ComboBox::from_id_salt("combo");
                    if let Some(width) = width {
                        combo = combo.width(width);
                    }
                    if search {
                        combo = combo.search_hint("search");
                    }
                    let response = combo.show_index(ui, &mut selected, items.len(), |i| items[i]);
                    trigger = response.rect;
                    menu = ctx.memory(|memory| memory.area_rect(response.id.with("popup")));
                });
            });
            output.textures_delta.clear();
        }
        (trigger, menu.expect("the menu is open"))
    }

    const SHORT: [&str; 3] = ["a", "b", "c"];
    const LONG: [&str; 2] = ["DraugrSkeletonProjectData~DraugrSkeletonProject", "DefaultMale~1"];

    #[test]
    fn it_is_as_wide_as_what_it_shows() {
        for search in [false, true] {
            let (short, _) = open(search, None, &SHORT);
            assert!(short.width() <= MIN_TRIGGER_WIDTH + 1.0, "search: {search}: {short:?}");

            let (long, menu) = open(search, None, &LONG);
            assert!(long.width() > 200.0, "search: {search}: fits its text: {long:?}");
            assert!(long.width() <= MAX_TRIGGER_WIDTH + 1.0, "search: {search}: {long:?}");
            assert!(menu.width() >= long.width(), "search: {search}: {menu:?}");
            assert!((long.height() - TRIGGER_HEIGHT).abs() < 0.5, "both kinds look the same: {long:?}");
        }
    }

    #[test]
    fn a_width_is_kept_and_the_menu_still_shows_its_items() {
        for search in [false, true] {
            let (trigger, menu) = open(search, Some(90.0), &LONG);
            assert!((trigger.width() - 90.0).abs() < 0.5, "search: {search}: {trigger:?}");
            assert!(menu.width() > 250.0, "search: {search}: the items are whole: {menu:?}");
            assert!(menu.width() <= MAX_MENU_WIDTH + 24.0, "search: {search}: {menu:?}");
        }
    }

    /// Opens a creatable combo box, types `typed` and presses `Enter`. Returns what it created.
    fn type_and_enter(typed: &str) -> Option<String> {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 900.0));
        let (mut selected, mut created, mut trigger) = (usize::MAX, None, egui::Rect::NOTHING);
        let press = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let enter = egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        for frame in 0..9 {
            let events = match frame {
                2 => vec![egui::Event::PointerMoved(trigger.center())],
                3 => vec![press(trigger.center(), true)],
                4 => vec![press(trigger.center(), false)],
                6 => vec![egui::Event::Text(typed.to_owned())],
                7 => vec![enter.clone()],
                _ => Vec::new(),
            };
            let input = egui::RawInput { screen_rect: Some(screen), events, ..Default::default() };
            let mut output = ctx.run_ui(input, |ui| {
                let ctx = ui.ctx().clone();
                egui::Window::new("window").show(&ctx, |ui| {
                    let response = ComboBox::from_id_salt("combo").show_creatable(
                        ui,
                        &mut selected,
                        LONG.len(),
                        |i| LONG[i],
                        ("New", &mut created),
                    );
                    trigger = response.rect;
                });
            });
            output.textures_delta.clear();
        }
        created
    }

    #[test]
    fn what_is_searched_for_can_be_taken_as_it_is() {
        assert_eq!(type_and_enter("Mine~1").as_deref(), Some("Mine~1"));
        assert_eq!(type_and_enter(" Mine~1 ").as_deref(), Some("Mine~1"));
        // It is in the list: that is no new value, whatever its case.
        assert_eq!(type_and_enter("defaultmale~1"), None);
        assert_eq!(type_and_enter("   "), None);
    }

    #[test]
    fn a_search_menu_is_as_tall_as_its_results() {
        let (_, menu) = open(true, None, &SHORT);
        assert!(menu.height() < 200.0, "three items: {menu:?}");
    }
}
