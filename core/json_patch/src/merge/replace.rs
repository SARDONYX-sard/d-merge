use super::PatchJson;
use crate::merge::error::{Error, Result};
use crate::ptr_mut::PointerMut as _;
use simd_json::BorrowedValue;

/// Replace one value.
///
/// # Note
/// - Support `Object` or `Array`
/// - Unsupported range remove. use `apply_range` instead
pub(crate) fn apply_replace<'a>(json: &mut BorrowedValue<'a>, patch: PatchJson<'a>) -> Result<()> {
    if let Some(target) = json.ptr_mut(&patch.path) {
        *target = patch.value;
        Ok(())
    } else {
        Err(Error::NotFoundTarget {
            path: patch.path.join("."),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merge::Op;
    use simd_json::json_typed;
    use std::borrow::Cow;

    #[test]
    fn replace_existing_key_in_object() {
        let mut target_json = json_typed!(borrowed, {
            "data": {
                "name": "John",
                "age": 30
            }
        });
        let patch = PatchJson {
            op: Op::Replace,
            path: vec![Cow::Borrowed("data"), Cow::Borrowed("name")],
            value: json_typed!(borrowed, "Jane"),
        };

        apply_replace(&mut target_json, patch).unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "data": {
                "name": "Jane",
                "age": 30
            }
        });
        assert_eq!(target_json, expected);
    }

    #[test]
    fn replace_existing_value_in_array() {
        let mut target_json = json_typed!(borrowed, {
            "items": [1, 2, 3]
        });
        let patch = PatchJson {
            op: Op::Replace,
            path: vec![Cow::Borrowed("items"), Cow::Borrowed("1")],
            value: json_typed!(borrowed, 99),
        };

        apply_replace(&mut target_json, patch).unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "items": [1, 99, 3]
        });
        assert_eq!(target_json, expected);
    }

    #[test]
    fn replace_top_level_key() {
        let mut target_json = json_typed!(borrowed, {
            "key1": "value1",
            "key2": "value2"
        });
        let patch = PatchJson {
            op: Op::Replace,
            path: vec![Cow::Borrowed("key1")],
            value: json_typed!(borrowed, "new_value1"),
        };

        apply_replace(&mut target_json, patch).unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "key1": "new_value1",
            "key2": "value2"
        });
        assert_eq!(target_json, expected);
    }

    #[test]
    fn replace_nonexistent_index_in_array() {
        let mut target_json = json_typed!(borrowed, {
            "data": [10, 20, 30]
        });
        let patch = PatchJson {
            op: Op::Replace,
            path: vec![Cow::Borrowed("data"), Cow::Borrowed("5")],
            value: json_typed!(borrowed, 99),
        };

        let result = apply_replace(&mut target_json, patch);

        assert_eq!(
            result,
            Err(Error::NotFoundTarget {
                path: "data.5".to_string()
            })
        );
    }

    #[test]
    fn replace_nested_object_key() {
        let mut target_json = json_typed!(borrowed, {
            "settings": {
                "theme": {
                    "color": "blue",
                    "font": "Arial"
                }
            }
        });
        let patch = PatchJson {
            op: Op::Replace,
            path: vec![
                Cow::Borrowed("settings"),
                Cow::Borrowed("theme"),
                Cow::Borrowed("color"),
            ],
            value: json_typed!(borrowed, "red"),
        };

        apply_replace(&mut target_json, patch).unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "settings": {
                "theme": {
                    "color": "red",
                    "font": "Arial"
                }
            }
        });
        assert_eq!(target_json, expected);
    }

    #[test]
    fn replace_whole_array() {
        let mut target_json = json_typed!(borrowed, {
            "data": [1, 2, 3]
        });
        let patch = PatchJson {
            op: Op::Replace,
            path: vec![Cow::Borrowed("data")],
            value: json_typed!(borrowed, [10, 20]),
        };

        apply_replace(&mut target_json, patch).unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "data": [10, 20]
        });
        assert_eq!(target_json, expected);
    }

    #[test]
    fn replace_nested_array_value() {
        let mut target_json = json_typed!(borrowed, {
            "nested": {
                "list": [1, 2, 3]
            }
        });
        let patch = PatchJson {
            op: Op::Replace,
            path: vec![
                Cow::Borrowed("nested"),
                Cow::Borrowed("list"),
                Cow::Borrowed("2"),
            ],
            value: json_typed!(borrowed, 99),
        };

        apply_replace(&mut target_json, patch).unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "nested": {
                "list": [1, 2, 99]
            }
        });
        assert_eq!(target_json, expected);
    }

    #[test]
    fn should_fail_replace_nonexistent_key_in_object() {
        let mut target_json = json_typed!(borrowed, {
            "data": []
        });
        let patch = PatchJson {
            op: Op::Replace,
            path: vec![Cow::Borrowed("address"), Cow::Borrowed("zip")],
            value: json_typed!(borrowed, "12345"),
        };

        let result = apply_replace(&mut target_json, patch);

        assert_eq!(
            result,
            Err(Error::NotFoundTarget {
                path: "address.zip".to_string()
            })
        );
    }
}
