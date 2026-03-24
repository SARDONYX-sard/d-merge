//! - Specifically for the 'config.json' namespace.
//! - Represents the configuration for each animation root specified in a `config.json` file.
use std::borrow::Cow;

use serde::Serialize;
use simd_json::owned::Object;

use crate::behaviors::tasks::fnis::patch_gen::alternate::group_names::AAGroupName;

use super::override_config::{FnisToOarConfig, SlotConfig};

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

pub fn prepare_namespace_json(
    namespace: &str,
    override_config: &Option<FnisToOarConfig<'_>>,
) -> String {
    let (name, description, author) = if let Some(c) = override_config {
        (
            c.name.as_deref().unwrap_or(namespace),
            c.description.as_deref().unwrap_or_default(),
            c.author.as_deref().unwrap_or_default(),
        )
    } else {
        (namespace, "", "")
    };
    let config = NamespaceConfig {
        name,
        description,
        author,
    };

    match sonic_rs::to_string_pretty(&config) {
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
    pub conditions: Vec<Object>,
}

/// new graph variable value condition
///
/// - group_name: e.g., `_1hm_eqp`
fn fnis_aa_condition(group_name: AAGroupName, value: u64) -> Object {
    simd_json::derived::ValueTryIntoObject::try_into_object(simd_json::json_typed!(owned, {
        "condition": "CompareValues",
        "requiredVersion": "1.0.0.0",
        "Value A": {
            "graphVariable": format!("FNISaa{}", group_name.as_fnis_str()), // e.g., "FNISaa_1hm_eqp"
            "graphVariableType": "Int"
        },
        "Comparison": "==",
        "Value B": {
            "value": value,
        }
    }))
    .unwrap_or_default()
}

/// Builds the per-slot OAR `config.json`.
///
/// `animations/OpenAnimationReplacer/<namespace>/<group name>_<slot>/config.json` for OAR, based on override config.
///
/// # Note
/// - `group_config_dir`: The caller side has already applied the override_config.
///
/// `Value B` is `base + slot` (1-based graph variable value):
/// - slot 0 of a mod whose base=6  → variable value 6
/// - slot 1                        → variable value 7
pub fn new_fnis_aa_slot_config_json(
    group_config_dir: &str,
    group_name: AAGroupName,
    slot: u64,
    base: u64,
    slot_config: Option<&SlotConfig>,
) -> String {
    let user_conditions = slot_config
        .map(|s| s.conditions.as_slice())
        .unwrap_or_default();

    // Always prepend the FNISaa condition, then append user-defined conditions.
    let conditions: Vec<_> = core::iter::once(fnis_aa_condition(group_name, base + slot))
        .chain(user_conditions.iter().cloned())
        .collect();

    // description
    let description = match slot_config.and_then(|s| s.description.as_deref()) {
        Some(desc) => Cow::Borrowed(desc),
        None => Cow::Owned(format!("base({base}) + value({slot})")),
    };

    // priority
    let priority = slot_config
        .and_then(|s| s.priority)
        .map_or(i32::MAX, |p| p as i32);

    let config = ConditionsConfig {
        name: Cow::Borrowed(group_config_dir),
        description,
        priority,
        override_animations_folder: None,
        conditions,
    };

    match sonic_rs::to_string_pretty(&config) {
        Ok(json) => json,
        Err(e) => {
            #[cfg(feature = "tracing")]
            tracing::error!(
                "Failed to serialize animation config JSON for group '{group_name}' slot {slot}: {e}"
            );
            String::new()
        }
    }
}
