//! What the file's names and the `when` names mean, in one place.
//!
//! Both the schema written for other editors ([`crate::file::schema`]) and the app's own editor of
//! the file ([`crate::json`]) take them from here, so that they say the same.

/// The names of an entry, with what they are. The commands' are in [`crate::COMMANDS`].
pub const ENTRY_FIELDS: [(&str, &str); 4] = [
    (
        "key",
        "Modifiers (ctrl, shift, alt) and a key joined by `+`; a sequence split by spaces. e.g. `ctrl+shift+l`, `space e`",
    ),
    ("command", "The command; `-` before it takes that default away. e.g. `tabs.close`, `-tabs.close`"),
    ("when", "Where it works. e.g. `editorFocus && vim.mode == 'normal'`"),
    ("args", "For the command, if it takes any."),
];

/// The names of the file itself.
pub const FILE_FIELDS: [(&str, &str); 2] = [
    ("$schema", "The schema of this file: completion and checks in an editor such as VS Code."),
    ("bindings", "The entries: what is changed of the defaults. An entry after another wins."),
];

/// The names a `when` can use ([`crate::KeyContext`]), with what they mean and the values they take.
pub const CONTEXT: [(&str, &str, &[&str]); 10] = [
    ("page", "The page shown.", &["patchEditor", "hkanno"]),
    ("editorFocus", "The code editor has the keyboard.", &[]),
    ("vim", "The Vim keymap is on.", &[]),
    (
        "vim.mode",
        "Vim's mode. `pending`: a command is being typed (`2d`); `command`: the `:` / `/` line.",
        &["normal", "insert", "visual", "visualLine", "visualBlock", "pending", "command"],
    ),
    ("explorerFocus", "The object tree / list has the keyboard.", &[]),
    ("explorer.view", "What the explorer shows.", &["tree", "list"]),
    ("problemsFocus", "The Problems panel has the keyboard.", &[]),
    ("problemsVisible", "The Problems panel shows.", &[]),
    ("inputFocus", "A text field that is not the editor has the keyboard.", &[]),
    ("userPatch", "In the user patch window.", &[]),
];

/// What a name of the file is, entry's or the file's own. e.g. `key` -> `Modifiers (ctrl, ...`
#[must_use]
pub fn field(name: &str) -> Option<&'static str> {
    ENTRY_FIELDS.iter().chain(&FILE_FIELDS).find(|(known, _)| *known == name).map(|(_, doc)| *doc)
}

/// The description of `when` in the schema: what it is, and every name with what it means.
#[must_use]
pub fn when_description() -> String {
    let mut out = String::from(ENTRY_FIELDS[2].1);
    out.push_str("\n\nNames:");
    for (name, doc, values) in CONTEXT {
        out.push_str(&format!("\n- {name}: {doc}"));
        if !values.is_empty() {
            out.push_str(&format!(" ({})", values.join(", ")));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every name of `KeyContext` is told of, and nothing else.
    #[test]
    fn the_context_names_are_those_of_key_context() {
        let context = crate::KeyContext::default();
        for (name, ..) in CONTEXT {
            assert!(crate::Context::value(&context, name).is_some() || name.contains('.'), "{name}");
        }
        assert_eq!(field("key"), Some(ENTRY_FIELDS[0].1));
        assert!(field("$schema").is_some() && field("nokey").is_none());
    }
}
