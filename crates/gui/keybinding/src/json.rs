//! `keybindings.json` as a language of the code editor: what VS Code gives the file from its schema.
//!
//! | feature     | what                                                                            |
//! | ----------- | ------------------------------------------------------------------------------- |
//! | highlight   | JSON with comments: names, texts, numbers, comments                             |
//! | completion  | `"command"`: the commands (and `-` ones); `"key"`: modifiers and keys;          |
//! |             | `"when"`: the context names and their values; a name: `key` `command` `when` `args` |
//! | hover       | a command: what it does and its default keys; a key: who else has it;           |
//! |             | a `when` name: what it means                                                    |
//! | diagnostics | [`diagnose`]: not JSON, no command, no key, a broken `when`, a `-command` that  |
//! |             | takes nothing away, a name the file has no use for                              |
//!
//! The lines are looked at one by one, as the editor asks: an entry is best on one line, as VS Code
//! writes them, but a value on a line of its own with its name works the same.

use std::{borrow::Cow, ops::Range};

use egui::Color32;
use egui_code_editor::{CompletionItem, CompletionList, Diagnostic, Language, Severity};

use crate::{COMMANDS, Keybindings, When, commands, file, key, keys_text, parse_keys};

/// The names of [`crate::KeyContext`], with what they mean and the values they take.
const CONTEXT_NAMES: [(&str, &str, &[&str]); 10] = [
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

/// The names an entry has.
const ENTRY_NAMES: [(&str, &str); 4] = [
    (
        "key",
        "Modifiers (ctrl, shift, alt) and a key joined by `+`; a sequence split by spaces. e.g. `ctrl+shift+l`, `space e`",
    ),
    ("command", "The command; `-` before it takes that default away. e.g. `tabs.close`, `-tabs.close`"),
    ("when", "Where it works. e.g. `editorFocus && vim.mode == 'normal'`"),
    ("args", "For the command, if it takes any."),
];

/// The names of the file itself.
const FILE_NAMES: [&str; 2] = ["$schema", "bindings"];

/// The language. `bindings`: the table in force, for the hover of a key.
#[derive(Debug, Clone, Copy)]
pub struct KeybindingsJson<'a> {
    pub bindings: &'a Keybindings,
}

/// A piece of a line, in chars.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    kind: Kind,
    range: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Comment,
    /// A text before `:`: the name of a value.
    Name,
    /// A text that is a value.
    Text,
    Number,
    Word,
    Punct(char),
}

/// The tokens of `line`. A text not closed on the line runs to its end (it is being typed).
fn tokens(chars: &[char]) -> Vec<Token> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        let start = at;
        let kind = match c {
            '/' if chars.get(at + 1) == Some(&'/') => {
                at = chars.len();
                Kind::Comment
            }
            '/' if chars.get(at + 1) == Some(&'*') => {
                at += 2;
                while at < chars.len() && !(chars[at - 1] == '*' && chars[at] == '/' && at > start + 2) {
                    at += 1;
                }
                at = (at + 1).min(chars.len());
                Kind::Comment
            }
            '"' => {
                at += 1;
                while at < chars.len() && chars[at] != '"' {
                    at += if chars[at] == '\\' { 2 } else { 1 };
                }
                at = (at + 1).min(chars.len());
                let next = chars[at..].iter().find(|c| !c.is_whitespace());
                if next == Some(&':') { Kind::Name } else { Kind::Text }
            }
            c if c.is_ascii_digit() || c == '-' => {
                at += 1;
                while at < chars.len() && (chars[at].is_ascii_alphanumeric() || matches!(chars[at], '.' | '+' | '-')) {
                    at += 1;
                }
                Kind::Number
            }
            c if c.is_alphabetic() => {
                while at < chars.len() && chars[at].is_alphanumeric() {
                    at += 1;
                }
                Kind::Word
            }
            c if c.is_whitespace() => {
                at += 1;
                continue;
            }
            c => {
                at += 1;
                Kind::Punct(c)
            }
        };
        out.push(Token { kind, range: start..at });
    }
    out
}

