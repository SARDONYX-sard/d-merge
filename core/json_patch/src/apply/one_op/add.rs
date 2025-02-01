use crate::apply::error::{JsonPatchError, Result};
use crate::JsonPatch;
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
pub(crate) fn apply_add<'value>(json: &mut Value<'value>, patch: JsonPatch<'value>) -> Result<()> {
    let JsonPatch { path, value, .. } = patch;

    if path.is_empty() {
        return Err(JsonPatchError::EmptyPointer);
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
                    return Err(JsonPatchError::InvalidIndex {
                        index: token.to_string(),
                    });
                }
            }
            Value::String(ref mut s) => {
                if i == last_index {
                    match value {
                        Value::String(s2) => {
                            *s = s2;
                            return Ok(());
                        }
                        _ => return Err(JsonPatchError::InvalidString),
                    }
                } else {
                    return Err(JsonPatchError::InvalidString); // Can't go deeper in a String
                }
            }
            Value::Static(ref mut static_node) => {
                if i == last_index {
                    return {
                        match static_node {
                            StaticNode::I64(n) => *n = value.try_as_i64()?,
                            StaticNode::U64(n) => *n = value.try_as_u64()?,
                            StaticNode::F64(n) => *n = value.try_as_f64()?,
                            StaticNode::Bool(b) => *b = value.try_as_bool()?,
                            StaticNode::Null => {}
                        };
                        Ok(())
                    };
                } else {
                    return Err(JsonPatchError::InvalidTarget); // Can't go deeper in a static node
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply::Op;
    use simd_json::{json_typed, value::StaticNode};
    use std::borrow::Cow;

    #[test]
    fn add_to_object() {
        let mut target = json_typed!(borrowed, {
            "name": "John",
            "age": 30
        });
        let patch = JsonPatch {
            op: Op::Add,
            path: vec![Cow::Borrowed("address")],
            value: Value::String(Cow::Borrowed("123 Main St")),
            ..Default::default()
        };

        apply_add(&mut target, patch).unwrap_or_else(|err| panic!("{err}"));

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
        let patch = JsonPatch {
            op: Op::Add,
            path: vec![Cow::Borrowed("user"), Cow::Borrowed("address")],
            value: Value::String(Cow::Borrowed("123 Main St")),
            ..Default::default()
        };

        apply_add(&mut target, patch).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(target["user"]["address"], "123 Main St");
    }

    #[test]
    fn add_to_array() {
        let mut target = json_typed!(borrowed, {
            "items": [1, 2, 3]
        });
        let patch = JsonPatch {
            op: Op::Add,
            path: vec![Cow::Borrowed("items"), Cow::Borrowed("3")],
            value: Value::Static(StaticNode::U64(4)),
            ..Default::default()
        };

        apply_add(&mut target, patch).unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(target["items"][3], 4);
    }

    #[test]
    fn should_add_to_nested_array() {
        let mut target = json_typed!(borrowed, {
            "data": {
                "items": [1, 2, 3]
            }
        });
        let patch = JsonPatch {
            op: Op::Add,
            path: vec![
                Cow::Borrowed("data"),
                Cow::Borrowed("items"),
                Cow::Borrowed("3"),
            ],
            value: Value::Static(StaticNode::U64(4)),
            ..Default::default()
        };

        apply_add(&mut target, patch).unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(target["data"]["items"][3], 4);
    }

    #[test]
    fn should_add_new_key_to_object() {
        let mut target = json_typed!(borrowed, {
            "existing_key": "existing_value"
        });
        let patch = JsonPatch {
            op: Op::Add,
            path: ["new_key".into()].to_vec(),
            value: "new_value".into(),
            ..Default::default()
        };

        apply_add(&mut target, patch).unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "existing_key": "existing_value",
            "new_key": "new_value"
        });
        assert_eq!(target, expected);
    }
}
