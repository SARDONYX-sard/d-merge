use simd_json::{derived::ValueTryAsScalar as _, BorrowedValue, TryTypeError};
use snafu::Snafu;
use std::borrow::Cow;

#[derive(Snafu, Debug, Clone)]
pub enum Error {
    #[snafu(display("Pointer is empty, cannot add"))]
    EmptyPointer,

    #[snafu(display("Invalid index: {}", index))]
    InvalidIndex { index: String },

    #[snafu(display("Cannot go deeper in a String"))]
    InvalidString,

    /// Can't go deeper in a static node
    InvalidTarget,

    #[snafu(transparent)]
    TryType { source: TryTypeError },
}

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
pub trait PointerMut<'v> {
    type Error;

    /// Given a path represented by `pointer`, attempts to navigate
    /// through a `BorrowedValue` structure and return a mutable reference
    /// to the target value.
    ///
    /// # Returns
    /// - `Some(&mut BorrowedValue)` if the path is found and points to a valid
    ///   mutable value.
    /// - `None` if the path is not valid or the value at the path cannot be mutated.
    fn ptr_mut(&mut self, ptr: &[Cow<'v, str>]) -> Option<&mut BorrowedValue<'v>>;

    /// Adds a new key (for objects) or a new index (for arrays) if they don't exist.
    fn push_by(
        &mut self,
        ptr: Vec<Cow<'v, str>>,
        value: BorrowedValue<'v>,
    ) -> Result<(), Self::Error>;
}

impl<'v> PointerMut<'v> for BorrowedValue<'v> {
    type Error = Error;

    fn ptr_mut(&mut self, ptr: &[Cow<'v, str>]) -> Option<&mut Self> {
        if ptr.is_empty() {
            return Some(self);
        }

        ptr.iter()
            .try_fold(self, move |target, token| match target {
                BorrowedValue::Object(map) => map.get_mut(token),
                BorrowedValue::Array(list) => {
                    fn parse_index(index: &str) -> Option<usize> {
                        index.parse().ok()
                    }
                    parse_index(token).and_then(move |x| list.get_mut(x))
                }
                _ => None,
            })
    }

    fn push_by(&mut self, ptr: Vec<Cow<'v, str>>, value: Self) -> Result<(), Error> {
        if ptr.is_empty() {
            return Err(Error::EmptyPointer);
        }
        let ptr_len = ptr.len();

        let mut target = self;
        for (i, token) in ptr.into_iter().enumerate() {
            match target {
                BorrowedValue::Object(ref mut map) => {
                    if i == ptr_len - 1 {
                        // Insert the final key-value pair
                        map.insert(token, value); // `value` is moved here, only once
                        return Ok(());
                    } else {
                        // Ensure the key exists, or create a new nested object
                        target = map
                            .entry(token.clone())
                            .or_insert_with(|| BorrowedValue::Object(Default::default()));
                    }
                }
                BorrowedValue::Array(ref mut list) => {
                    if let Ok(index) = token.parse::<usize>() {
                        while list.len() <= index {
                            list.push(Default::default()); // Push a placeholder to extend the array
                        }
                        if i == ptr_len - 1 {
                            list[index] = value; // `value` is moved here, only once
                            return Ok(());
                        } else {
                            target = &mut list[index];
                        }
                    } else {
                        return Err(Error::InvalidIndex {
                            index: token.to_string(),
                        });
                    }
                }
                BorrowedValue::String(ref mut s) => {
                    if i == ptr_len - 1 {
                        match value {
                            BorrowedValue::String(s2) => {
                                *s = s2;
                                return Ok(());
                            }
                            _ => return Err(Error::InvalidString),
                        }
                    } else {
                        return Err(Error::InvalidString); // Can't go deeper in a String
                    }
                }
                BorrowedValue::Static(ref mut static_node) => {
                    if i == ptr_len - 1 {
                        match static_node {
                            simd_json::StaticNode::I64(n) => *n = value.try_as_i64()?,
                            simd_json::StaticNode::U64(n) => *n = value.try_as_u64()?,
                            simd_json::StaticNode::F64(n) => *n = value.try_as_f64()?,
                            simd_json::StaticNode::Bool(b) => *b = value.try_as_bool()?,
                            simd_json::StaticNode::Null => {}
                        }
                        return Ok(());
                    } else {
                        return Err(Error::InvalidTarget); // Can't go deeper in a static node
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simd_json::json_typed;

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

    #[test]
    fn test_add_key_to_object() {
        let mut value = json_typed!(borrowed, {
            "existing_key": "existing_value"
        });
        let pointer = ["new_key".into()].to_vec();

        value
            .push_by(pointer, "new_value".into())
            .unwrap_or_else(|err| panic!("{err}"));

        // Expected result: The new key-value pair should be added
        let expected = json_typed!(borrowed, {
            "existing_key": "existing_value",
            "new_key": "new_value"
        });

        assert_eq!(value, expected);
    }

    #[test]
    fn test_add_element_to_array() {
        let mut value = json_typed!(borrowed, {
            "array_key": ["item_0", "item_1"]
        });
        let pointer = ["array_key".into(), "2".into()].to_vec();

        value
            .push_by(pointer, "item_2".into())
            .unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "array_key": ["item_0", "item_1", "item_2"]
        });

        assert_eq!(value, expected);
    }
}
