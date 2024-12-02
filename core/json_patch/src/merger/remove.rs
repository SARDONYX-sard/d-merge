use super::error::{PatchError, PathNotFoundSnafu, Result};
use super::range::{parse_index_or_range, RangeOrIndex};
use super::PatchJson;
use crate::searcher::PointerMut as _;
use simd_json::borrowed::{Array, Object};
use simd_json::BorrowedValue;

pub(crate) fn apply_remove<'a>(json: &mut BorrowedValue<'a>, patch: PatchJson<'a>) -> Result<()> {
    if let Some(target) = json.ptr_mut(&patch.path) {
        match target {
            BorrowedValue::Object(map) => remove_from_object(map, &patch),
            BorrowedValue::Array(list) => remove_from_array(list, patch),
            _ => Err(PatchError::InvalidOperation {
                path: patch.path.join("."),
            }),
        }
    } else {
        PathNotFoundSnafu {
            path: patch.path.join("."),
        }
        .fail()
    }
}

fn remove_from_object<'a>(map: &mut Object<'a>, patch: &PatchJson<'a>) -> Result<()> {
    let key = patch.path.last().unwrap();
    if map.remove(key).is_none() {
        return PathNotFoundSnafu {
            path: patch.path.join("."),
        }
        .fail();
    }
    Ok(())
}

fn remove_from_array<'a>(list: &mut Array<'a>, patch: PatchJson<'a>) -> Result<()> {
    let segment = patch.path.last().unwrap();
    match parse_index_or_range(segment)? {
        RangeOrIndex::Index(index) => {
            if index < list.len() {
                list.remove(index);
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
                list.drain(range);
                Ok(())
            } else {
                PathNotFoundSnafu {
                    path: patch.path.join("."),
                }
                .fail()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merger::Op;
    use simd_json::{json_typed, StaticNode};
    use std::borrow::Cow;

    #[test]
    fn remove_range_from_array() {
        let mut target_json = json_typed!(borrowed, {
            "items": [1, 2, 3, 4, 5]
        });

        let patch = PatchJson {
            op: Op::Remove,
            path: vec![Cow::Borrowed("items"), Cow::Borrowed("[1:3]")],
            value: BorrowedValue::Static(StaticNode::Null),
        };

        apply_remove(&mut target_json, patch)
            .unwrap_or_else(|err| panic!("Error applying patch: {err}"));
        assert_eq!(target_json["items"], json_typed!(borrowed, [1, 4, 5]));
    }
}
