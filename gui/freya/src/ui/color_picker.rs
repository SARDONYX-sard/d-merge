// SPDX-FileCopyrightText: (C) 2026 Marc Espín Sanz
// SPDX-License-Identifier: MIT
//
// Fork of `freya_extra::color_picker::ColorPicker`(itself a fork of freya-components' ColorPicker).
//
// Fixes:
// - The cursor position and the value disagreed: the hue/saturation were re-derived from RGB on every
//   update, so they were lost when the saturation or the value reached 0 (e.g. picking black or gray
//   reset the hue to red). Now H/S/V/A are kept as the source of truth.
// - The picker kept its first value: a new `value` from the parent(e.g. after loading another preset)
//   was ignored. Now it follows `value` unless the user is dragging.
// - The value can be typed as `#RRGGBB` / `#RRGGBBAA`.
use freya::{
    animation::{
        AnimNum, AnimatedValue as _, Ease, Function, OnChange, OnCreation, ReadAnimatedValue as _,
        use_animation,
    },
    prelude::*,
    torin::geometry::Point2D,
};

/// Width of the popup content: popup width(240) - padding(8 * 2).
const CONTENT_WIDTH: f32 = 224.0;
const SV_HEIGHT: f32 = 140.0;
const BAR_HEIGHT: f32 = 16.0;

/// HSV + alpha.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Hsva {
    /// 0..360
    h: f32,
    /// 0..=1
    s: f32,
    /// 0..=1
    v: f32,
    a: u8,
}

impl Hsva {
    fn from_color(color: Color) -> Self {
        let hsv = color.to_hsv();
        Self { h: hsv.h.rem_euclid(360.0), s: hsv.s, v: hsv.v, a: color.a() }
    }

    fn to_color(self) -> Color {
        let rgb = Color::from_hsv(self.h, self.s, self.v);
        Color::from_argb(self.a, rgb.r(), rgb.g(), rgb.b())
    }
}

/// `#RRGGBBAA`
fn to_hex(color: Color) -> String {
    format!("#{:02X}{:02X}{:02X}{:02X}", color.r(), color.g(), color.b(), color.a())
}

