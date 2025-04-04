mod add;
mod remove;
mod replace;

use self::add::handle_add;
use self::remove::handle_remove;
use self::replace::handle_replace;
use super::error::{JsonPatchError, Result};
use crate::operation::Op;
use crate::ptr_mut::PointerMut as _;
use crate::JsonPath;
use simd_json::borrowed::Value;

/// Apply json patch for range statements(`[index]`,`[start..end]`, `[start..]`, `[end..]`, `[..]`)
///
/// # Errors
/// - If `range` is out of bounds.
/// - If `target` is not [`Value::Array`]
pub fn apply_range<'a>(
    json: &mut Value<'a>,
    path: JsonPath<'a>,
    op: Op,
    range: crate::range::Range,
    value: Value<'a>,
) -> Result<()> {
    let target = json
        .ptr_mut(&path)
        .ok_or_else(|| JsonPatchError::NotFoundTarget {
            path: path.join("."),
        })?;

    match target {
        Value::Array(target) => {
            match op {
                Op::Add => handle_add(target, range, value),
                Op::Remove => {
                    range.check_valid_range(target.len()).map_err(|err| {
                        JsonPatchError::OutOfRange {
                            path: path.join("."),
                            source: err,
                        }
                    })?;
                    handle_remove(target, range);
                }
                Op::Replace => {
                    range.check_valid_range(target.len()).map_err(|err| {
                        JsonPatchError::OutOfRange {
                            path: path.join("."),
                            source: err,
                        }
                    })?;
                    handle_replace(target, range, value);
                }
            };
            Ok(())
        }
        _ => Err(JsonPatchError::UnsupportedRangeKind),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::range::Range;
    use crate::{apply::Op, json_path};
    use simd_json::{json_typed, ValueBuilder as _};
    use std::borrow::Cow;

    #[test]
    fn test_add_to_full_range() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });

        let path = json_path!["array_key"];
        let range = Range::Full;
        let value = json_typed!(borrowed, [4, 5]);
        apply_range(&mut target, path, Op::Add, range, value).unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "array_key": [1, 2, 3, 4, 5]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_add_to_specific_index() {
        let mut target = json_typed!(borrowed, {
            "array_key": ["a", "b", "c"]
        });
        // let patch = JsonPatch {
        //     op: Op::Add,
        //     path: vec![Cow::Borrowed("array_key"), Cow::Borrowed("[1]")],
        //     value: json_typed!(borrowed, "x"),
        //     ..Default::default()
        // };

        let path = json_path!["array_key"];
        let range = Range::Index(1);
        let value = json_typed!(borrowed, "x");
        apply_range(&mut target, path, Op::Add, range, value).unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "array_key": ["a", "x", "b", "c"]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_add_to_0start_of_array() {
        let mut target = json_typed!(borrowed, {
            "array_key": [2, 3, 4]
        });
        // let patch = JsonPatch {
        //     op: Op::Add,
        //     path: vec![Cow::Borrowed("array_key"), Cow::Borrowed("[0:]")],
        //     value: json_typed!(borrowed, 1),
        //     ..Default::default()
        // };

        let path = json_path!["array_key"];
        let range = Range::FromTo(0..3);
        let value = json_typed!(borrowed, 1);
        apply_range(&mut target, path, Op::Add, range, value).unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "array_key": [1, 1, 1, 2, 3, 4]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_remove_specific_index() {
        let mut target = json_typed!(borrowed, {
            "array_key": ["x", "y", "z"]
        });
        // let patch = JsonPatch {
        //     op: Op::Remove,
        //     path: vec![Cow::Borrowed("array_key"), Cow::Borrowed("[1]")],
        //     value: Value::null(),
        //     ..Default::default()
        // };

        let path = json_path!["array_key"];
        let range = Range::Index(1);
        let value = Value::null();
        apply_range(&mut target, path, Op::Remove, range, value)
            .unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "array_key": ["x", "z"]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_remove_slice() {
        let mut target = json_typed!(borrowed, {
            "array_key": [10, 20, 30, 40, 50]
        });
        // let patch = JsonPatch {
        //     op: Op::Remove,
        //     path: vec![Cow::Borrowed("array_key"), Cow::Borrowed("[1:4]")],
        //     value: Value::null(),
        //     ..Default::default()
        // };

        let path: Vec<Cow<'_, str>> = json_path!["array_key"];
        let range = Range::FromTo(1..4);
        let value = Value::null();
        apply_range(&mut target, path, Op::Remove, range, value)
            .unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "array_key": [10, 50]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_remove_up_to_index() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3, 4, 5]
        });
        // let patch = JsonPatch {
        //     op: Op::Remove,
        //     path: vec![Cow::Borrowed("array_key"), Cow::Borrowed("[:3]")],
        //     value: Value::null(),
        //     ..Default::default()
        // };

        let path: Vec<Cow<'_, str>> = json_path!["array_key"];
        let range = Range::FromTo(0..3);
        let value = Value::null();
        apply_range(&mut target, path, Op::Remove, range, value)
            .unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "array_key": [4, 5]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_remove_from_index() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3, 4, 5]
        });
        // let patch = JsonPatch {
        //     op: Op::Remove,
        //     path: vec![Cow::Borrowed("array_key"), Cow::Borrowed("[3:]")],
        //     value: Value::null(),
        //     ..Default::default()
        // };

        let path: Vec<Cow<'_, str>> = json_path!["array_key"];
        let range = Range::FromTo(3..5);
        let value = Value::null();
        apply_range(&mut target, path, Op::Remove, range, value)
            .unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_replace_slice() {
        let mut target = json_typed!(borrowed, {
            "array_key": ["a", "b", "c", "d"]
        });
        // let patch = JsonPatch {
        //     op: Op::Replace,
        //     path: vec![Cow::Borrowed("array_key"), Cow::Borrowed("[1:3]")],
        //     value: json_typed!(borrowed, ["x", "y"]),
        //     ..Default::default()
        // };

        let path: Vec<Cow<'_, str>> = json_path!["array_key"];
        let range = Range::FromTo(1..3);
        let value = json_typed!(borrowed, ["x", "y"]);
        apply_range(&mut target, path, Op::Replace, range, value)
            .unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "array_key": ["a", "x", "y", "d"]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_replace_entire_array() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3]
        });
        // let patch = JsonPatch {
        //     op: Op::Replace,
        //     path: vec![Cow::Borrowed("array_key"), Cow::Borrowed("[:]")],
        //     value: json_typed!(borrowed, [4, 5, 6]),
        //     ..Default::default()
        // };

        let path: Vec<Cow<'_, str>> = json_path!["array_key"];
        let range = Range::Full;
        let value = json_typed!(borrowed, [4, 5, 6]);
        apply_range(&mut target, path, Op::Replace, range, value)
            .unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "array_key": [4, 5, 6]
        });
        assert_eq!(target, expected);
    }

    #[test]
    fn test_clear_array() {
        let mut target = json_typed!(borrowed, {
            "array_key": [1, 2, 3, 4]
        });
        // let patch = JsonPatch {
        //     op: Op::Remove,
        //     path: vec![Cow::Borrowed("array_key"), Cow::Borrowed("[:]")],
        //     value: Value::null(),
        //     ..Default::default()
        // };

        let path: Vec<Cow<'_, str>> = json_path!["array_key"];
        let range = Range::Full;
        let value = Value::null();
        apply_range(&mut target, path, Op::Remove, range, value)
            .unwrap_or_else(|err| panic!("{err}"));

        let expected = json_typed!(borrowed, {
            "array_key": []
        });
        assert_eq!(target, expected);
    }
}
