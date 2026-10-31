// SPDX-FileCopyrightText: Copyright (c) 2023 max-su-2019
// SPDX-License-Identifier: MIT
//
// The annotation formats, key names and their descriptions in this file are taken from the
// SCAR developer manual (https://github.com/max-su-2019/SCAR, MIT License; the license text is in
// `LICENSES/LICENSE-MIT` at the repository root).

//! SCAR (Skyrim Combat AI Revamp) 2.0+ annotations: definitions shared by completion, hover and
//! diagnostics.
//!
//! ```hkanno
//! 0.833333 SCAR_ComboStart
//! 0.833333 SCAR_ComboStart.60
//! 0.000000 SCAR_UpdateDummy
//! 1.000000 SCAR_ActionData{"AttackData":"attackStart", "MinDistance":0, "MaxDistance":139, "StartAngle":-60, "EndAngle":60, "Chance":100, "Type":"RA"}
//! ```
//! Source: [Developers Manual For SCAR 2.0+](https://github.com/max-su-2019/SCAR/blob/main/docs/EN/Developers%20Manual%20For%20SCAR%202.0%2B.md)

/// Link to the SCAR developer manual (shown in hovers).
pub(crate) const MANUAL_URL: &str = "https://github.com/max-su-2019/SCAR/blob/main/docs/EN/Developers%20Manual%20For%20SCAR%202.0%2B.md";

/// `SCAR_ActionData` (the text before the JSON body).
pub(crate) const ACTION_DATA: &str = "SCAR_ActionData";
/// `SCAR_ComboStart` (optionally `.<chance>`).
pub(crate) const COMBO_START: &str = "SCAR_ComboStart";
/// `SCAR_UpdateDummy`
pub(crate) const UPDATE_DUMMY: &str = "SCAR_UpdateDummy";

/// The `Type` values of an action.
pub(crate) const ACTION_TYPES: [(&str, &str); 9] = [
    ("RA", "Right-hand attack"),
    ("RPA", "Right-hand power attack"),
    ("LA", "Left-hand attack"),
    ("LPA", "Left-hand power attack"),
    ("DA", "Dual attack"),
    ("DPA", "Dual power attack"),
    ("BA", "Bash attack"),
    ("BPA", "Power bash attack"),
    ("IDLE", "Idle (no attack)"),
];

/// Built-in idle animations for `IdleAnimation`.
pub(crate) const BUILTIN_IDLES: [&str; 3] =
    ["ADXP_NPCPowerAttack", "ADXP_NPCNormalAttack", "ADXP_NPCPowerAttack_H2H"];

/// What a JSON key holds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ValueKind {
    /// A JSON string (e.g., an EditorID).
    String,
    /// A JSON number, with an optional inclusive range.
    Number { min: Option<f64>, max: Option<f64> },
    /// A positive integer.
    PositiveInt,
    /// One of [`ACTION_TYPES`].
    ActionType,
}

/// One key of the `SCAR_ActionData` JSON.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ActionKey {
    pub(crate) name: &'static str,
    pub(crate) kind: ValueKind,
    /// The value inserted by completion, in snippet syntax (e.g., `${1:0}`).
    pub(crate) placeholder: &'static str,
    pub(crate) doc: &'static str,
}

const fn number(min: Option<f64>, max: Option<f64>) -> ValueKind {
    ValueKind::Number { min, max }
}

