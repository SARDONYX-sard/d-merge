use crate::apply::error::{JsonPatchError, Result};
use crate::JsonPatch;
use simd_json::borrowed::Value;
use std::borrow::Cow;

/// Remove one value.
///
/// # Note
/// - Support `Object` or `Array`
/// - Unsupported range remove. use `apply_range` instead
pub(crate) fn apply_remove<'a>(json: &mut Value<'a>, patch: JsonPatch<'a>) -> Result<()> {
    remove(json, &patch.path).ok_or_else(|| JsonPatchError::NotFoundTarget {
        path: patch.path.join("."),
    })?;
    Ok(())
}

/// Removes the value at the given `ptr` path and returns it, if it exists.
fn remove<'value>(target: &mut Value<'value>, path: &[Cow<'value, str>]) -> Option<Value<'value>> {
    if path.is_empty() {
        return None;
    }

    let mut path = path.iter();
    let last = path.next_back()?;

    // Special case: If there's only one element in the path, remove directly from the top-level target.
    if path.len() == 0 {
        return match target {
            Value::Object(map) => map.remove(last),
            Value::Array(list) => {
                let index = last.parse::<usize>().ok()?;
                if index < list.len() {
                    Some(list.remove(index))
                } else {
                    None
                }
            }
            _ => None,
        };
    }

    // Navigate to the second-to-last element in the path
    let parent = path.try_fold(target, |target, token| match target {
        Value::Object(ref mut map) => map.get_mut(token),
        Value::Array(list) => list.get_mut(last.parse::<usize>().ok()?),
        _ => None,
    })?;

    // Remove the value from the parent
    match parent {
        Value::Object(map) => map.remove(last),
        Value::Array(list) => {
            let index = last.parse::<usize>().ok()?;
            if index < list.len() {
                Some(list.remove(index))
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply::Op;
    use simd_json::{json_typed, StaticNode};
    use std::borrow::Cow;

    #[test]
    fn remove_object_key() {
        let mut target_json = json_typed!(borrowed, {
            "items": {
                "key1": 1,
                "key2": 2,
                "key3": 3
            }
        });

        let patch = JsonPatch {
            op: Op::Remove,
            path: vec![Cow::Borrowed("items"), Cow::Borrowed("key2")],
            value: Value::Static(StaticNode::Null),
            ..Default::default()
        };

        apply_remove(&mut target_json, patch).unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "items": {
                "key1": 1,
                "key3": 3
            }
        });
        assert_eq!(target_json, expected);
    }

    #[test]
    fn remove_array() {
        let mut target_json = json_typed!(borrowed, {
            "data": [1, 2, 3]
        });

        let patch = JsonPatch {
            op: Op::Remove,
            path: vec![Cow::Borrowed("data")],
            value: Value::Static(StaticNode::Null),
            ..Default::default()
        };

        apply_remove(&mut target_json, patch).unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {});
        assert_eq!(target_json, expected);
    }

    #[test]
    fn remove_object() {
        let mut target_json = json_typed!(borrowed, {
            "settings": {
                "option1": true,
                "option2": false
            }
        });

        let patch = JsonPatch {
            op: Op::Remove,
            path: vec![Cow::Borrowed("settings")],
            value: Value::Static(StaticNode::Null),
            ..Default::default()
        };

        apply_remove(&mut target_json, patch).unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {});
        assert_eq!(target_json, expected);
    }

    #[test]
    fn remove_top_level_key() {
        let mut target_json = json_typed!(borrowed, {
            "key1": 123,
            "key2": "value"
        });

        let patch = JsonPatch {
            op: Op::Remove,
            path: vec![Cow::Borrowed("key1")],
            value: Value::Static(StaticNode::Null),
            ..Default::default()
        };

        apply_remove(&mut target_json, patch).unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "key2": "value"
        });
        assert_eq!(target_json, expected);
    }

    #[test]
    fn should_fail_remove_nonexistent_key_in_object() {
        let mut target_json = json_typed!(borrowed, {
            "items": {
                "key1": 1,
                "key2": 2
            }
        });

        let patch = JsonPatch {
            op: Op::Remove,
            path: vec![Cow::Borrowed("items"), Cow::Borrowed("key3")],
            value: Value::Static(StaticNode::Null),
            ..Default::default()
        };

        let result = apply_remove(&mut target_json, patch);

        let expected = Err(JsonPatchError::NotFoundTarget {
            path: "items.key3".to_string(),
        });
        assert_eq!(result, expected);
    }

    #[test]
    fn should_fail_remove_nonexistent_array() {
        let mut target_json = json_typed!(borrowed, {
            "data": []
        });

        let patch = JsonPatch {
            op: Op::Remove,
            path: vec![Cow::Borrowed("missing")],
            value: Value::Static(StaticNode::Null),
            ..Default::default()
        };

        let result = apply_remove(&mut target_json, patch);

        assert_eq!(
            result,
            Err(JsonPatchError::NotFoundTarget {
                path: "missing".to_string()
            })
        );
    }
}
