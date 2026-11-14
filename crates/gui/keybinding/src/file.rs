//! The user file, `keybindings.json`: what the user changes of the defaults.
//!
//! ```jsonc
//! {
//!   "$schema": "./keybindings.schema.json",
//!   "bindings": [
//!     // Vim's Normal mode: Shift+H / Shift+L switch tabs (Vim's H / L are then gone)
//!     { "key": "shift+h", "command": "tabs.previous", "when": "editorFocus && vim.mode == 'normal'" },
//!     // No Ctrl+Shift+L for the Problems panel
//!     { "key": "ctrl+shift+l", "command": "-problems.toggle" },
//!   ],
//! }
//! ```
//! VS Code's form, the array alone, is read too (to paste from it), but has no place for `$schema`.

use serde::Deserialize;

use crate::{commands::COMMANDS, jsonc};

/// One entry of the file, as written.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Entry {
    /// e.g. `ctrl+shift+l`, `space e`
    pub key: String,
    /// e.g. `tabs.close`; `-tabs.close` takes that default away.
    pub command: String,
    #[serde(default)]
    pub when: Option<String>,
    /// For the command, if it takes any.
    #[serde(default)]
    pub args: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum FileForm {
    Object {
        #[serde(default)]
        bindings: Vec<Entry>,
    },
    Array(Vec<Entry>),
}

/// The entries of `text`. Empty text is no entry.
///
/// # Errors
/// It is not JSON (with comments) of either form; the message says where.
pub fn parse(text: &str) -> Result<Vec<Entry>, String> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    match serde_json::from_str::<FileForm>(&jsonc::strip(text)) {
        Ok(FileForm::Object { bindings } | FileForm::Array(bindings)) => Ok(bindings),
        Err(err) => Err(err.to_string()),
    }
}

/// Like [`parse`], with where the text stops being right: `(message, line, column)`, both 1-based.
///
/// # Errors
/// See [`parse`].
pub fn parse_at(text: &str) -> Result<Vec<Entry>, (String, usize, usize)> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    match serde_json::from_str::<FileForm>(&jsonc::strip(text)) {
        Ok(FileForm::Object { bindings } | FileForm::Array(bindings)) => Ok(bindings),
        Err(err) => Err((err.to_string(), err.line(), err.column())),
    }
}

/// The file name of the schema, next to the user file.
pub const SCHEMA_FILE: &str = "keybindings.schema.json";

/// What a new user file has: nothing changed, and the schema for the editor that opens it.
///
/// The defaults are in it as comments to start from: a line uncommented is a binding.
#[must_use]
pub fn template() -> String {
    let mut out = String::from(
        "// Key bindings: what you change of the defaults, as VS Code's keybindings.json.\n\
         // An entry after another wins; `\"command\": \"-tabs.close\"` takes that default away.\n\
         {\n",
    );
    out.push_str(&format!("  \"$schema\": \"./{SCHEMA_FILE}\",\n  \"bindings\": [\n"));
    out.push_str("    // The defaults:\n");
    for command in COMMANDS {
        for &(keys, when) in command.defaults {
            out.push_str("    // ");
            out.push_str(&entry_text(keys, command.id, when));
            out.push_str(",\n");
        }
    }
    out.push_str("  ],\n}\n");
    out
}

/// One entry as the file writes it. e.g. `{ "key": "ctrl+w", "command": "tabs.close", "when": "editorFocus" }`
#[must_use]
pub fn entry_text(keys: &str, command: &str, when: Option<&str>) -> String {
    let quoted = |text: &str| serde_json::to_string(text).unwrap_or_default();
    let when = when.map(|when| format!(", \"when\": {}", quoted(when))).unwrap_or_default();
    format!("{{ \"key\": {}, \"command\": {}{when} }}", quoted(keys), quoted(command))
}

/// `text` with `entries` added at the end of its bindings, each on a line.
///
/// They go before the `]` that closes them (the last `]` of the file), with a comma after what was
/// before. Comments and the rest stay as they are. Text that is no file of bindings gets the template
/// first.
///
/// e.g. `{ "bindings": [ {..} ] }` + `{..}` -> `{ "bindings": [ {..},\n    {..},\n] }`
#[must_use]
pub fn add_entries(text: &str, entries: &[String]) -> String {
    let base = if parse(text).is_ok() && !text.trim().is_empty() { text.to_owned() } else { template() };
    let Some((close, last)) = jsonc::last_close_bracket(&base) else { return base };
    // The `]` alone on its line: the entries go on lines of their own above it.
    let line_start = base[..close].rfind('\n').map_or(0, |at| at + 1);
    let at = if base[line_start..close].trim().is_empty() { line_start } else { close };
    let mut out = String::with_capacity(base.len() + entries.len() * 80);
    match last {
        // `[ {..} ]`: the last entry needs its comma.
        Some(last) if !matches!(base.as_bytes()[last], b'[' | b',') => {
            out.push_str(&base[..=last]);
            out.push(',');
            out.push_str(&base[last + 1..at]);
        }
        _ => out.push_str(&base[..at]),
    }
    if at == close {
        out.push('\n');
    }
    for entry in entries {
        out.push_str("    ");
        out.push_str(entry);
        out.push_str(",\n");
    }
    out.push_str(&base[at..]);
    out
}

