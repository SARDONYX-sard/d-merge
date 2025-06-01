use crate::apply::error::{JsonPatchError, Result};
use crate::JsonPath;
use simd_json::borrowed::Value;
use simd_json::derived::ValueTryAsScalar as _;
use simd_json::StaticNode;

/// Adds a new key (for objects) or a new index (for arrays) if they don't exist.
///
/// # Errors
/// If failed to cast.
///
/// # Note
/// - Support `Object` or `Array`
/// - Unsupported range remove. use `apply_range` instead.
pub(crate) fn apply_add<'value>(
    json: &mut Value<'value>,
    path: JsonPath<'value>,
    value: Value<'value>,
) -> Result<()> {
    if path.is_empty() {
        return Err(JsonPatchError::empty_pointer_from(&path, &value));
    }
    let last_index = path.len() - 1;

    let mut target = json;
    for (i, token) in path.into_iter().enumerate() {
        match target {
            Value::Object(ref mut map) => {
                if i == last_index {
                    map.insert(token, value);
                    return Ok(());
                } else {
                    // Ensure the key exists, or create a new nested object
                    target = map
                        .entry(token.clone())
                        .or_insert_with(|| Value::Object(Default::default()));
                }
            }
            Value::Array(ref mut list) => {
                if let Ok(index) = token.parse::<usize>() {
                    while list.len() <= index {
                        list.push(Default::default()); // Push a placeholder to extend the array
                    }
                    if i == last_index {
                        list[index] = value; // `value` is moved here, only once
                        return Ok(());
                    } else {
                        target = &mut list[index];
                    }
                } else {
                    return Err(JsonPatchError::invalid_index_from(
                        last_index,
                        &[token],
                        &value,
                    ));
                }
            }
            Value::String(ref mut s) => {
                if i == last_index {
                    match value {
                        Value::String(s2) => {
                            *s = s2;
                            return Ok(());
                        }
                        _ => return Err(JsonPatchError::invalid_string_from(&[token], &value)),
                    }
                } else {
                    return Err(JsonPatchError::invalid_string_from(&[token], &value));
                    // Can't go deeper in a String
                }
            }
            Value::Static(ref mut static_node) => {
                if i == last_index {
                    return {
                        macro_rules! try_insert {
                            ($n:ident, $try_exp:expr) => {
                                match $try_exp {
                                    Ok(v) => *$n = v,
                                    Err(err) => {
                                        return Err(JsonPatchError::try_type_from(
                                            err,
                                            &[token],
                                            &value,
                                        ))
                                    }
                                }
                            };
                        }

                        match static_node {
                            StaticNode::I64(n) => try_insert!(n, value.try_as_i64()),
                            StaticNode::U64(n) => try_insert!(n, value.try_as_u64()),
                            StaticNode::F64(n) => try_insert!(n, value.try_as_f64()),
                            StaticNode::Bool(n) => try_insert!(n, value.try_as_bool()),
                            StaticNode::Null => {}
                        };
                        Ok(())
                    };
                } else {
                    return Err(JsonPatchError::invalid_target_from(&[token], &value));
                    // Can't go deeper in a static node
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_path;
    use simd_json::{json_typed, value::StaticNode};
    use std::borrow::Cow;

    #[test]
    fn add_to_object() {
        let mut target = json_typed!(borrowed, {
            "name": "John",
            "age": 30
        });

        apply_add(
            &mut target,
            json_path!("address"),
            Value::String(Cow::Borrowed("123 Main St")),
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(target["address"], "123 Main St");
    }

    #[test]
    fn add_to_nested_object() {
        let mut target = json_typed!(borrowed, {
            "user": {
                "name": "John",
                "age": 30
            }
        });

        let value = Value::String(Cow::Borrowed("123 Main St"));
        apply_add(&mut target, json_path!("user", "address"), value)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(target["user"]["address"], "123 Main St");
    }

    #[test]
    fn add_to_array() {
        let mut target = json_typed!(borrowed, {
            "items": [1, 2, 3]
        });
        let value = Value::Static(StaticNode::U64(4));
        apply_add(&mut target, json_path!("items", "3"), value)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(target["items"][3], 4);
    }

    #[test]
    fn should_add_to_nested_array() {
        let mut target = json_typed!(borrowed, {
            "data": {
                "items": [1, 2, 3]
            }
        });

        let value = Value::Static(StaticNode::U64(4));
        apply_add(&mut target, json_path!("data", "items", "3"), value)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(target["data"]["items"][3], 4);
    }

    #[test]
    fn should_add_new_key_to_object() {
        let mut target = json_typed!(borrowed, {
            "existing_key": "existing_value"
        });

        let value = "new_value".into();
        apply_add(&mut target, json_path!("new_key"), value).unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "existing_key": "existing_value",
            "new_key": "new_value"
        });
        assert_eq!(target, expected);
    }
}
