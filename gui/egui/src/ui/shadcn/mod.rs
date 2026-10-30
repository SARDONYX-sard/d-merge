//! Minimal shadcn/ui-inspired widgets for egui.
//!
//! Ported from the subset of [egui-shadcn](https://github.com/pjankiewicz/egui-shadcn) used by this app.
//!
//! The API mirrors egui's: replacing `use crate::ui::shadcn::{..}` with `use egui::{..}` compiles
//! once the extension builder methods (marked "not in egui") are commented out.
//! Unlike egui-shadcn, every widget reports `Response::changed()` correctly.

// Keep the layout arithmetic readable (and identical to the original) instead of `mul_add`.
#![allow(clippy::suboptimal_flops)]
// egui-compatible builder methods are provided even when this app does not use them (yet).
#![allow(dead_code)]

mod button;
mod controls;
pub(crate) mod icon;
mod select;
mod text;
pub(crate) mod theme;

pub(crate) use self::{
    button::Button,
    controls::{Checkbox, RadioButton, Slider, ToggleGroup},
    icon::Icon,
    select::ComboBox,
    text::{Label, TextEdit},
};
