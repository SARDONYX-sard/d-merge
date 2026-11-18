//! The colors of the node graph, as a blueprint's: a node's header by its kind of class, a pin (and
//! its edge) by the kind it points to, a value by its type.
//!
//! ```text
//! kind        classes (and their subclasses)                       color
//! generator   hkbGenerator: hkbStateMachine, hkbClipGenerator, ...  blue
//! modifier    hkbModifier: hkbModifierList, hkbEventDrivenModifier  green
//! transition  hkbTransitionEffect: hkbBlendingTransitionEffect      purple
//! condition   hkbCondition: hkbExpressionCondition, ...             orange
//! data        ...Data, ...Array, ...Set: hkbBehaviorGraphData, ...  gray
//!
//! value       Bool red, integers teal, Real green, strings pink, enums / flags yellow
//! ```

use egui::Color32;
use nemesis_diff::schema::{FieldSchema, TypeKind, is_subclass};

const GENERATOR: Color32 = Color32::from_rgb(70, 130, 220);
const MODIFIER: Color32 = Color32::from_rgb(70, 170, 110);
const TRANSITION: Color32 = Color32::from_rgb(150, 100, 210);
const CONDITION: Color32 = Color32::from_rgb(220, 140, 60);
const DATA: Color32 = Color32::from_rgb(130, 130, 145);

const BOOL: Color32 = Color32::from_rgb(210, 80, 80);
const INTEGER: Color32 = Color32::from_rgb(60, 180, 180);
const REAL: Color32 = Color32::from_rgb(120, 190, 90);
const STRING: Color32 = Color32::from_rgb(220, 100, 180);
const ENUM: Color32 = Color32::from_rgb(210, 180, 60);

/// The color of a kind of class. `None`: of no kind above, e.g. `hkRootLevelContainer`.
///
/// e.g. `hkbClipGenerator` -> blue, `hkbBehaviorGraphStringData` -> gray
pub(super) fn of_class(class: &str) -> Option<Color32> {
    // The nearest kind first: a transition effect is a generator too.
    let kinds = [
        ("hkbTransitionEffect", TRANSITION),
        ("hkbModifier", MODIFIER),
        ("hkbCondition", CONDITION),
        ("hkbGenerator", GENERATOR),
    ];
    let kind = kinds.into_iter().find(|(base, _)| is_subclass(class, base)).map(|(_, color)| color);
    kind.or_else(|| ["Data", "Array", "Set"].iter().any(|end| class.ends_with(end)).then_some(DATA))
}

/// The color of a field's row: of what a pointer points to, or of the value's type.
pub(super) fn of_field(field: &FieldSchema) -> Option<Color32> {
    if nemesis_diff::schema::holds_pointers(field) {
        return field.class.and_then(of_class);
    }
    match field.kind {
        TypeKind::Bool => Some(BOOL),
        TypeKind::Char
        | TypeKind::Int8
        | TypeKind::Uint8
        | TypeKind::Int16
        | TypeKind::Uint16
        | TypeKind::Int32
        | TypeKind::Uint32
        | TypeKind::Int64
        | TypeKind::Uint64
        | TypeKind::Ulong => Some(INTEGER),
        TypeKind::Real | TypeKind::Half => Some(REAL),
        TypeKind::CString | TypeKind::StringPtr => Some(STRING),
        TypeKind::Enum | TypeKind::Flags => Some(ENUM),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_have_the_color_of_their_kind() {
        assert_eq!(of_class("hkbClipGenerator"), Some(GENERATOR));
        assert_eq!(of_class("hkbStateMachine"), Some(GENERATOR));
        assert_eq!(of_class("hkbModifierList"), Some(MODIFIER));
        assert_eq!(of_class("hkbBlendingTransitionEffect"), Some(TRANSITION));
        assert_eq!(of_class("hkbExpressionCondition"), Some(CONDITION));
        assert_eq!(of_class("hkbBehaviorGraphStringData"), Some(DATA));
        assert_eq!(of_class("hkRootLevelContainer"), None);
    }
}
