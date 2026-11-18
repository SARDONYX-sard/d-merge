//! The input of a value in the details panel, by its type: only what the type can hold can be
//! entered, and it is written as Havok's XML writes it.
//!
//! ```text
//! type                      input                      written
//! Bool                      checkbox                   true / false
//! Int8..Uint64, Char        number, in the type's range 42
//! Real, Half                number                     1.500000
//! Enum                      list of its values         MODE_LOOPING
//! Flags                     a checkbox per flag        FLAG_A|FLAG_B, or 0
//! Vector4, Quaternion       four numbers               (0.000000 1.000000 0.000000 0.000000)
//! anything else             text (no `<`)              as typed
//! ```
//!
//! A number being dragged is kept apart (egui's memory) and written once the drag ends: the
//! object's text, its graph and its problems are not redone every frame.

use egui_widgets::shadcn::{Checkbox, ComboBox, NumberInput, TextEdit};
use nemesis_diff::schema::{FieldSchema, TypeKind};

/// The height of an input: a row of the details.
const HEIGHT: f32 = 24.0;
/// An enum with more values than this can be searched.
const SEARCH_FROM: usize = 8;

/// The input of `text`, of `kind` by `schema`. Returns the new text once one is made.
pub(super) fn value_input(
    ui: &mut egui::Ui,
    id: egui::Id,
    (text, schema, kind): (&str, Option<&'static FieldSchema>, Option<TypeKind>),
) -> Option<String> {
    let values = schema.map_or(&[][..], |schema| schema.values);
    match kind {
        Some(TypeKind::Bool) => bool_input(ui, text),
        Some(kind) if int_range(kind).is_some() => {
            let (min, max) = int_range(kind)?;
            let current = text.trim().parse::<f64>().ok()?;
            let width = ui.available_width();
            number_input(ui, id, (current, 1.0, 0), (min..=max, width)).map(|n| format!("{n:.0}"))
        }
        Some(TypeKind::Real | TypeKind::Half) => {
            let current = text.trim().parse::<f64>().ok()?;
            let width = ui.available_width();
            number_input(ui, id, (current, 0.01, 6), (f64::MIN..=f64::MAX, width)).map(format_real)
        }
        Some(TypeKind::Enum) if !values.is_empty() => enum_input(ui, id, text, values),
        Some(TypeKind::Flags) if is_flags(text, values) => flags_input(ui, id, text, values),
        Some(TypeKind::Vector4 | TypeKind::Quaternion) => vector_input(ui, id, text),
        _ => text_input(ui, id, text),
    }
}

/// The range of an integer type, as `f64` (what the number input holds). `None`: not an integer.
///
/// e.g. `Uint8` -> `0..=255`
fn int_range(kind: TypeKind) -> Option<(f64, f64)> {
    #[expect(clippy::cast_precision_loss, reason = "the ends of 64-bit ranges: near enough")]
    let range = match kind {
        TypeKind::Int8 | TypeKind::Char => (f64::from(i8::MIN), f64::from(i8::MAX)),
        TypeKind::Uint8 => (0.0, f64::from(u8::MAX)),
        TypeKind::Int16 => (f64::from(i16::MIN), f64::from(i16::MAX)),
        TypeKind::Uint16 => (0.0, f64::from(u16::MAX)),
        TypeKind::Int32 => (f64::from(i32::MIN), f64::from(i32::MAX)),
        TypeKind::Uint32 => (0.0, f64::from(u32::MAX)),
        TypeKind::Int64 => (i64::MIN as f64, i64::MAX as f64),
        TypeKind::Uint64 | TypeKind::Ulong => (0.0, u64::MAX as f64),
        _ => return None,
    };
    Some(range)
}

/// e.g. `1.5` -> `1.500000`
fn format_real(value: f64) -> String {
    format!("{value:.6}")
}

/// `(x y z w)` -> its four numbers.
///
/// e.g. `(0.000000 1.000000 0.000000 0.000000)` -> `[0, 1, 0, 0]`
fn parse_vector(text: &str) -> Option<[f64; 4]> {
    let inner = text.trim().strip_prefix('(')?.strip_suffix(')')?;
    let mut numbers = inner.split_whitespace().map(str::parse::<f64>);
    let vector = [numbers.next()?.ok()?, numbers.next()?.ok()?, numbers.next()?.ok()?, numbers.next()?.ok()?];
    numbers.next().is_none().then_some(vector)
}

fn format_vector([x, y, z, w]: [f64; 4]) -> String {
    format!("({x:.6} {y:.6} {z:.6} {w:.6})")
}

/// Is `text` flags of `values` (`FLAG_A|FLAG_B`, or `0`)? Else it is typed as text.
fn is_flags(text: &str, values: &[&str]) -> bool {
    !values.is_empty() && flags_of(text).all(|flag| values.contains(&flag))
}

/// e.g. `FLAG_A|FLAG_B` -> `FLAG_A`, `FLAG_B`; `0` -> none
fn flags_of(text: &str) -> impl Iterator<Item = &str> {
    text.split('|').map(str::trim).filter(|flag| !flag.is_empty() && *flag != "0")
}

/// `on` in the order of `values`, joined: `0` if none.
fn flags_text(values: &[&str], on: impl Fn(&str) -> bool) -> String {
    let set: Vec<&str> = values.iter().copied().filter(|value| on(value)).collect();
    if set.is_empty() { "0".to_owned() } else { set.join("|") }
}

fn bool_input(ui: &mut egui::Ui, text: &str) -> Option<String> {
    let mut on = text.trim() == "true";
    ui.add(Checkbox::without_text(&mut on)).changed().then(|| on.to_string())
}

/// A number: dragged (kept apart until it ends) or typed. `(current, speed, decimals)`,
/// `(range, width)`.
fn number_input(
    ui: &mut egui::Ui,
    id: egui::Id,
    (current, speed, decimals): (f64, f64, usize),
    (range, width): (std::ops::RangeInclusive<f64>, f32),
) -> Option<f64> {
    let mut value = ui.data(|data| data.get_temp::<f64>(id)).unwrap_or(current);
    let input = NumberInput::new(&mut value).speed(speed).range(range).max_decimals(decimals);
    let response = ui.add(input.desired_width(width).height(HEIGHT));
    if response.dragged() {
        ui.data_mut(|data| data.insert_temp(id, value));
        return None;
    }
    ui.data_mut(|data| data.remove::<f64>(id));
    #[expect(clippy::float_cmp, reason = "a change at all")]
    let changed = value != current;
    ((response.changed() || response.drag_stopped()) && changed).then_some(value)
}

fn enum_input(ui: &mut egui::Ui, id: egui::Id, text: &str, values: &[&str]) -> Option<String> {
    let mut index = values.iter().position(|value| *value == text).unwrap_or(usize::MAX);
    let mut select = ComboBox::from_id_salt(id).selected_text(text).width(ui.available_width()).trigger_height(HEIGHT);
    if values.len() > SEARCH_FROM {
        select = select.search_hint("");
    }
    let response = select.show_index(ui, &mut index, values.len(), |at| values[at]);
    response.changed().then(|| values.get(index).map(|value| (*value).to_owned())).flatten()
}

/// The flags set, a checkbox per flag in a list that stays open while they are set.
fn flags_input(ui: &mut egui::Ui, id: egui::Id, text: &str, values: &[&str]) -> Option<String> {
    let mut picked = None;
    let select = ComboBox::from_id_salt(id)
        .selected_text(text)
        .width(ui.available_width())
        .trigger_height(HEIGHT)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
    select.show_ui(ui, |ui| {
        for &value in values {
            let mut on = flags_of(text).any(|flag| flag == value);
            if ui.add(Checkbox::new(&mut on, value)).changed() {
                let toggled = |flag: &str| if flag == value { on } else { flags_of(text).any(|set| set == flag) };
                picked = Some(flags_text(values, toggled));
            }
        }
    });
    picked
}

fn vector_input(ui: &mut egui::Ui, id: egui::Id, text: &str) -> Option<String> {
    let Some(mut vector) = parse_vector(text) else { return text_input(ui, id, text) };
    let spacing = ui.spacing().item_spacing.x;
    let width = spacing.mul_add(-3.0, ui.available_width()) / 4.0;
    let mut changed = false;
    for (i, part) in vector.iter_mut().enumerate() {
        if let Some(value) = number_input(ui, id.with(i), (*part, 0.01, 6), (f64::MIN..=f64::MAX, width)) {
            *part = value;
            changed = true;
        }
    }
    changed.then(|| format_vector(vector))
}

/// Text, written when the field is left (`Esc`: not). `<` would break the XML: not written.
fn text_input(ui: &mut egui::Ui, id: egui::Id, text: &str) -> Option<String> {
    let mut buffer = ui.data(|data| data.get_temp::<String>(id)).unwrap_or_else(|| text.to_owned());
    let field = TextEdit::singleline(&mut buffer).id_salt(id).desired_width(ui.available_width()).height(HEIGHT);
    let response = ui.add(field);
    if response.has_focus() {
        ui.data_mut(|data| data.insert_temp(id, buffer));
        return None;
    }
    ui.data_mut(|data| data.remove::<String>(id));
    let cancelled = ui.input(|input| input.key_pressed(egui::Key::Escape));
    (response.lost_focus() && !cancelled && buffer != text && !buffer.contains('<')).then_some(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_written_as_havok_writes_them() {
        assert_eq!(format_real(1.5), "1.500000");
        assert_eq!(parse_vector("(0.000000 1.000000 0.000000 0.500000)"), Some([0.0, 1.0, 0.0, 0.5]));
        assert_eq!(parse_vector("(1 2 3)"), None);
        assert_eq!(format_vector([0.0, 1.0, 0.0, 0.5]), "(0.000000 1.000000 0.000000 0.500000)");
        assert_eq!(int_range(TypeKind::Uint8), Some((0.0, 255.0)));
        assert_eq!(int_range(TypeKind::Real), None);
    }

    #[test]
    fn flags_are_set_in_the_order_of_their_values() {
        let values = ["FLAG_A", "FLAG_B", "FLAG_C"];
        assert!(is_flags("FLAG_C|FLAG_A", &values));
        assert!(is_flags("0", &values));
        assert!(!is_flags("7", &values), "a number: typed as text");
        assert_eq!(flags_text(&values, |flag| flag != "FLAG_B"), "FLAG_A|FLAG_C");
        assert_eq!(flags_text(&values, |_| false), "0");
    }
}