/// The text value at `column` and the name it is of, if it is one: `(name, the chars inside the
/// quotes)`. e.g. `{ "command": "tabs.cl|" }` -> `("command", 14..21)`
fn value_at(chars: &[char], tokens: &[Token], column: usize) -> Option<(String, Range<usize>)> {
    let at = tokens.iter().position(|t| t.kind == Kind::Text && t.range.start < column && column <= t.range.end)?;
    let inner = inside(chars, &tokens[at].range);
    if column > inner.end + usize::from(closed(chars, &tokens[at].range)) {
        return None; // after the closing quote
    }
    let name = match tokens.get(at.wrapping_sub(2)..at)? {
        [Token { kind: Kind::Name, range }, Token { kind: Kind::Punct(':'), .. }] => {
            chars[inside(chars, range)].iter().collect()
        }
        _ => return None,
    };
    Some((name, inner))
}

/// The name being typed at `column`, if any: the chars inside its quotes. A text with no `:` after it
/// yet is a name being typed when it is the first thing of an object or follows a `,`.
fn name_at(chars: &[char], tokens: &[Token], column: usize) -> Option<Range<usize>> {
    let at = tokens.iter().position(|t| t.range.start < column && column <= t.range.end)?;
    let token = &tokens[at];
    let is_name = token.kind == Kind::Name
        || (token.kind == Kind::Text
            && matches!(tokens.get(at.wrapping_sub(1)).map(|t| t.kind), Some(Kind::Punct('{' | ',')) | None));
    is_name.then(|| inside(chars, &token.range))
}

/// The chars of a text inside its quotes.
fn inside(chars: &[char], range: &Range<usize>) -> Range<usize> {
    let end = if closed(chars, range) { range.end - 1 } else { range.end };
    (range.start + 1).min(end)..end
}

fn closed(chars: &[char], range: &Range<usize>) -> bool {
    range.end > range.start + 1 && chars.get(range.end - 1) == Some(&'"')
}

impl Language for KeybindingsJson<'_> {
    fn comment(&self) -> Option<egui_code_editor::Comment> {
        Some(egui_code_editor::Comment::Line("//"))
    }

    fn highlight(&self, line: &str, visuals: &egui::Visuals) -> Vec<(Range<usize>, Color32)> {
        let dark = visuals.dark_mode;
        let rgb = |d: (u8, u8, u8), l: (u8, u8, u8)| {
            let (r, g, b) = if dark { d } else { l };
            Color32::from_rgb(r, g, b)
        };
        let chars: Vec<char> = line.chars().collect();
        tokens(&chars)
            .into_iter()
            .filter_map(|token| {
                let color = match token.kind {
                    Kind::Comment => rgb((127, 132, 142), (160, 161, 167)),
                    Kind::Name => rgb((224, 108, 117), (228, 86, 73)),
                    Kind::Text => rgb((152, 195, 121), (80, 161, 79)),
                    Kind::Number | Kind::Word => rgb((209, 154, 102), (152, 104, 1)),
                    Kind::Punct(_) => return None,
                };
                Some((token.range, color))
            })
            .collect()
    }

    fn completion_triggers(&self) -> &[char] {
        &['"', '+', ' ', '.', '\'', '-']
    }

    fn complete(&self, line: &str, column: usize) -> CompletionList {
        let chars: Vec<char> = line.chars().collect();
        let tokens = tokens(&chars);
        if let Some((name, inner)) = value_at(&chars, &tokens, column) {
            let typed: String = chars[inner.start..column.min(inner.end).max(inner.start)].iter().collect();
            return match name.as_str() {
                "command" => complete_command(inner),
                "key" => complete_key(&chars, inner.start, column),
                "when" => complete_when(&chars, inner.start, column, &typed),
                _ => CompletionList::default(),
            };
        }
        if let Some(inner) = name_at(&chars, &tokens, column) {
            let items = ENTRY_NAMES.iter().map(|(name, doc)| item(name, name, doc)).collect();
            return CompletionList { replace: inner, items };
        }
        CompletionList::default()
    }

    fn hover(&self, line: &str, column: usize) -> Option<String> {
        let chars: Vec<char> = line.chars().collect();
        let tokens = tokens(&chars);
        let (name, inner) = value_at(&chars, &tokens, column)?;
        let value: String = chars[inner.clone()].iter().collect();
        match name.as_str() {
            "command" => {
                let id = value.strip_prefix('-').unwrap_or(&value);
                let command = commands::command(id)?;
                let mut doc = format!("**{}**\n\n{}", command.id, command.description);
                if value.starts_with('-') {
                    doc.push_str("\n\n`-`: takes these default bindings away.");
                }
                push_defaults(&mut doc, command);
                Some(doc)
            }
            "key" => {
                let keys = parse_keys(&value).ok()?;
                let mut doc = format!("`{}`", keys_text(&keys));
                let mut others = self.bindings.bound_to(&keys).peekable();
                if others.peek().is_some() {
                    doc.push_str("\n\nBound to:");
                    for (command, when) in others {
                        doc.push_str(&format!("\n- `{command}`"));
                        if let Some(when) = when {
                            doc.push_str(&format!(" when `{when}`"));
                        }
                    }
                }
                Some(doc)
            }
            "when" => {
                let word = word_around(&chars, inner.start, column);
                let (name, doc, values) = CONTEXT_NAMES.iter().find(|(name, _, _)| *name == word)?;
                let mut out = format!("**{name}**\n\n{doc}");
                if !values.is_empty() {
                    out.push_str(&format!("\n\nValues: {}", values.join(", ")));
                }
                Some(out)
            }
            _ => None,
        }
    }
}