/// The JSON Schema of the user file: the commands as a list to pick from, each with what it does
/// and its default keys.
#[must_use]
pub fn schema() -> String {
    let mut ids = Vec::with_capacity(COMMANDS.len() * 2);
    let mut descriptions = Vec::with_capacity(COMMANDS.len() * 2);
    for command in COMMANDS {
        let keys: Vec<&str> = command.defaults.iter().map(|(keys, _)| *keys).collect();
        let defaults = if keys.is_empty() { String::new() } else { format!(" ({})", keys.join(", ")) };
        ids.push(command.id.to_owned());
        descriptions.push(format!("{}{defaults}", command.description));
        ids.push(format!("-{}", command.id));
        descriptions.push(format!("Takes away a default binding of: {}", command.description));
    }
    let entry = serde_json::json!({
        "type": "object",
        "required": ["key", "command"],
        "additionalProperties": false,
        "properties": {
            "key": {
                "type": "string",
                "description": crate::docs::ENTRY_FIELDS[0].1,
                "pattern": r"^\s*((ctrl|cmd|control|shift|alt|option)\+)*[^\s+]+\+?(\s+((ctrl|cmd|control|shift|alt|option)\+)*[^\s+]+\+?)*\s*$"
            },
            "command": {
                "type": "string",
                "description": crate::docs::ENTRY_FIELDS[1].1,
                "enum": ids,
                "enumDescriptions": descriptions,
            },
            "when": {
                "type": "string",
                "description": crate::docs::when_description(),
            },
            "args": { "description": crate::docs::ENTRY_FIELDS[3].1 },
        },
    });
    let schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "d-merge key bindings",
        "definitions": { "entry": entry },
        "oneOf": [
            {
                "type": "object",
                "properties": {
                    "$schema": { "type": "string", "description": crate::docs::FILE_FIELDS[0].1 },
                    "bindings": {
                        "type": "array",
                        "description": crate::docs::FILE_FIELDS[1].1,
                        "items": { "$ref": "#/definitions/entry" },
                    },
                },
            },
            { "type": "array", "items": { "$ref": "#/definitions/entry" } },
        ],
    });
    serde_json::to_string_pretty(&schema).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_forms_are_read() {
        let object = r#"{ "$schema": "x", "bindings": [ { "key": "a", "command": "tabs.next", }, ], }"#;
        let array = r#"[ // VS Code's
            { "key": "a", "command": "tabs.next", "when": "editorFocus" } ]"#;
        assert_eq!(parse(object).unwrap()[0].command, "tabs.next");
        assert_eq!(parse(array).unwrap()[0].when.as_deref(), Some("editorFocus"));
        assert!(parse("").unwrap().is_empty());
        assert!(parse(&template()).unwrap().is_empty(), "the new file is read too");
        assert!(parse("{ \"bindings\": 1 }").is_err());
    }

    #[test]
    fn the_template_has_the_defaults_as_comments() {
        let template = template();
        assert!(
            template.contains(r#"// { "key": "ctrl+w", "command": "tabs.close", "when": "editorFocus && !vim" },"#)
        );
        assert!(parse(&template).unwrap().is_empty(), "comments only");
    }

    #[test]
    fn entries_are_added_at_the_end_of_the_bindings() {
        let entry = entry_text("f9", "tabs.close", None);
        let added = add_entries(&template(), std::slice::from_ref(&entry));
        assert_eq!(parse(&added).unwrap().len(), 1);
        assert!(added.contains("// The defaults:"), "comments stay");

        let one = r#"{ "bindings": [ { "key": "a", "command": "tabs.next" } ] }"#;
        let added = add_entries(one, std::slice::from_ref(&entry));
        let entries = parse(&added).unwrap();
        assert_eq!((entries.len(), entries[1].key.as_str()), (2, "f9"), "{added}");

        let array = "[\n  // VS Code's\n  { \"key\": \"a\", \"command\": \"tabs.next\" },\n]";
        assert_eq!(parse(&add_entries(array, std::slice::from_ref(&entry))).unwrap().len(), 2);
        assert_eq!(parse(&add_entries("", &[entry])).unwrap().len(), 1, "an empty file gets the template");
    }

    #[test]
    fn the_schema_lists_the_commands() {
        let schema: serde_json::Value = serde_json::from_str(&schema()).unwrap();
        let ids = &schema["definitions"]["entry"]["properties"]["command"]["enum"];
        assert!(ids.as_array().unwrap().iter().any(|id| id == "tabs.close"));
        assert!(ids.as_array().unwrap().iter().any(|id| id == "-tabs.close"));
    }
}