/// Every key of `SCAR_ActionData{...}`, in the manual's order.
pub(crate) const ACTION_KEYS: [ActionKey; 15] = [
    ActionKey {
        name: "IdleAnimation",
        kind: ValueKind::String,
        placeholder: "\"${1:ADXP_NPCNormalAttack}\"",
        doc: "EditorID of the Idle Animation performed when the conditions pass. Cannot be combined with `AttackData`.\nBuilt-in: `ADXP_NPCPowerAttack`, `ADXP_NPCNormalAttack`.",
    },
    ActionKey {
        name: "AttackData",
        kind: ValueKind::String,
        placeholder: "\"${1:attackStart}\"",
        doc: "Attack data name of the actor's race record; performs the attack directly, without an Idle record. Cannot be combined with `IdleAnimation`.",
    },
    ActionKey {
        name: "MinDistance",
        kind: number(Some(0.0), None),
        placeholder: "${1:0}",
        doc: "Minimum distance to the target. The action runs only if the distance is greater.",
    },
    ActionKey {
        name: "MaxDistance",
        kind: number(Some(0.0), None),
        placeholder: "${1:139}",
        doc: "Maximum distance to the target. The action runs only if the distance is less. The weapon reach is added automatically.",
    },
    ActionKey {
        name: "StartAngle",
        kind: number(Some(-180.0), Some(180.0)),
        placeholder: "${1:-60}",
        doc: "Start of the clockwise yaw arc to the target (-180 to 180). Used with `EndAngle`.",
    },
    ActionKey {
        name: "EndAngle",
        kind: number(Some(-180.0), Some(180.0)),
        placeholder: "${1:60}",
        doc: "End of the clockwise yaw arc to the target (-180 to 180). Used with `StartAngle`.",
    },
    ActionKey {
        name: "Chance",
        kind: number(Some(0.0), Some(100.0)),
        placeholder: "${1:100}",
        doc: "Random chance (0 to 100) that the action executes.",
    },
    ActionKey {
        name: "Type",
        kind: ValueKind::ActionType,
        placeholder: "\"${1:RA}\"",
        doc: "Action type: `RA` `RPA` `LA` `LPA` `DA` `DPA` `BA` `BPA` `IDLE`. Affects the attack data and the weapon reach.",
    },
    ActionKey {
        name: "CoolDownTime",
        kind: number(Some(0.0), None),
        placeholder: "${1:2.25}",
        doc: "(v2.0+) Cooldown of the attack data in seconds (> 0), so the same attack is not repeated within the interval.",
    },
    ActionKey {
        name: "CoolDownAlias",
        kind: ValueKind::String,
        placeholder: "\"${1:attackStart}\"",
        doc: "(v2.0+) Used with `IdleAnimation`: borrows the cooldown of this attack data name.",
    },
    ActionKey {
        name: "ConditionsAlias",
        kind: ValueKind::String,
        placeholder: "\"${1:ADXP_NPCNormalAttack}\"",
        doc: "(v2.0+) Used with `AttackData`: borrows the conditions of this Idle Animation.",
    },
    ActionKey {
        name: "VariantID",
        kind: ValueKind::PositiveInt,
        placeholder: "${1:1}",
        doc: "(v2.0+) Sets the graph variable Int `SCAR_AttackVariants` (> 0), e.g. for Open Animation Replacer conditions.",
    },
    ActionKey {
        name: "WeaponLength",
        kind: number(Some(0.0), None),
        placeholder: "${1:90}",
        doc: "Overrides the weapon reach with a fixed value. Recommended for unarmed movesets.",
    },
    ActionKey {
        name: "TriggerStartTime",
        kind: number(Some(0.0), None),
        placeholder: "${1:0.8}",
        doc: "Animation time at which the window in which the action is valid starts.",
    },
    ActionKey {
        name: "TriggerEndTime",
        kind: number(Some(0.0), None),
        placeholder: "${1:1.0}",
        doc: "Animation time at which the window in which the action is valid ends.",
    },
];

pub(crate) fn action_key(name: &str) -> Option<&'static ActionKey> {
    ACTION_KEYS.iter().find(|key| key.name == name)
}

/// The known key `name` is a typo of: same ignoring case, or at most 2 edits away
/// (e.g., `chance` / `Chanse` -> `Chance`).
pub(crate) fn closest_key(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    ACTION_KEYS
        .iter()
        .map(|key| (edit_distance(&lower, &key.name.to_ascii_lowercase()), key.name))
        .filter(|&(distance, _)| distance <= 2)
        .min_by_key(|&(distance, _)| distance)
        .map(|(_, name)| name)
}

/// Levenshtein distance (chars).
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (above + 1).min(row[j] + 1).min(diagonal + usize::from(ca != cb));
            diagonal = above;
        }
    }
    row[b.len()]
}

/// Markdown for `SCAR_ComboStart`.
pub(crate) const COMBO_START_DOC: &str = "# SCAR_ComboStart\nAnnotated at the local time when the next attack transition may begin.\nFor ADXP/AMCO, place it between `MCO_WinOpen` and `MCO_WinClose` (or `MCO_PowerWinOpen` and `MCO_PowerWinClose`).\n\n# Format\n```hkanno\nSCAR_ComboStart\nSCAR_ComboStart.<chance: 0-100>\n```\nThe optional `.<chance>` suffix is the percentage chance that the next attack starts (default: 100).\n- required: SCAR (Skyrim Combat AI Revamp)";

/// Markdown for `SCAR_UpdateDummy`.
pub(crate) const UPDATE_DUMMY_DOC: &str = "# SCAR_UpdateDummy\nAnimation event (v0.85c+): reactivates the dummy animation immediately. Useful for switching an NPC's moveset during combat.\n- required: SCAR (Skyrim Combat AI Revamp)";