/// The word of a `when` at `column`: a name of the context. e.g. `vim.mode`
fn word_around(chars: &[char], start: usize, column: usize) -> String {
    let is_word = |c: char| c.is_alphanumeric() || c == '.';
    let from = (start..column).rev().take_while(|&at| is_word(chars[at])).last().unwrap_or(column);
    let to = (column..chars.len()).take_while(|&at| is_word(chars[at])).last().map_or(column, |at| at + 1);
    chars[from..to].iter().collect()
}

fn push_defaults(doc: &mut String, command: &commands::Command) {
    if command.defaults.is_empty() {
        doc.push_str("\n\nNo default keys.");
        return;
    }
    doc.push_str("\n\nDefaults:");
    for (keys, when) in command.defaults {
        doc.push_str(&format!("\n- `{keys}`"));
        if let Some(when) = when {
            doc.push_str(&format!(" when `{when}`"));
        }
    }
}

const fn item(label: &'static str, insert: &'static str, doc: &'static str) -> CompletionItem {
    CompletionItem {
        label: Cow::Borrowed(label),
        icon: "",
        insert: Cow::Borrowed(insert),
        select: None,
        documentation: Cow::Borrowed(doc),
    }
}

/// The commands, and their `-` (take a default away), in place of the whole value.
fn complete_command(replace: Range<usize>) -> CompletionList {
    let mut items = Vec::with_capacity(COMMANDS.len() * 2);
    for command in COMMANDS {
        let mut doc = command.description.to_owned();
        push_defaults(&mut doc, command);
        items.push(CompletionItem {
            label: Cow::Borrowed(command.id),
            icon: "",
            insert: Cow::Borrowed(command.id),
            select: None,
            documentation: Cow::Owned(doc),
        });
    }
    for command in COMMANDS {
        let id = format!("-{}", command.id);
        items.push(CompletionItem {
            label: Cow::Owned(id.clone()),
            icon: "",
            insert: Cow::Owned(id),
            select: None,
            documentation: Cow::Owned(format!("Takes the default bindings of `{}` away.", command.id)),
        });
    }
    CompletionList { replace, items }
}

