use crate::range::Range;
use simd_json::borrowed::{Object, Value};
use std::{iter::repeat, ops::RangeFrom};

/// Add `value` to the `range` portion of `target`.
///
/// # Note
/// - If `range.start` is out of bounds, then extend by default value.
pub fn handle_add<'value>(target: &mut Vec<Value<'value>>, range: Range, value: Value<'value>) {
    match value {
        Value::Array(vec) => target.extend(*vec), // TODO: separate fn & file. And fix array range patterns.
        other => match range {
            Range::Index(index) => {
                if index >= target.len() {
                    // Extend with default values if out-of-range
                    target.extend(repeat(default_value(&other)).take(index - target.len()));
                }
                target.insert(index, other);
            }
            Range::FromTo(std::ops::Range { start, end }) => {
                let target_len = target.len();

                if start >= target_len {
                    target.extend(repeat(default_value(&other)).take(start - target.len()));
                }
                if end > target_len {
                    target.extend(repeat(default_value(&other)).take(end - target.len()));
                }
                let (prefix, suffix) = target.split_at_mut(start);
                let insert_count = end - start;

                let mut new_target = Vec::with_capacity(target_len + insert_count);
                new_target.extend_from_slice(prefix);
                new_target.extend(repeat(other).take(insert_count));
                new_target.extend_from_slice(suffix);

                *target = new_target;
            }
            Range::To(range_to) => {
                if range_to.end > target.len() {
                    target.extend(repeat(default_value(&other)).take(range_to.end - target.len()));
                }
                let mut new_target: Vec<_> = repeat(other).take(range_to.end).collect();
                new_target.extend(core::mem::take(target));
                *target = new_target;
            }
            Range::From(RangeFrom { start }) => {
                let target_len = target.len();
                let insert_count = match start {
                    start if start == target_len + 1 => {
                        target.push(other); // There is only one element, and Add after len means push.
                        return;
                    }
                    start if start > target_len => start - target_len,
                    start if start < target_len => target_len - start,
                    _ => return,
                };

                if insert_count > target_len {
                    target.extend(repeat(default_value(&other)).take(insert_count));
                }
                target.splice(start.., repeat(other).take(insert_count));
            }
            Range::Full => target.push(other),
        },
    }
}

fn default_value<'a>(value: &Value<'_>) -> Value<'a> {
    use simd_json::{StaticNode, ValueBuilder as _};

    match value {
        Value::Static(StaticNode::I64(_)) => Value::Static(StaticNode::I64(0)),
        Value::Static(StaticNode::Bool(_)) => Value::Static(StaticNode::Bool(false)),
        Value::Static(StaticNode::U64(_)) => Value::Static(StaticNode::U64(0)),
        Value::Static(StaticNode::F64(_)) => Value::Static(StaticNode::F64(0.0)),
        Value::Static(StaticNode::Null) => Value::null(),
        Value::String(_) => Value::from(""),
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
            "array_key": [1, 2, 99, 99, 99]
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
            "array_key": [1, 2, 3, 99]
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
