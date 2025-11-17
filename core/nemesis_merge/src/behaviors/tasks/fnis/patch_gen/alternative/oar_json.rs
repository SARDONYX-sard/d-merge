//! - Specifically for the 'config.json' namespace.
//! - Represents the configuration for each animation root specified in a `config.json` file.
use serde::{Deserialize, Serialize};
use simd_json::borrowed::Object;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;

use crate::behaviors::tasks::hkx::generate::write_patched_json;
use crate::errors::Error;

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

pub fn write_namespace_json(
    namespace: &str,
    output_dir: &Path,
    override_config: &FnisToOarConfig<'_>,
) -> Result<(), Error> {
    let namespace_config = NamespaceConfig {
        name: override_config.name.as_deref().unwrap_or(namespace),
        description: override_config.description.as_deref().unwrap_or_default(),
        author: override_config.author.as_deref().unwrap_or_default(),
    };
    let namespace_config_path = output_dir.join("config.json");
    write_patched_json(&namespace_config_path, &namespace_config)
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

/// Write the each `animations/OpenAnimationReplacer/<namespace>/<group name>_<slot>/config.json` for OAR, based on override config.
///
/// # Note
/// - `group_config_dir`: The caller side has already applied the override_config.
pub fn write_anim_config_json(
    output_dir: &Path,
    group_config_dir: &str,
    group_name: &str,
    slot: u64,
    override_config: &FnisToOarConfig<'_>,
) -> Result<(), Error> {
    let slot_cfg = override_config
        .groups
        .get(group_name)
        .and_then(|group_cfg| group_cfg.slots.get(&slot));

    let config = ConditionsConfig {
        name: Cow::Borrowed(group_config_dir), // NOTE: The caller side has already applied the override_config.
        description: Cow::Borrowed(
            slot_cfg
                .and_then(|slot| slot.description.as_deref())
                .unwrap_or_default(),
        ),
        priority: slot_cfg.and_then(|slot| slot.priority).unwrap_or(0) as i32,
        override_animations_folder: None,
        conditions: slot_cfg
            .map(|slot| slot.conditions.as_slice())
            .unwrap_or_default(),
    };

    let conditions_path = output_dir.join("config.json");
    write_patched_json(&conditions_path, &config)
}

/// Override configuration stored in:
/// - humanoid: `FNIS_<namespace>_toOAR.json`
/// - creature: `FNIS_<namespace>_<creature>_toOAR.json`
///
/// Allows overriding default FNIS -> OAR conversion per namespace.
#[derive(Debug, Default, Serialize, Deserialize)]
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
    #[serde(default, borrow, flatten)]
    #[serde(bound(
        deserialize = "HashMap<Cow<'a, str>, GroupConfig<'a>>: serde::Deserialize<'de>"
    ))]
    pub groups: HashMap<Cow<'a, str>, GroupConfig<'a>>,
}

/// Configuration for a single FNIS group.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GroupConfig<'a> {
    /// Slot index -> Slot configuration
    #[serde(default, borrow)]
    #[serde(bound(deserialize = "HashMap<u64, SlotConfig<'a>>: serde::Deserialize<'de>"))]
    pub slots: HashMap<u64, SlotConfig<'a>>,
}

/// Configuration for a single slot in a group.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SlotConfig<'a> {
    /// Optional rename of the animation
    #[serde(borrow)]
    pub rename_to: Option<Cow<'a, str>>,

    /// The description associated with the each config.json.
    pub description: Option<Cow<'a, str>>,

    /// Optional OAR priority for this slot
    pub priority: Option<u32>,

    /// Arbitrary JSON object representing OAR conditions.
    /// Unsafe: ensure valid JSON for OAR `config.json`.
    #[serde(borrow)]
    #[serde(bound(deserialize = "Vec<Object<'a>>: serde::Deserialize<'de>"))]
    pub conditions: Vec<Object<'a>>,
}
