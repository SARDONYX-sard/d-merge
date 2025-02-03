use crate::apply::error::{JsonPatchError, Result};
use crate::ptr_mut::PointerMut as _;
use crate::JsonPath;
use simd_json::borrowed::Value;

/// Replace one value.
///
/// # Note
/// - Support `Object` or `Array`
/// - Unsupported range remove. use `apply_range` instead
pub(crate) fn apply_replace<'a>(
    json: &mut Value<'a>,
    path: JsonPath<'a>,
    value: Value<'a>,
) -> Result<()> {
    json.ptr_mut(&path).map_or_else(
        || {
            Err(JsonPatchError::NotFoundTarget {
                path: path.join("."),
            })
        },
        |target| {
            *target = value;
            Ok(())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_path;
    use simd_json::json_typed;

    #[test]
    fn replace_existing_key_in_object() {
        let mut target_json = json_typed!(borrowed, {
            "data": {
                "name": "John",
                "age": 30
            }
        });

        let path = json_path!["data", "name"];
        let value = json_typed!(borrowed, "Jane");

        apply_replace(&mut target_json, path, value).unwrap_or_else(|err| panic!("{err}"));

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

        let path = json_path!["items", "[1]"];
        let value = json_typed!(borrowed, 99);
        apply_replace(&mut target_json, path, value).unwrap_or_else(|err| panic!("{err}"));

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

        let path = json_path!["key1"];
        let value = json_typed!(borrowed, "new_value1");
        apply_replace(&mut target_json, path, value).unwrap_or_else(|err| panic!("{err}"));

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

        let path = json_path!["data", "5"];
        let value = json_typed!(borrowed, 99);
        let result = apply_replace(&mut target_json, path, value);

        assert_eq!(
            result,
            Err(JsonPatchError::NotFoundTarget {
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

        let path = json_path!["settings", "theme", "color"];
        let value = json_typed!(borrowed, "red");
        apply_replace(&mut target_json, path, value).unwrap_or_else(|err| panic!("{err}"));

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
        let path = json_path!["data"];
        let value = json_typed!(borrowed, [10, 20]);
        apply_replace(&mut target_json, path, value).unwrap_or_else(|err| panic!("{err}"));

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

        let path = json_path!["nested", "list", "[2]"];
        let value = json_typed!(borrowed, 99);
        apply_replace(&mut target_json, path, value).unwrap_or_else(|err| panic!("{err}"));

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

        let path = json_path!["address", "zip"];
        let value = json_typed!(borrowed, 12345);
        let result = apply_replace(&mut target_json, path, value);

        assert_eq!(
            result,
            Err(JsonPatchError::NotFoundTarget {
                path: "address.zip".to_string()
            })
        );
    }
}