/// The modifiers and keys, in place of the part of a chord being typed (after `+` or a space).
fn complete_key(chars: &[char], start: usize, column: usize) -> CompletionList {
    let from = (start..column).rev().find(|&at| matches!(chars[at], '+' | ' ')).map_or(start, |at| at + 1);
    let mut items: Vec<CompletionItem> = key::MODIFIERS
        .iter()
        .map(|modifier| CompletionItem {
            label: Cow::Owned(format!("{modifier}+")),
            icon: "",
            insert: Cow::Owned(format!("{modifier}+")),
            select: None,
            documentation: Cow::Borrowed("A modifier: a key follows it."),
        })
        .collect();
    items.extend(key::key_names().map(|name| item(name, name, "")));
    CompletionList { replace: from..column, items }
}

/// The context names, or (after `name == '`) the values of that name.
fn complete_when(chars: &[char], start: usize, column: usize, typed: &str) -> CompletionList {
    let is_word = |c: char| c.is_alphanumeric() || c == '.';
    let from = (start..column).rev().take_while(|&at| is_word(chars[at])).last().unwrap_or(column);
    let before = typed[..typed.len() - chars[from..column].iter().map(|c| c.len_utf8()).sum::<usize>()].trim_end();
    // `vim.mode == '` or `vim.mode != "`: its values.
    let compared = before.strip_suffix(['\'', '"']).map(str::trim_end).and_then(|rest| {
        let rest = rest.strip_suffix("==").or_else(|| rest.strip_suffix("!="))?.trim_end();
        CONTEXT_NAMES.iter().find(|(name, _, _)| rest.ends_with(name))
    });
    let items = match compared {
        Some((_, _, values)) => values.iter().map(|value| item(value, value, "")).collect(),
        None => CONTEXT_NAMES.iter().map(|(name, doc, _)| item(name, name, doc)).collect(),
    };
    CompletionList { replace: from..column, items }
}

