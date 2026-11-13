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

/// The file name of the schema, next to the user file.
pub const SCHEMA_FILE: &str = "keybindings.schema.json";

/// What a new user file has: nothing changed, and the schema for the editor that opens it.
#[must_use]
pub fn template() -> String {
    format!(
        "// Key bindings: what you change of the defaults, as VS Code's keybindings.json.\n\
         // e.g. {{ \"key\": \"shift+l\", \"command\": \"tabs.next\", \"when\": \"editorFocus && vim.mode == 'normal'\" }}\n\
         //      {{ \"key\": \"ctrl+shift+l\", \"command\": \"-problems.toggle\" }}  (takes a default away)\n\
         {{\n  \"$schema\": \"./{SCHEMA_FILE}\",\n  \"bindings\": [\n  ],\n}}\n"
    )
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
                "description": "Modifiers (ctrl, shift, alt) and a key joined by `+`; a sequence split by spaces. e.g. `ctrl+shift+l`, `space e`",
                "pattern": r"^\s*((ctrl|cmd|control|shift|alt|option)\+)*[^\s+]+\+?(\s+((ctrl|cmd|control|shift|alt|option)\+)*[^\s+]+\+?)*\s*$"
            },
            "command": {
                "type": "string",
                "description": "The command; `-` before it takes that default away.",
                "enum": ids,
                "enumDescriptions": descriptions,
            },
            "when": {
                "type": "string",
                "description": "Where it works. Names: page, editorFocus, vim, vim.mode, explorerFocus, explorer.view, problemsFocus, problemsVisible, inputFocus, userPatch. e.g. `editorFocus && vim.mode == 'normal'`",
            },
            "args": { "description": "For the command, if it takes any." },
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
                    "$schema": { "type": "string" },
                    "bindings": { "type": "array", "items": { "$ref": "#/definitions/entry" } },
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
    fn the_schema_lists_the_commands() {
        let schema: serde_json::Value = serde_json::from_str(&schema()).unwrap();
        let ids = &schema["definitions"]["entry"]["properties"]["command"]["enum"];
        assert!(ids.as_array().unwrap().iter().any(|id| id == "tabs.close"));
        assert!(ids.as_array().unwrap().iter().any(|id| id == "-tabs.close"));
    }
}
