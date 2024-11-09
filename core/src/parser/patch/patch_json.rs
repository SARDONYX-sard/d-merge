use super::merge::searcher::PointerMut as _;
use simd_json::{
    borrowed::{Array, Object},
    BorrowedValue,
};
use snafu::Snafu;
use std::borrow::Cow;

/// Custom error type for JSON patch operations.
#[derive(Debug, Snafu)]
pub enum PatchError {
    /// Error indicating that the specified path was not found in the JSON structure.
    #[snafu(display("Path not found: {}", path))]
    PathNotFound { path: String },

    /// Error indicating an invalid operation at the given path.
    #[snafu(display("Invalid operation for path: {}", path))]
    InvalidOperation { path: String },

    #[snafu(transparent)]
    AccessError { source: simd_json::AccessError },

    #[snafu(transparent)]
    TryTypeError { source: simd_json::TryTypeError },

    #[snafu(transparent)]
    SearchedError {
        source: super::merge::searcher::Error,
    },
}

/// Result type alias for JSON patch operations.
type Result<T, E = PatchError> = std::result::Result<T, E>;

/// Enum representing the type of operation for the JSON patch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    /// Add a new value to the JSON at the specified path.
    Add,
    /// Remove the value from the JSON at the specified path.
    Remove,
    /// Replace the value at the specified path with a new value.
    Replace,
}

/// Struct representing a JSON patch operation.
#[derive(Debug, Clone, PartialEq)]
pub struct PatchJson<'a> {
    /// The type of operation to perform (Add, Remove, Replace).
    pub op: Op,
    /// A vector representing the path in the JSON where the operation is applied.
    ///
    /// $(root), index, className, fieldName
    /// - e.g. "$.4514.hkbStateMachineStateInfo.generator",
    /// - e.g. "$.1.hkRootLevelContainer.namedVariants[0]",
    pub path: Vec<Cow<'a, str>>,
    /// The value to be added or replaced in the JSON.
    pub value: BorrowedValue<'a>,
}

/// # Errors
/// Failed to parse
pub fn apply_patch<'v>(json: &mut BorrowedValue<'v>, patch: PatchJson<'v>) -> Result<()> {
    match patch.op {
        Op::Add => apply_add(json, patch),
        Op::Remove => apply_remove(json, patch),
        Op::Replace => apply_replace(json, patch),
    }
}

fn apply_add<'v>(json: &mut BorrowedValue<'v>, patch: PatchJson<'v>) -> Result<()> {
    json.push_by(patch.path, patch.value)?;
    Ok(())
}

fn apply_remove<'a>(json: &mut BorrowedValue<'a>, patch: PatchJson<'a>) -> Result<()> {
    if let Some(target) = json.ptr_mut(&patch.path) {
        match target {
            BorrowedValue::Object(map) => remove_from_object(map, &patch),
            BorrowedValue::Array(list) => remove_from_array(list, patch),
            _ => Err(PatchError::InvalidOperation {
                path: patch.path.join("."),
            }),
        }
    } else {
        Err(PatchError::PathNotFound {
            path: patch.path.join("."),
        })
    }
}

fn remove_from_object<'a>(map: &mut Object<'a>, patch: &PatchJson<'a>) -> Result<()> {
    let key = patch.path.last().unwrap();
    if map.remove(key).is_none() {
        return Err(PatchError::PathNotFound {
            path: patch.path.join("."),
        });
    }
    Ok(())
}

fn remove_from_array<'a>(list: &mut Array<'a>, patch: PatchJson<'a>) -> Result<()> {
    if let Ok(index) = patch.path.last().unwrap().parse::<usize>() {
        if index < list.len() {
            list.remove(index);
            Ok(())
        } else {
            Err(PatchError::PathNotFound {
                path: patch.path.join("."),
            })
        }
    } else {
        Err(PatchError::InvalidOperation {
            path: patch.path.join("."),
        })
    }
}

fn apply_replace<'a>(json: &mut BorrowedValue<'a>, patch: PatchJson<'a>) -> Result<()> {
    if let Some(target) = json.ptr_mut(&patch.path) {
        *target = patch.value;
        Ok(())
    } else {
        Err(PatchError::PathNotFound {
            path: patch.path.join("."),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simd_json::{json_typed, value::StaticNode};

    #[test]
    fn add_to_object() {
        let mut target_json = json_typed!(borrowed, {
            "name": "John",
            "age": 30
        });

        let patch = PatchJson {
            op: Op::Add,
            path: vec![Cow::Borrowed("address")],
            value: BorrowedValue::String(Cow::Borrowed("123 Main St")),
        };

        apply_patch(&mut target_json, patch)
            .unwrap_or_else(|err| panic!("Error applying patch: {err}"));

        // Check if the address field is added.
        assert_eq!(target_json["address"], "123 Main St");
    }

    #[test]
    fn add_to_nested_object() {
        let mut target_json = json_typed!(borrowed, {
            "user": {
                "name": "John",
                "age": 30
            }
        });

        let patch = PatchJson {
            op: Op::Add,
            path: vec![Cow::Borrowed("user"), Cow::Borrowed("address")],
            value: BorrowedValue::String(Cow::Borrowed("123 Main St")),
        };

        apply_patch(&mut target_json, patch)
            .unwrap_or_else(|err| panic!("Error applying patch: {err}"));
        // Check if the address field is added within the user object.
        assert_eq!(target_json["user"]["address"], "123 Main St");
    }

    #[test]
    fn add_to_array() {
        let mut target_json = json_typed!(borrowed, {
            "items": [1, 2, 3]
        });

        let patch = PatchJson {
            op: Op::Add,
            path: vec![Cow::Borrowed("items"), Cow::Borrowed("3")],
            value: BorrowedValue::Static(StaticNode::U64(4)),
        };

        apply_patch(&mut target_json, patch)
            .unwrap_or_else(|err| panic!("Error applying patch: {err}"));
        assert_eq!(target_json["items"][3], 4);
    }

    #[test]
    fn add_to_nested_array() {
        let mut target_json = json_typed!(borrowed, {
            "data": {
                "items": [1, 2, 3]
            }
        });

        let patch = PatchJson {
            op: Op::Add,
            path: vec![
                Cow::Borrowed("data"),
                Cow::Borrowed("items"),
                Cow::Borrowed("3"),
            ],
            value: BorrowedValue::Static(StaticNode::U64(4)),
        };

        apply_patch(&mut target_json, patch)
            .unwrap_or_else(|err| panic!("Error applying patch: {err}"));
        // Check if the value 4 is added to the array at index 3 within the nested object.
        assert_eq!(target_json["data"]["items"][3], 4);
    }

    #[test]
    fn replace_invalid_path() {
        let mut target_json = json_typed!(borrowed, {
            "name": "John",
            "age": 30
        });

        let patch = PatchJson {
            op: Op::Replace,
            path: vec![Cow::Borrowed("address"), Cow::Borrowed("zip")],
            value: BorrowedValue::String(Cow::Borrowed("12345")),
        };

        // The path "address.zip" doesn't exist yet, so this should result in an error.
        match apply_patch(&mut target_json, patch) {
            Ok(_) => panic!("Patch should not have succeeded"),
            Err(err) => {
                assert!(matches!(err, PatchError::PathNotFound { path } if path == "address.zip"));
            }
        }
    }
}
