use super::error::Result;
use super::PatchJson;
use crate::searcher::PointerMut as _;
use simd_json::BorrowedValue;

pub(crate) fn apply_add<'v>(json: &mut BorrowedValue<'v>, patch: PatchJson<'v>) -> Result<()> {
    json.push_by(patch.path, patch.value)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merger::Op;
    use simd_json::{json_typed, value::StaticNode};
    use std::borrow::Cow;

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

        apply_add(&mut target_json, patch)
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

        apply_add(&mut target_json, patch)
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

        apply_add(&mut target_json, patch)
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

        apply_add(&mut target_json, patch)
            .unwrap_or_else(|err| panic!("Error applying patch: {err}"));
        // Check if the value 4 is added to the array at index 3 within the nested object.
        assert_eq!(target_json["data"]["items"][3], 4);
    }
}