/// The problems of the whole file, as the editor underlines them. See the module docs.
#[must_use]
pub fn diagnose(text: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let entries = match file::parse_at(text) {
        Ok(entries) => Some(entries),
        Err((message, line, column)) => {
            let line = line.saturating_sub(1);
            let column = column.saturating_sub(1);
            out.push(Diagnostic { line, columns: column..column + 1, severity: Severity::Error, message });
            None
        }
    };
    // Where each entry's command is, in the order of the entries.
    let mut command_places = Vec::new();
    for (line_no, line) in text.split('\n').enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let tokens = tokens(&chars);
        for (at, token) in tokens.iter().enumerate() {
            if token.kind != Kind::Name {
                continue;
            }
            let name: String = chars[inside(&chars, &token.range)].iter().collect();
            if !ENTRY_NAMES.iter().any(|(known, _)| *known == name) && !FILE_NAMES.contains(&name.as_str()) {
                let message = format!("`{name}` is not a name of this file: key, command, when, args");
                out.push(Diagnostic {
                    line: line_no,
                    columns: token.range.clone(),
                    severity: Severity::Warning,
                    message,
                });
                continue;
            }
            let Some(value) = tokens.get(at + 2).filter(|t| t.kind == Kind::Text) else { continue };
            let inner = inside(&chars, &value.range);
            let text: String = chars[inner.clone()].iter().collect();
            let problem = match name.as_str() {
                "command" => {
                    command_places.push((line_no, inner.clone()));
                    let id = text.strip_prefix('-').unwrap_or(&text);
                    commands::command(id).is_none().then(|| format!("`{id}` is no command"))
                }
                "key" => parse_keys(&text).err(),
                "when" => When::parse(&text).err().map(|err| format!("when: {err}")),
                _ => None,
            };
            if let Some(message) = problem {
                out.push(Diagnostic { line: line_no, columns: inner, severity: Severity::Error, message });
            }
        }
    }
    // What only the whole table tells: a `-command` that takes nothing away.
    for (at, why) in entries.as_deref().map(crate::entry_problems).unwrap_or_default() {
        if why == crate::TAKES_NOTHING
            && let Some((line, columns)) = command_places.get(at)
        {
            let message = why;
            out.push(Diagnostic { line: *line, columns: columns.clone(), severity: Severity::Warning, message });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column_of(line: &str, marker: &str) -> usize {
        line[..line.find(marker).unwrap()].chars().count()
    }

    #[test]
    fn a_command_value_completes_the_commands() {
        let line = r#"    { "key": "f9", "command": "tabs.cl" },"#;
        let keys = Keybindings::default();
        let language = KeybindingsJson { bindings: &keys };
        let list = language.complete(line, column_of(line, r#"" }"#));
        assert!(list.items.iter().any(|item| item.label == "tabs.close"));
        assert!(list.items.iter().any(|item| item.label == "-tabs.close"));
        assert_eq!(list.replace, column_of(line, "tabs.cl")..column_of(line, r#"" }"#));
    }

    #[test]
    fn a_key_completes_the_part_being_typed() {
        let line = r#"{ "key": "ctrl+pa", "command": "tabs.next" }"#;
        let keys = Keybindings::default();
        let list = KeybindingsJson { bindings: &keys }.complete(line, column_of(line, r#"", "command""#));
        assert!(list.items.iter().any(|item| item.label == "pagedown"));
        assert_eq!(list.replace.start, column_of(line, "pa\""));
    }

    #[test]
    fn a_when_completes_names_and_their_values() {
        let keys = Keybindings::default();
        let language = KeybindingsJson { bindings: &keys };
        let line = r#"{ "when": "editorFocus && vim.mode == 'no" }"#;
        let list = language.complete(line, column_of(line, r#"" }"#));
        assert!(list.items.iter().any(|item| item.label == "normal"), "{list:?}");
        let line = r#"{ "when": "editorFocus && pro" }"#;
        let list = language.complete(line, column_of(line, r#"" }"#));
        assert!(list.items.iter().any(|item| item.label == "problemsFocus"));
    }

    #[test]
    fn a_name_completes_the_names_of_an_entry() {
        let keys = Keybindings::default();
        let line = r#"{ "key": "a", "com" }"#;
        let list = KeybindingsJson { bindings: &keys }.complete(line, column_of(line, r#"" }"#));
        assert!(list.items.iter().any(|item| item.label == "command"));
    }

    #[test]
    fn hovers_tell_what_it_is() {
        let keys = Keybindings::default();
        let language = KeybindingsJson { bindings: &keys };
        let line = r#"{ "key": "ctrl+shift+l", "command": "problems.toggle", "when": "editorFocus" }"#;
        let command = language.hover(line, column_of(line, "toggle")).unwrap();
        assert!(command.contains("Problems panel") && command.contains("ctrl+shift+l"), "{command}");
        let key = language.hover(line, column_of(line, "shift+l")).unwrap();
        assert!(key.contains("problems.toggle"), "{key}");
        let when = language.hover(line, column_of(line, "Focus\" }")).unwrap();
        assert!(when.contains("editorFocus"), "{when}");
    }

    #[test]
    fn problems_are_where_they_are() {
        let text = "{\n  \"bindings\": [\n    { \"key\": \"hyper+q\", \"command\": \"tabs.closee\", \"when\": \"a &&\" },\n    { \"key\": \"f9\", \"command\": \"-tabs.close\" },\n    { \"key\": \"f8\", \"command\": \"tabs.next\", \"whne\": \"x\" },\n  ],\n}";
        let problems = diagnose(text);
        let at = |needle: &str| problems.iter().find(|p| p.message.contains(needle)).map(|p| p.line);
        assert_eq!(at("hyper"), Some(2));
        assert_eq!(at("tabs.closee"), Some(2));
        assert_eq!(at("when:"), Some(2));
        assert_eq!(at(crate::TAKES_NOTHING), Some(3), "no `f9` of `tabs.close` to take away");
        assert_eq!(at("whne"), Some(4));

        let text = "[\n  { \"key\": \"f9\", \"command\": \"-tabs.close\" },\n]";
        assert_eq!(diagnose(text).iter().map(|p| p.line).collect::<Vec<_>>(), [1], "takes nothing away");
        let broken = diagnose("{ \"bindings\": [ { \"key\": } ] }");
        assert_eq!(broken[0].severity, Severity::Error);
        assert!(diagnose(&file::template()).is_empty(), "the new file is fine");
    }
}