/// Markdown for `SCAR_ActionData`.
pub(crate) const ACTION_DATA_DOC: &str = "# SCAR_ActionData\nAn action (idle or attack) the NPC may perform. The time is also the priority: higher times are checked first, and the first entry whose conditions all pass is executed.\n\n# Format\n```hkanno\n<weight> SCAR_ActionData{\"Key\":\"Value\", ...}\n```\n# Example\n```hkanno\n1.000000 SCAR_ActionData{\"AttackData\":\"attackStart\", \"MinDistance\":0, \"MaxDistance\":139, \"StartAngle\":-60, \"EndAngle\":60, \"Chance\":100, \"Type\":\"RA\"}\n```\n- required: SCAR (Skyrim Combat AI Revamp)";

/// The cursor position inside the JSON body of `SCAR_ActionData{...}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JsonPos {
    /// Where a key is expected. `quoted`: its opening `"` is already typed.
    Key { quoted: bool, used: Vec<String> },
    /// Where the value of `key` is expected.
    Value { key: String, quoted: bool },
    /// Anywhere else (e.g., after a value, before `,`).
    Other,
}

/// Where `before_cursor` (the JSON body from `{` up to the cursor) ends.
///
/// e.g., `{"Min` -> key; `{"Type":"` -> value of `Type`; `{"Chance":3` -> other
pub(crate) fn json_position(before_cursor: &str) -> JsonPos {
    #[derive(PartialEq)]
    enum Expect {
        Key,
        Value,
        Other,
    }
    let mut expect = Expect::Other;
    let (mut in_string, mut escaped) = (false, false);
    let (mut current, mut last_key) = (String::new(), String::new());
    let mut used = Vec::new();

    for c in before_cursor.chars() {
        if in_string {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => {
                    in_string = false;
                    if expect == Expect::Key {
                        last_key = std::mem::take(&mut current);
                        used.push(last_key.clone());
                    }
                    expect = Expect::Other;
                }
                _ => current.push(c),
            }
            continue;
        }
        match c {
            '{' | ',' => expect = Expect::Key,
            ':' => expect = Expect::Value,
            '"' => {
                in_string = true;
                current.clear();
            }
            c if c.is_whitespace() => {}
            _ => expect = Expect::Other, // a number / literal value
        }
    }

    match expect {
        Expect::Key => JsonPos::Key { quoted: in_string, used },
        Expect::Value => JsonPos::Value { key: last_key, quoted: in_string },
        Expect::Other if in_string => JsonPos::Other,
        Expect::Other => JsonPos::Other,
    }
}

/// A problem of a `SCAR_ActionData` JSON object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Problem {
    pub(crate) is_error: bool,
    pub(crate) message: String,
    /// The key it is about (to point at it), e.g. `Chance`.
    pub(crate) key: Option<String>,
    /// The known key `key` was probably meant to be (quick fix).
    pub(crate) rename: Option<&'static str>,
}

const fn problem(is_error: bool, message: String, key: Option<String>) -> Problem {
    Problem { is_error, message, key, rename: None }
}

/// Problems of a `SCAR_ActionData` JSON object.
pub(crate) fn check_action_data(json: &str) -> Vec<Problem> {
    let value: serde_json::Value = match serde_json::from_str(json) {
        Ok(value) => value,
        Err(err) => {
            return vec![problem(true, format!("SCAR_ActionData JSON parse error: {err}"), None)];
        }
    };
    let Some(object) = value.as_object() else {
        return vec![problem(
            true,
            "SCAR_ActionData must be a JSON object `{...}`.".to_string(),
            None,
        )];
    };

    let mut problems = Vec::new();
    for (name, value) in object {
        let Some(key) = action_key(name) else {
            let rename = closest_key(name);
            let message = match rename {
                Some(known) => {
                    format!("Unknown SCAR_ActionData key `{name}`. Did you mean `{known}`?")
                }
                None => format!("Unknown SCAR_ActionData key `{name}`."),
            };
            problems.push(Problem { rename, ..problem(false, message, Some(name.clone())) });
            continue;
        };
        if let Some(message) = check_value(key, value) {
            problems.push(problem(true, message, Some(name.clone())));
        }
    }

    let has = |name: &str| object.contains_key(name);
    match (has("IdleAnimation"), has("AttackData")) {
        (true, true) => problems.push(problem(
            true,
            "`IdleAnimation` and `AttackData` cannot be combined.".to_string(),
            Some("AttackData".to_string()),
        )),
        (false, false) => problems.push(problem(
            false,
            "Neither `IdleAnimation` nor `AttackData` is set: nothing is performed.".to_string(),
            None,
        )),
        _ => {}
    }
    if has("StartAngle") != has("EndAngle") {
        problems.push(problem(
            false,
            "`StartAngle` and `EndAngle` are used together.".to_string(),
            None,
        ));
    }
    if has("CoolDownAlias") && !has("IdleAnimation") {
        problems.push(problem(
            false,
            "`CoolDownAlias` is meant to be used with `IdleAnimation`.".to_string(),
            None,
        ));
    }
    if has("ConditionsAlias") && !has("AttackData") {
        problems.push(problem(
            false,
            "`ConditionsAlias` is meant to be used with `AttackData`.".to_string(),
            None,
        ));
    }
    problems
}

