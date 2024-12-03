use crate::merge::range::Range;
use simd_json::borrowed::Value;
use std::iter::repeat;

/// Replace `value` to the `range` portion of `target`.
///
/// # Panics
/// - Panics if `range` is out of bounds.
pub fn handle_replace<'value>(target: &mut Vec<Value<'value>>, range: Range, value: Value<'value>) {
    match value {
        Value::Array(vec) => {
            let vec = *vec;
            match range {
                Range::Index(index) => {
                    if let Some(value) = target.get_mut(index) {
                        *value = vec.into();
                    }
                }
                Range::FromTo(range) => {
                    target.splice(range, vec);
                }
                Range::To(range_to) => {
                    target.splice(range_to, vec);
                }
                Range::From(range_from) => {
                    target.splice(range_from, vec);
                }
                Range::Full => *target = vec,
            }
        }
        other => match range {
            Range::Index(index) => {
                if let Some(value) = target.get_mut(index) {
                    *value = other;
                }
            }
            Range::FromTo(range) => {
                target.splice(range.clone(), repeat(other).take(range.count()));
            }
            Range::To(range_to) => {
                target.splice(range_to, repeat(other).take(range_to.end));
            }
            Range::From(range_from) => {
                let replace_count = target.len() - range_from.start;
                target.splice(range_from, repeat(other).take(replace_count));
            }
            Range::Full => *target = repeat(other).take(target.len()).collect(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merge::range::Range;
    use simd_json::{base::ValueTryAsArrayMut as _, json_typed};

    #[test]
    fn test_replace_full_range() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_replace(
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
    fn test_replace_index_range() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_replace(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::Index(1),
            json_typed!(borrowed, 99),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [1, 99, 3]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_replace_to_range() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_replace(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::To(..2),
            json_typed!(borrowed, 99),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [99, 99, 3]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_replace_from_range() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_replace(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::From(1..),
            json_typed!(borrowed, 99),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [1, 99, 99]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_replace_to_from_range() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_replace(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::FromTo(1..2),
            json_typed!(borrowed, 99),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [1, 99, 3]
        });
        assert_eq!(target, expected);
    }

    // Additional array pattern tests

    #[test]
    fn test_replace_with_multiple_elements() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_replace(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::To(..2),
            json_typed!(borrowed, [99, 100]),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [99, 100, 3]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_replace_insert_at_index() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_replace(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::Index(1),
            json_typed!(borrowed, [99]),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [1, [99], 3]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_replace_empty_range() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_replace(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::FromTo(0..0), // Empty range
            json_typed!(borrowed, [99]),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [99, 1, 2, 3]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_replace_range_that_fits_exactly() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3, 4]
        });

        handle_replace(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::FromTo(1..3),
            json_typed!(borrowed, [99, 100]),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [1, 99, 100, 4]
        });
        assert_eq!(target, expected);
    }

    // New test for replacing with an array value

    #[test]
    fn test_replace_with_array_value() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_replace(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::To(..2),
            json_typed!(borrowed, [99, 100]),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [99, 100, 3]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_replace_with_array_value_at_index() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_replace(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::Index(1),
            json_typed!(borrowed, [99, 100]),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [1, [99, 100], 3]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_replace_with_array_value_full_range() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        handle_replace(
            target["array_key"].try_as_array_mut().unwrap(),
            Range::Full,
            json_typed!(borrowed, [99, 100, 101]),
        );

        let expected = json_typed!(borrowed, {
            "array_key": [99, 100, 101]
        });
        assert_eq!(target, expected);
    }
}
