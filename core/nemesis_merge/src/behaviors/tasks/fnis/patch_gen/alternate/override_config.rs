use std::{borrow::Cow, collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use simd_json::owned::Object;

/// Override configuration stored in:
/// - humanoid: `FNIS_<namespace>_toOAR.json`
/// - creature: `FNIS_<namespace>_<creature>_toOAR.json`
///
/// Allows overriding default FNIS -> OAR conversion per namespace.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FnisToOarConfig<'a> {
    #[serde(borrow)]
    #[serde(rename = "$schema")]
    pub schema: Option<Cow<'a, str>>,

    /// Name for mod-specific OAR settings in `animations/OpenAnimationReplacer/<name>/config.json`.
    /// If unspecified, FNIS's namespace is used.
    ///
    /// e.g., "XPMSE".
    #[serde(borrow)]
    pub name: Option<Cow<'a, str>>,

    /// The description associated with the configuration.
    #[serde(borrow)]
    pub description: Option<Cow<'a, str>>,

    /// The author associated with the configuration.
    #[serde(borrow)]
    pub author: Option<Cow<'a, str>>,

    /// Mapping of FNIS group names to their override settings.
    ///
    /// Key: group name such as `"_1hm_eqp"`.
    #[serde(default)]
    pub groups: HashMap<Cow<'a, str>, GroupConfig>,
}

/// Configuration for a single FNIS group.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GroupConfig {
    /// 1based Slot index -> Slot configuration
    #[serde(default, flatten)]
    #[serde(deserialize_with = "num_key_map::deserialize")]
    pub slots: HashMap<u64, Arc<SlotConfig>>,
}

/// JSON keys can only be strings; if we put a number in, it will fail 100% of the time, so change the processing.
mod num_key_map {
    use std::{collections::HashMap, fmt, sync::Arc};

    use serde::{
        de::{self, MapAccess, Visitor},
        Deserializer,
    };

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<u64, Arc<super::SlotConfig>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(SlotMapVisitor)
    }

    struct SlotMapVisitor;

    impl<'de> Visitor<'de> for SlotMapVisitor {
        type Value = HashMap<u64, Arc<super::SlotConfig>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("map with string keys representing u64")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut out = HashMap::new();

            while let Some((key, value)) = access.next_entry::<&str, super::SlotConfig>()? {
                let parsed = key.parse::<u64>().map_err(de::Error::custom)?;
                out.insert(parsed, Arc::new(value));
            }

            Ok(out)
        }
    }
}

/// Configuration for a single slot in a group.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlotConfig {
    /// Optional rename of the animation
    pub rename_to: Option<String>,

    /// The description associated with the each config.json.
    pub description: Option<String>,

    /// Optional OAR priority for this slot
    pub priority: Option<u32>,

    /// Arbitrary JSON object representing OAR conditions.
    /// Unsafe: ensure valid JSON for OAR `config.json`.
    #[serde(default)]
    pub conditions: Vec<Object>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_compare_with_expected_struct() {
        let input_json = r#"
        {
            "name": "XPMSE",
            "author": "Generated",
            "groups": {
                "_1hmeqp": {
                    "0": {
                        "rename_to": "IA Sword at Hip (Dual Sheath) _1hmeqp",
                        "priority": 5000
                    },
                    "1": {
                        "rename_to": "IA Sword at Back (Dual Sheath) _1hmeqp",
                        "priority": 5000
                    },
                    "3": {
                        "rename_to": "IA Sword n Board _1hmeqp",
                        "priority": 5000
                    },
                    "4": {
                        "rename_to": "IA Sword at Back + Dagger on Back Combo _1hmeqp",
                        "priority": 5000
                    }
                }
            }
        }
        "#;

        let actual: FnisToOarConfig = sonic_rs::from_str(input_json).expect("deserialize actual");

        let mut expected_groups = HashMap::new();
        let mut slots_map = HashMap::new();

        slots_map.insert(
            0,
            Arc::new(SlotConfig {
                rename_to: Some("IA Sword at Hip (Dual Sheath) _1hmeqp".to_string()),
                description: None,
                priority: Some(5000),
                conditions: Vec::new(),
            }),
        );
        slots_map.insert(
            1,
            Arc::new(SlotConfig {
                rename_to: Some("IA Sword at Back (Dual Sheath) _1hmeqp".to_string()),
                description: None,
                priority: Some(5000),
                conditions: Vec::new(),
            }),
        );

        slots_map.insert(
            3,
            Arc::new(SlotConfig {
                rename_to: Some("IA Sword n Board _1hmeqp".to_string()),
                description: None,
                priority: Some(5000),
                conditions: Vec::new(),
            }),
        );

        slots_map.insert(
            4,
            Arc::new(SlotConfig {
                rename_to: Some("IA Sword at Back + Dagger on Back Combo _1hmeqp".to_string()),
                description: None,
                priority: Some(5000),
                conditions: Vec::new(),
            }),
        );

        expected_groups.insert(Cow::Borrowed("_1hmeqp"), GroupConfig { slots: slots_map });

        let expected = FnisToOarConfig {
            schema: None,
            name: Some(Cow::Borrowed("XPMSE")),
            description: None,
            author: Some(Cow::Borrowed("Generated")),
            groups: expected_groups,
        };

        assert_eq!(actual, expected);
    }
}
