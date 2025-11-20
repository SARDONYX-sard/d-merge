//! - Specifically for the 'config.json' namespace.
//! - Represents the configuration for each animation root specified in a `config.json` file.
use serde::{Deserialize, Serialize};
use simd_json::borrowed::Object;
use std::borrow::Cow;
use std::collections::HashMap;

/// Represents the configuration structure for the 'config.json' namespace.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct NamespaceConfig<'config> {
    /// The name associated with the configuration.
    #[serde(default)]
    pub name: &'config str,

    /// The description associated with the configuration.
    #[serde(default)]
    pub description: &'config str,

    /// The author associated with the configuration.
    #[serde(default)]
    pub author: &'config str,
}

pub fn prepare_namespace_json(namespace: &str, override_config: &FnisToOarConfig<'_>) -> String {
    let config = NamespaceConfig {
        name: override_config.name.as_deref().unwrap_or(namespace),
        description: override_config.description.as_deref().unwrap_or_default(),
        author: override_config.author.as_deref().unwrap_or_default(),
    };

    match simd_json::to_string_pretty(&config) {
        Ok(json) => json,
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::error!("(Originally unreachable)Failed to serialize namespace config JSON for namespace '{namespace}': {err}");
            String::new()
        }
    }
}

/// Represents the configuration for each animation root specified in a `config.json` file.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ConditionsConfig<'a> {
    /// An arbitrary name given by the user (value in the mapping table).
    ///
    /// # Note
    /// The name will probably exceed 24 bytes, so it should not be a [`CompactString`].
    #[serde(default)]
    pub name: Cow<'a, str>,
    /// The description associated with the animation root configuration.
    #[serde(default)]
    pub description: Cow<'a, str>,
    /// The priority of the animation root.
    #[serde(default)]
    pub priority: i32,
    /// An optional override for the animations folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "overrideAnimationsFolder")]
    pub override_animations_folder: Option<Cow<'a, str>>,
    /// A vector containing the conditions associated with the animation root.
    pub conditions: &'a [Object<'a>],
}

/// For the each `animations/OpenAnimationReplacer/<namespace>/<group name>_<slot>/config.json` for OAR, based on override config.
///
/// # Note
/// - `group_config_dir`: The caller side has already applied the override_config.
pub fn prepare_anim_config_json(
    group_config_dir: &str,
    group_name: &str,
    slot: u64,
    slot_config: Option<&SlotConfig<'_>>,
) -> String {
    let config = ConditionsConfig {
        name: Cow::Borrowed(group_config_dir), // NOTE: The caller side has already applied the override_config.
        description: Cow::Borrowed(
            slot_config
                .and_then(|slot| slot.description.as_deref())
                .unwrap_or_default(),
        ),
        priority: slot_config.and_then(|slot| slot.priority).unwrap_or(0) as i32,
        override_animations_folder: None,
        conditions: slot_config
            .map(|slot| slot.conditions.as_slice())
            .unwrap_or_default(),
    };

    match simd_json::to_string_pretty(&config) {
        Ok(json) => json,
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::error!(
                "(Originally unreachable)Failed to serialize animation config JSON for group '{}', slot {}: {}",
                group_name,
                slot,
                err
            );
            String::new()
        }
    }
}

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
    #[serde(default, borrow)]
    #[serde(bound(
        deserialize = "HashMap<Cow<'a, str>, GroupConfig<'a>>: serde::Deserialize<'de>"
    ))]
    pub groups: HashMap<Cow<'a, str>, GroupConfig<'a>>,
}

/// Configuration for a single FNIS group.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GroupConfig<'a> {
    /// Slot index -> Slot configuration
    #[serde(default, borrow, flatten)]
    #[serde(deserialize_with = "num_key_map::deserialize")]
    #[serde(bound(deserialize = "HashMap<u64, SlotConfig<'a>>: serde::Deserialize<'de>"))]
    pub slots: HashMap<u64, SlotConfig<'a>>,
}

/// JSON keys can only be strings; if we put a number in, it will fail 100% of the time, so change the processing.
mod num_key_map {
    use serde::de::{self, MapAccess, Visitor};
    use serde::Deserializer;
    use std::collections::HashMap;
    use std::fmt;

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<u64, super::SlotConfig<'de>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(SlotMapVisitor)
    }

    struct SlotMapVisitor;

    impl<'de> Visitor<'de> for SlotMapVisitor {
        type Value = HashMap<u64, super::SlotConfig<'de>>;

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
                out.insert(parsed, value);
            }

            Ok(out)
        }
    }
}

/// Configuration for a single slot in a group.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SlotConfig<'a> {
    /// Optional rename of the animation
    #[serde(borrow)]
    pub rename_to: Option<Cow<'a, str>>,

    /// The description associated with the each config.json.
    #[serde(borrow)]
    pub description: Option<Cow<'a, str>>,

    /// Optional OAR priority for this slot
    pub priority: Option<u32>,

    /// Arbitrary JSON object representing OAR conditions.
    /// Unsafe: ensure valid JSON for OAR `config.json`.
    #[serde(default, borrow)]
    #[serde(bound(deserialize = "Vec<Object<'a>>: serde::Deserialize<'de>"))]
    pub conditions: Vec<Object<'a>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_de_config_json() {
    //     let s = include_str!(r"");
    //     let config: FnisToOarConfig = sonic_rs::from_str(s).unwrap();
    //     dbg!(&config);
    // }

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

        let expected = {
            let mut expected_groups: HashMap<Cow<'static, str>, GroupConfig<'static>> =
                HashMap::new();
            let mut slots_map: HashMap<u64, SlotConfig<'static>> = HashMap::new();
            slots_map.insert(
                0,
                SlotConfig {
                    rename_to: Some(Cow::Borrowed("IA Sword at Hip (Dual Sheath) _1hmeqp")),
                    description: None,
                    priority: Some(5000),
                    conditions: Vec::new(),
                },
            );
            slots_map.insert(
                1,
                SlotConfig {
                    rename_to: Some(Cow::Borrowed("IA Sword at Back (Dual Sheath) _1hmeqp")),
                    description: None,
                    priority: Some(5000),
                    conditions: Vec::new(),
                },
            );
            slots_map.insert(
                3,
                SlotConfig {
                    rename_to: Some(Cow::Borrowed("IA Sword n Board _1hmeqp")),
                    description: None,
                    priority: Some(5000),
                    conditions: Vec::new(),
                },
            );
            slots_map.insert(
                4,
                SlotConfig {
                    rename_to: Some(Cow::Borrowed(
                        "IA Sword at Back + Dagger on Back Combo _1hmeqp",
                    )),
                    description: None,
                    priority: Some(5000),
                    conditions: Vec::new(),
                },
            );
            expected_groups.insert(Cow::Borrowed("_1hmeqp"), GroupConfig { slots: slots_map });

            FnisToOarConfig {
                schema: None,
                name: Some(Cow::Borrowed("XPMSE")),
                description: None,
                author: Some(Cow::Borrowed("Generated")),
                groups: expected_groups,
            }
        };

        assert_eq!(actual, expected);
    }
}