/// Parses `#RRGGBB` or `#RRGGBBAA`.
fn parse_hex(text: &str) -> Option<Color> {
    let hex = text.trim().strip_prefix('#').unwrap_or_else(|| text.trim());
    let byte = |index: usize| u8::from_str_radix(hex.get(index..index + 2)?, 16).ok();

    match hex.len() {
        6 => Some(Color::from_argb(255, byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color::from_argb(byte(6)?, byte(0)?, byte(2)?, byte(4)?)),
        _ => None,
    }
}

/// HSV-based gradient color picker with alpha.
#[derive(Clone, PartialEq)]
pub(crate) struct ColorPicker {
    value: Color,
    on_change: EventHandler<Color>,
    width: Size,
    key: DiffKey,
}

impl KeyExt for ColorPicker {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl ColorPicker {
    pub(crate) fn new(on_change: impl Into<EventHandler<Color>>) -> Self {
        Self {
            value: Color::WHITE,
            on_change: on_change.into(),
            width: Size::px(240.),
            key: DiffKey::None,
        }
    }

    pub(crate) const fn value(mut self, value: Color) -> Self {
        self.value = value;
        self
    }
}

/// Which part of the color picker is being dragged, if any.
#[derive(Clone, Copy, PartialEq, Default)]
enum DragTarget {
    #[default]
    None,
    Sv,
    Hue,
    Alpha,
}

/// Ratio of `coords` inside `area` along x and y, clamped to `0..=1`.
fn ratio(area: Area, coords: CursorPoint) -> Option<(f32, f32)> {
    let area = area.to_f64();
    if area.width() <= 0.0 || area.height() <= 0.0 {
        return None; // Not laid out yet.
    }
    let x = ((coords.x - area.min_x()) / area.width()).clamp(0.0, 1.0) as f32;
    let y = ((coords.y - area.min_y()) / area.height()).clamp(0.0, 1.0) as f32;
    Some((x, y))
}

impl Component for ColorPicker {
    #[allow(clippy::too_many_lines)]
    fn render(&self) -> impl IntoElement {
        let mut open = use_state(|| false);
        let mut hsva = use_state(|| Hsva::from_color(self.value));
        // The value last emitted or received, to detect a new value from the parent.
        let mut last_value = use_state(|| self.value);
        let mut hex_text = use_state(|| to_hex(self.value));
        let mut dragging = use_state(DragTarget::default);
        let sv_area = use_state(Area::default);
        let mut preview_area = use_state(|| None::<Area>);
        let mut popup_size = use_state(|| None::<Size2D>);

        let mut popup_area = use_state(|| None::<Area>);

        // Follow the parent value, unless it is the value this picker just emitted.
        use_side_effect_with_deps(&self.value, move |value| {
            if *last_value.peek() == *value {
                return;
            }
            last_value.set(*value);
            // Keep the hue when the new value has no hue. (gray)
            let mut next = Hsva::from_color(*value);
            if next.s == 0.0 || next.v == 0.0 {
                next.h = hsva.peek().h;
            }
            hsva.set(next);
            hex_text.set(to_hex(*value));
        });

        let theme = get_theme!(
            &None::<ColorPickerThemePartial>,
            ColorPickerThemePreference,
            "color_picker"
        );
        let is_open = open();
        let current = *hsva.read();
        let current_color = current.to_color();

        let emit = {
            let on_change = self.on_change.clone();
            move |next: Hsva| {
                let mut hsva = hsva;
                if *hsva.peek() == next {
                    return;
                }
                hsva.set(next);
                let color = next.to_color();
                let mut last_value = last_value;
                last_value.set(color);
                let mut hex_text = hex_text;
                hex_text.set(to_hex(color));
                on_change.call(color);
            }
        };

        let update = {
            let emit = emit.clone();
            move |target: DragTarget, coords: CursorPoint| {
                let mut next = *hsva.peek();
                match target {
                    DragTarget::Sv => {
                        let Some((x, y)) = ratio(*sv_area.peek(), coords) else { return };
                        next.s = x;
                        next.v = 1.0 - y;
                    }
                    DragTarget::Hue => {
                        let Some((x, _)) = ratio(*sv_area.peek(), coords) else { return };
                        // NOTE: Clamp to 359 degrees to prevent wrapping at the 360-degree boundary.
                        next.h = (x * 360.0).min(359.0);
                    }
                    DragTarget::Alpha => {
                        let Some((x, _)) = ratio(*sv_area.peek(), coords) else { return };
                        next.a = (x * 255.0).round() as u8;
                    }
                    DragTarget::None => return,
                }
                emit(next);
            }
        };

        let pointer_down = |target: DragTarget| {
            let update = update.clone();
            move |e: Event<PointerEventData>| {
                if !e.data().is_primary() {
                    return;
                }
                let mut dragging = dragging;
                dragging.set(target);
                // NOTE: `on_sized` is not delivered inside the overlay popup, so the area is built from the
                //       pressed element: origin = global - element location, size = the fixed layout size.
                let (global, local) = (e.global_location(), e.element_location());
                let height = if target == DragTarget::Sv { SV_HEIGHT } else { BAR_HEIGHT };
                let mut drag_area = sv_area;
                drag_area.set(Area::new(
                    Point2D::new((global.x - local.x) as f32, (global.y - local.y) as f32),
                    Size2D::new(CONTENT_WIDTH, height),
                ));
                update(target, global);
                e.stop_propagation();
                e.prevent_default();
            }
        };

        let preview = rect()
            .width(Size::px(40.))
            .height(Size::px(20.))
            .corner_radius(4.)
            .background(self.value)
            .border(
                Border::new().fill(theme.border_fill).width(1.).alignment(BorderAlignment::Inner),
            )
            .on_sized(move |e: Event<SizedEventData>| preview_area.set_if_modified(Some(e.area)))
            .on_press(move |_| open.toggle());

        const MARKER_WIDTH: f32 = 3.0;
        let marker = |bar_width: f32, ratio: f32| {
            let x = bar_width
                .mul_add(ratio, -(MARKER_WIDTH / 2.0))
                .clamp(0.0, (bar_width - MARKER_WIDTH).max(0.0));
            rect()
                // NOTE: `offset_x` is not applied to an absolute element; use `left`/`top`.
                .position(Position::new_absolute().left(x).top(0.0))
                .width(Size::px(MARKER_WIDTH))
                .height(Size::px(BAR_HEIGHT))
                .background(Color::WHITE)
                .border(
                    Border::new().fill(Color::BLACK).width(1.).alignment(BorderAlignment::Outer),
                )
        };

        // -- Saturation(x) / value(y)
        let sv = {
            let area = Area::from_size(Size2D::new(CONTENT_WIDTH, SV_HEIGHT)).to_f64();
            let cursor = 14.0_f32;
            let x = current
                .s
                .mul_add(area.width() as f32, -(cursor / 2.0))
                .clamp(0.0, (area.width() as f32 - cursor).max(0.0));
            let y = (1.0 - current.v)
                .mul_add(area.height() as f32, -(cursor / 2.0))
                .clamp(0.0, (area.height() as f32 - cursor).max(0.0));

            rect()
                .height(Size::px(SV_HEIGHT))
                .width(Size::fill())
                .corner_radius(4.)
                .overflow(Overflow::Clip)
                .on_pointer_down(pointer_down(DragTarget::Sv))
                .child(
                    rect()
                        .expanded()
                        .background(
                            LinearGradient::new()
                                .angle(-90.)
                                .stop(((255, 255, 255), 0.))
                                .stop((Color::from_hsv(current.h, 1.0, 1.0), 100.)),
                        )
                        .child(
                            rect().position(Position::new_absolute()).expanded().background(
                                LinearGradient::new()
                                    .stop(((255, 255, 255, 0.0), 0.))
                                    .stop(((0, 0, 0), 100.)),
                            ),
                        ),
                )
                .child(
                    rect()
                        .position(Position::new_absolute().left(x).top(y))
                        .width(Size::px(cursor))
                        .height(Size::px(cursor))
                        .corner_radius(cursor / 2.0)
                        .border(
                            Border::new()
                                .fill(Color::WHITE)
                                .width(2.)
                                .alignment(BorderAlignment::Inner),
                        )
                        .background(current_color),
                )
        };

        // -- Hue
        let hue = {
            let width = CONTENT_WIDTH;
            rect()
                .height(Size::px(BAR_HEIGHT))
                .width(Size::fill())
                .corner_radius(4.)
                .on_pointer_down(pointer_down(DragTarget::Hue))
                .background(
                    LinearGradient::new()
                        .angle(-90.)
                        .stop(((255, 0, 0), 0.))
                        .stop(((255, 255, 0), 100. / 6.))
                        .stop(((0, 255, 0), 200. / 6.))
                        .stop(((0, 255, 255), 50.))
                        .stop(((0, 0, 255), 400. / 6.))
                        .stop(((255, 0, 255), 500. / 6.))
                        .stop(((255, 0, 0), 100.)),
                )
                .child(marker(width, current.h / 360.0))
        };

        // -- Alpha
        let alpha = {
            let width = CONTENT_WIDTH;
            let (r, g, b) = (current_color.r(), current_color.g(), current_color.b());
            rect()
                .height(Size::px(BAR_HEIGHT))
                .width(Size::fill())
                .corner_radius(4.)
                .background((190, 190, 190))
                .on_pointer_down(pointer_down(DragTarget::Alpha))
                .child(
                    rect().expanded().corner_radius(4.).background(
                        LinearGradient::new()
                            .angle(-90.)
                            .stop((Color::from_argb(0, r, g, b), 0.))
                            .stop((Color::from_argb(255, r, g, b), 100.)),
                    ),
                )
                .child(marker(width, f32::from(current.a) / 255.0))
        };

        // -- Typed value
        let hex_input = Input::new(hex_text).width(Size::fill()).on_submit({
            move |text: String| {
                if let Some(color) = parse_hex(&text) {
                    let mut next = Hsva::from_color(color);
                    if next.s == 0.0 || next.v == 0.0 {
                        next.h = hsva.peek().h;
                    }
                    emit(next);
                } else {
                    hex_text.set(to_hex(hsva.peek().to_color()));
                }
            }
        });

        let on_global_pointer_move = {
            let update = update.clone();
            move |e: Event<PointerEventData>| {
                let target = *dragging.peek();
                if target != DragTarget::None {
                    update(target, e.global_location());
                }
            }
        };
        let on_global_pointer_up = move |_| {
            dragging.set_if_modified(DragTarget::None);
        };
        // Close when clicking outside of the popup and the preview.
        let on_global_pointer_down = move |e: Event<PointerEventData>| {
            let point = e.global_location();
            let inside =
                |area: Option<Area>| area.is_some_and(|area| area.contains(point.to_f32()));
            if !inside(*popup_area.peek()) && !inside(*preview_area.peek()) {
                open.set_if_modified(false);
            }
        };

        let animation = use_animation(move |conf| {
            conf.on_change(OnChange::Rerun);
            conf.on_creation(OnCreation::Finish);

            let scale = AnimNum::new(0.8, 1.).time(150).ease(Ease::Out).function(Function::Expo);
            let opacity = AnimNum::new(0., 1.).time(150).ease(Ease::Out).function(Function::Expo);

            if open() { (scale, opacity) } else { (scale, opacity).into_reversed() }
        });
        let (scale, opacity) = animation.read().value();

        // Show the popup next to the preview, inside the window.
        let (offset_x, offset_y) = match (preview_area(), popup_size()) {
            (Some(preview), Some(size)) => {
                let window = Area::from_size(*Platform::get().current_window().root_size.peek());
                let popup = Area::new(Point2D::new(preview.max_x() + 4.0, preview.min_y()), size);
                let target = if popup.max_x() > window.max_x() && preview.min_x() >= size.width {
                    Point2D::new(preview.min_x() - size.width - 4.0, popup.min_y())
                } else {
                    popup.origin
                };
                let clamped = target.clamp(window.origin, window.max() - size.to_vector());
                let offset = clamped - popup.origin;
                (offset.x, offset.y)
            }
            _ => (0., 0.),
        };

        let popup = rect()
            .on_global_pointer_move(on_global_pointer_move)
            .on_global_pointer_up(on_global_pointer_up)
            .on_global_pointer_down(on_global_pointer_down)
            .width(self.width.clone())
            .padding(8.)
            .corner_radius(6.)
            .background(theme.background)
            .border(
                Border::new().fill(theme.border_fill).width(1.).alignment(BorderAlignment::Inner),
            )
            .color(theme.color)
            .spacing(8.)
            .shadow(Shadow::new().x(0.).y(2.).blur(8.).color((0, 0, 0, 0.25)))
            .child(sv)
            .child(hue)
            .child(alpha)
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .spacing(8.)
                    .child(rect().width(Size::flex(1.0)).child(hex_input))
                    .child(Button::new().compact().on_press(move |_| open.set(false)).child("OK")),
            );

        rect().horizontal().child(preview).maybe_child((is_open || opacity > 0.).then(|| {
            rect()
                .layer(Layer::Overlay)
                .width(Size::px(0.))
                .height(Size::px(0.))
                .offset_x(offset_x)
                .offset_y(offset_y)
                .opacity(opacity)
                .child(popup.scale(scale).on_sized(move |e: Event<SizedEventData>| {
                    popup_size.set_if_modified(Some(e.area.size));
                    popup_area.set_if_modified(Some(e.area));
                }))
        }))
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let color = Color::from_argb(0x80, 0x12, 0xAB, 0xEF);
        assert_eq!(to_hex(color), "#12ABEF80");
        assert_eq!(parse_hex("#12ABEF80"), Some(color));
        assert_eq!(parse_hex("12abef"), Some(Color::from_argb(255, 0x12, 0xAB, 0xEF)));
        assert_eq!(parse_hex("#123"), None);
    }

    #[test]
    fn hsva_keeps_value() {
        let color = Color::from_argb(200, 10, 120, 240);
        let back = Hsva::from_color(color).to_color();
        assert!(
            back.r().abs_diff(10) <= 1
                && back.g().abs_diff(120) <= 1
                && back.b().abs_diff(240) <= 1
        );
        assert_eq!(back.a(), 200);
    }
}
