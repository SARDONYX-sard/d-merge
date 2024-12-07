mod array_handler;
mod other_handler;

use crate::range::Range;
use array_handler::{
    process_array_from, process_array_from_to, process_array_index, process_array_to,
};
use other_handler::{process_from, process_from_to, process_index, process_to};
use simd_json::borrowed::{Object, Value};
use std::borrow::Cow;

/// Add `value` to the `range` portion of `target`.
///
/// # Note
/// - If `range.start` is out of bounds, then extend by default value.
pub fn handle_add<'value>(target: &mut Vec<Value<'value>>, range: Range, value: Value<'value>) {
    match value {
        Value::Array(vec) => {
            let vec = *vec;
            match range {
                Range::Index(index) => process_array_index(target, vec, index),
                Range::FromTo(range) => process_array_from_to(target, vec, range),
                Range::To(range_to) => process_array_to(target, vec, range_to.end),
                Range::From(range_from) => process_array_from(target, vec, range_from.start),
                Range::Full => target.extend(vec),
            }
        }
        other => match range {
            Range::Index(index) => process_index(target, other, index),
            Range::FromTo(range) => process_from_to(target, other, range),
            Range::To(range_to) => process_to(target, other, range_to.end),
            Range::From(range_from) => process_from(target, other, range_from.start),
            Range::Full => target.push(other),
        },
    }
}

pub(super) fn default_value<'a>(value: &Value<'_>) -> Value<'a> {
    use simd_json::StaticNode;

    match value {
        Value::Static(StaticNode::I64(_)) => Value::Static(StaticNode::I64(0)),
        Value::Static(StaticNode::Bool(_)) => Value::Static(StaticNode::Bool(false)),
        Value::Static(StaticNode::U64(_)) => Value::Static(StaticNode::U64(0)),
        Value::Static(StaticNode::F64(_)) => Value::Static(StaticNode::F64(0.0)),
        Value::Static(StaticNode::Null) => Value::Static(StaticNode::Null),
        Value::String(_) => Value::String(Cow::Borrowed("")),
        Value::Array(_) => Value::Array(Box::new(Vec::new())),
        Value::Object(_) => Value::Object(Box::new(Object::new())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simd_json::{base::ValueTryAsArrayMut, json_typed};

    #[test]
    fn test_add_to_full_range() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_add(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::Full,
            json_typed!(borrowed, [4, 5]),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [1, 2, 3, 4, 5]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_add_to_index_range() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_add(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::Index(1),
            json_typed!(borrowed, 99),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [1, 99, 2, 3]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_add_to_range_from() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3, 4, 5]
        });

        handle_add(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::From(2..),
            json_typed!(borrowed, 99),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [1, 2, 99, 99, 99, 3, 4, 5]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_add_to_range_to() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_add(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::To(..2),
            json_typed!(borrowed, 99),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [99, 99, 1, 2, 3]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_add_to_range_to_from() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_add(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::FromTo(1..2),
            json_typed!(borrowed, 99),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [1, 99, 2, 3]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_add_to_empty_target() {
        let mut target = json_typed!(borrowed, {
            "array_key": []
        });

        handle_add(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::Full,
            json_typed!(borrowed, [4, 5]),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [4, 5]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_add_to_range_from_with_multiple_elements() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_add(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::From(4..),
            json_typed!(borrowed, 99),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [1, 2, 3, 0, 99]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_add_with_array_value() {
        let mut target = json_typed!(borrowed, [1, 2, 3]);

        handle_add(
            target.try_as_array_mut().unwrap(),
            Range::Full,
            json_typed!(borrowed, [99, 100]),
        );

        let expected = json_typed!(borrowed, [1, 2, 3, 99, 100]);
        assert_eq!(target, expected);
    }
}
