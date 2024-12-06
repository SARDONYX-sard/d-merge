use simd_json::BorrowedValue;

/// A trait that provides a mutable reference to a `BorrowedValue`
/// given a sequence of strings (representing the path or pointer).
///
/// This trait is useful for navigating and mutating nested structures
/// (e.g., `Object` or `Array`) within a `BorrowedValue`.
///
/// # Associated Methods
/// - `pointer_mut`: Takes a slice of `Cow<'v, str>` representing a path,
///   and attempts to traverse through the structure, returning a mutable
///   reference to the target value if found.
pub trait PointerMut<'value> {
    /// Given a path represented by `pointer`, attempts to navigate
    /// through a `BorrowedValue` structure and return a mutable reference
    /// to the target value.
    ///
    /// # Returns
    /// - `Some(&mut BorrowedValue)` if the path is found and points to a valid
    ///   mutable value.
    /// - `None` if the path is not valid or the value at the path cannot be mutated.
    fn ptr_mut<I>(&mut self, ptr: I) -> Option<&mut BorrowedValue<'value>>
    where
        I: IntoIterator,
        I::Item: AsRef<str>;
}

impl<'value> PointerMut<'value> for BorrowedValue<'value> {
    fn ptr_mut<I>(&mut self, ptr: I) -> Option<&mut Self>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        ptr.into_iter()
            .try_fold(self, move |target, token| match target {
                BorrowedValue::Object(map) => map.get_mut(token.as_ref()),
                BorrowedValue::Array(list) => {
                    let n = token.as_ref().parse::<usize>().ok()?;
                    list.get_mut(n)
                }
                _ => None,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simd_json::json_typed;
    use std::borrow::Cow;

    // Test: Nested JSON and Array Search and Mutation
    #[test]
    fn test_pointer_mut_nested_json() {
        let mut value: BorrowedValue = json_typed!(borrowed, {
            "outer_key": {
                "inner_key": "inner_value"
            }
        });
        let pointer: &[Cow<str>] = &["outer_key".into(), "inner_key".into()];

        if let Some(val) = value.ptr_mut(pointer) {
            if let BorrowedValue::String(ref mut s) = val {
                *s = Cow::Borrowed("modified_inner_value");
            }
        } else {
            panic!("Expected a mutable reference");
        }

        // Verify modification
        let expected = json_typed!(borrowed, {
            "outer_key": {
                "inner_key": "modified_inner_value"
            }
        });

        assert_eq!(value, expected);
    }

    #[test]
    fn test_pointer_mut_array_in_object() {
        let mut value = json_typed!(borrowed, {
            "array_key": ["first", "second"]
        });
        let pointer: &[Cow<str>] = &["array_key".into(), "1".into()];

        if let Some(val) = value.ptr_mut(pointer) {
            if let BorrowedValue::String(ref mut s) = val {
                *s = Cow::Borrowed("modified_second");
            }
        } else {
            panic!("Expected a mutable reference");
        }

        // Verify modification
        let expected = json_typed!(borrowed, {
            "array_key": ["first", "modified_second"]
        });

        assert_eq!(value, expected);
    }

    #[test]
    fn test_pointer_mut_complex_nested() {
        let mut value = json_typed!(borrowed, {
            "object_key": {
                "array_key": ["item_0", "item_1"]
            }
        });
        let pointer: &[Cow<str>] = &["object_key".into(), "array_key".into(), "1".into()];

        if let Some(val) = value.ptr_mut(pointer) {
            if let BorrowedValue::String(ref mut s) = val {
                *s = Cow::Borrowed("modified_item_1");
            }
        } else {
            panic!("Expected a mutable reference");
        }

        let expected = json_typed!(borrowed, {
            "object_key": {
                "array_key": ["item_0", "modified_item_1"]
            }
        });

        assert_eq!(value, expected);
    }

    #[test]
    fn test_pointer_mut_invalid_pointer() {
        let mut value = json_typed!(borrowed, {
            "existing_key": "value"
        });
        let pointer: &[Cow<str>] = &["non_existent_key".into()];
        dbg!(&pointer);
        assert!(value.ptr_mut(pointer).is_none());
    }

    // Test: Array index out of bounds
    #[test]
    fn test_pointer_mut_array_out_of_bounds() {
        let mut value = json_typed!(borrowed, {
            "array_key": ["first", "second"]
        });
        let pointer: &[Cow<str>] = &["array_key".into(), "2".into()];

        assert!(value.ptr_mut(pointer).is_none());
    }

    #[test]
    fn test_pointer_mut_object_but_points_to_array() {
        let mut value = json_typed!(borrowed, {
            "array_key": ["item_0", "item_1"]
        });
        let pointer: &[Cow<str>] = &["array_key".into(), "1".into()];

        let val = value
            .ptr_mut(pointer)
            .unwrap_or_else(|| panic!("Expected a mutable reference"));
        if let BorrowedValue::String(ref mut s) = val {
            *s = Cow::Borrowed("modified_item_1");
        }

        let expected = json_typed!(borrowed, {
            "array_key": ["item_0", "modified_item_1"]
        });

        assert_eq!(value, expected);
    }
}
