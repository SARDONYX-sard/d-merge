//! Minimal shadcn/ui-inspired widgets for egui.
//!
//! Ported from the subset of [egui-shadcn](https://github.com/pjankiewicz/egui-shadcn) used by this app.
//!
//! The API mirrors egui's: replacing `use egui_widgets::shadcn::{..}` with `use egui::{..}` compiles
//! once the extension builder methods (marked "not in egui") are commented out.
//! Unlike egui-shadcn, every widget reports `Response::changed()` correctly.

// Keep the layout arithmetic readable (and identical to the original) instead of `mul_add`.
#![allow(clippy::suboptimal_flops)]

mod button;
mod controls;
pub mod icon;
mod select;
mod text;
pub mod theme;

pub use self::{
    button::Button,
    controls::{Checkbox, RadioButton, Slider, ToggleGroup},
    icon::Icon,
    select::ComboBox,
    text::{Label, TextEdit},
};
