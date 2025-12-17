use crate::asdsf::patch::de::diff::patch_map::{OnePatchMap, SeqPatchMap};

/// Attacks diff patch
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AttacksDiff<'a> {
    /// modifying fields of a single attack.
    ///
    /// - key(json path): e.g.,
    ///   - ["[0]", "attack_trigger"]
    ///   - ["[1]", "is_contextual"]
    ///   - ["[2]", "clip_names", "[0]"]
    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "OnePatchMap<'a>: serde::Deserialize<'de>")
        )
    )]
    pub one: OnePatchMap<'a>,

    /// add/replace/remove entire attacks in the range.
    ///
    /// - key(json path): e.g.,
    ///   - []: attacks array itself
    ///   - ["[1]", "clip_names"]
    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "SeqPatchMap<'a>: serde::Deserialize<'de>")
        )
    )]
    pub seq: SeqPatchMap<'a>,
}

pub(crate) fn attacks_to_borrowed_value(
    attacks: Vec<crate::asdsf::normal::Attack<'_>>,
) -> simd_json::borrowed::Value<'_> {
    use simd_json::borrowed::Value;

    let mut vec = Vec::with_capacity(attacks.len());
    for attack in attacks {
        let mut map = simd_json::borrowed::Object::new();
        map.insert(
            "attack_trigger".into(),
            Value::String(attack.attack_trigger),
        );
        map.insert("is_contextual".into(), attack.is_contextual.into());
        map.insert("clip_names_len".into(), attack.clip_names_len.into());
        let clip_names_array = attack
            .clip_names
            .into_iter()
            .map(Value::String)
            .collect::<Vec<_>>();
        map.insert(
            "clip_names".into(),
            Value::Array(Box::new(clip_names_array)),
        );

        vec.push(Value::Object(Box::new(map)));
    }

    Value::Array(Box::new(vec))
}
