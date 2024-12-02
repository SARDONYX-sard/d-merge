use super::error::{PathNotFoundSnafu, Result};
use super::range::{parse_index_or_range, RangeOrIndex};
use super::PatchJson;
use crate::searcher::PointerMut as _;
use simd_json::BorrowedValue;

pub(crate) fn apply_replace<'a>(json: &mut BorrowedValue<'a>, patch: PatchJson<'a>) -> Result<()> {
    if let Some(target) = json.ptr_mut(&patch.path) {
        if let BorrowedValue::Array(list) = target {
            let segment = patch.path.last().ok_or_else(|| {
                PathNotFoundSnafu {
                    path: patch.path.join("."),
                }
                .build()
            })?;
            match parse_index_or_range(segment)? {
                RangeOrIndex::Index(index) => {
                    if index < list.len() {
                        list[index] = patch.value;
                        Ok(())
                    } else {
                        PathNotFoundSnafu {
                            path: patch.path.join("."),
                        }
                        .fail()
                    }
                }
                RangeOrIndex::Range(range) => {
                    if range.end <= list.len() {
                        for i in range {
                            list[i] = patch.value.clone();
                        }
                        Ok(())
                    } else {
                        PathNotFoundSnafu {
                            path: patch.path.join("."),
                        }
                        .fail()
                    }
                }
            }
        } else {
            *target = patch.value;
            Ok(())
        }
    } else {
        PathNotFoundSnafu {
            path: patch.path.join("."),
        }
        .fail()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merger::{error::PatchError, Op};
    use simd_json::json_typed;
    use std::borrow::Cow;

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
        match apply_replace(&mut target_json, patch) {
            Ok(_) => panic!("Patch should not have succeeded"),
            Err(err) => {
                assert!(
                    matches!(err, PatchError::PathNotFound { path, .. } if path == "address.zip")
                );
            }
        }
    }
}