fn check_value(key: &ActionKey, value: &serde_json::Value) -> Option<String> {
    let name = key.name;
    match key.kind {
        ValueKind::String => (!value.is_string()).then(|| format!("`{name}` must be a string.")),
        ValueKind::ActionType => {
            let valid = value.as_str().is_some_and(|t| ACTION_TYPES.iter().any(|(v, _)| *v == t));
            (!valid).then(|| {
                format!("`{name}` must be one of RA, RPA, LA, LPA, DA, DPA, BA, BPA, IDLE.")
            })
        }
        ValueKind::PositiveInt => {
            let valid = value.as_u64().is_some_and(|n| n > 0);
            (!valid).then(|| format!("`{name}` must be an integer greater than 0."))
        }
        ValueKind::Number { min, max } => {
            let Some(n) = value.as_f64() else {
                return Some(format!("`{name}` must be a number."));
            };
            let low = min.is_some_and(|min| n < min);
            let high = max.is_some_and(|max| n > max);
            (low || high).then(|| match (min, max) {
                (Some(min), Some(max)) => format!("`{name}` must be between {min} and {max}."),
                (Some(min), None) => format!("`{name}` must be at least {min}."),
                _ => format!("`{name}` is out of range."),
            })
        }
    }
}

/// `SCAR_ComboStart.<chance>`: problem with the suffix, if any.
pub(crate) fn check_combo_start(text: &str) -> Option<String> {
    let suffix = text.trim_end().strip_prefix(COMBO_START)?.strip_prefix('.')?;
    let valid = suffix.parse::<f64>().is_ok_and(|chance| (0.0..=100.0).contains(&chance));
    (!valid).then(|| {
        format!("`SCAR_ComboStart.<chance>` needs a chance from 0 to 100, but got `{suffix}`.")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions() {
        assert_eq!(json_position("{"), JsonPos::Key { quoted: false, used: vec![] });
        assert_eq!(json_position("{\""), JsonPos::Key { quoted: true, used: vec![] });
        assert_eq!(
            json_position("{\"Type\":\"RA\", \""),
            JsonPos::Key { quoted: true, used: vec!["Type".to_string()] }
        );
        assert_eq!(
            json_position("{\"Type\":\""),
            JsonPos::Value { key: "Type".to_string(), quoted: true }
        );
        assert_eq!(
            json_position("{\"Chance\": "),
            JsonPos::Value { key: "Chance".to_string(), quoted: false }
        );
        assert_eq!(json_position("{\"Chance\":3"), JsonPos::Other);
    }

    #[test]
    fn checks() {
        let ok = r#"{"AttackData":"attackStart", "MinDistance":0, "MaxDistance":139, "StartAngle":-60, "EndAngle":60, "Chance":100, "Type":"RA"}"#;
        assert!(check_action_data(ok).is_empty());

        let bad = r#"{"IdleAnimation":"a", "AttackData":"b", "Type":"XX", "Chance":200, "Foo":1}"#;
        let messages: Vec<_> = check_action_data(bad).into_iter().map(|p| p.message).collect();
        assert!(messages.iter().any(|m| m.contains("cannot be combined")));
        assert!(messages.iter().any(|m| m.contains("`Type` must be one of")));
        assert!(messages.iter().any(|m| m.contains("`Chance` must be between")));
        assert!(messages.iter().any(|m| m.contains("Unknown SCAR_ActionData key `Foo`")));

        assert_eq!(check_combo_start("SCAR_ComboStart.60"), None);
        assert!(check_combo_start("SCAR_ComboStart.x").is_some());
        assert_eq!(closest_key("chanse"), Some("Chance"));
        assert_eq!(closest_key("Foo"), None);
        assert_eq!(check_combo_start("SCAR_ComboStart"), None);
    }
}
