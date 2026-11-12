//! What can follow a key that waits for another (like Neovim's which-key).
//!
//! ```text
//! Space            <- `Space` was pressed and nothing followed it for a moment
//!   f  Format
//!   w  Format and save
//!   l  Next tab
//! ```
//!
//! The parser in [`super::parse`] decides what the keys do; this table only describes them. A test
//! compares the two over every key, so that a new command cannot be left out of the list.

use super::{keys::VimKey, parse::take_count};
use crate::EditorCommand;

/// A key that waits for another, with a list to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Prefix {
    Leader,
    G,
}

impl Prefix {
    /// The heading of the list.
    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::Leader => "Space",
            Self::G => "g",
        }
    }

    const fn bindings(self) -> &'static [WhichKey] {
        match self {
            Self::Leader => LEADER,
            Self::G => G,
        }
    }

    /// What the list shows. `host_commands`: what the host acts on; the keys of the rest do nothing,
    /// so they are not offered.
    pub(crate) fn hints(self, host_commands: &[EditorCommand]) -> impl Iterator<Item = &'static WhichKey> {
        self.bindings().iter().filter(move |binding| binding.needs.is_none_or(|needs| host_commands.contains(&needs)))
    }
}

/// A key after a [`Prefix`], and what it does.
#[derive(Debug)]
pub(crate) struct WhichKey {
    /// The whole key sequence. e.g., ` f`, `gd`
    keys: &'static str,
    /// What it does. In English, like the keys themselves and the rest of Vim's prompt.
    pub(crate) text: &'static str,
    /// Only does something if the host acts on this command (e.g., the tabs).
    needs: Option<EditorCommand>,
}

impl WhichKey {
    /// The key to press. e.g., `f` of ` f`
    pub(crate) fn key(&self) -> &'static str {
        &self.keys[1..]
    }
}

const fn own(keys: &'static str, text: &'static str) -> WhichKey {
    WhichKey { keys, text, needs: None }
}

const fn host(keys: &'static str, command: EditorCommand, text: &'static str) -> WhichKey {
    WhichKey { keys, text, needs: Some(command) }
}

const LEADER: &[WhichKey] = &[
    own(" f", "Format"),
    own(" w", "Format and save"),
    host(" c", EditorCommand::CloseTab, "Close the tab"),
    host(" p", EditorCommand::TogglePreview, "Show or hide the preview"),
    host(" d", EditorCommand::ToggleInlineDiff, "Show or hide the inline diff"),
    host(" e", EditorCommand::ToggleExplorer, "Move to the tree"),
];

const G: &[WhichKey] = &[
    own("gg", "First line"),
    own("gd", "Go to definition"),
    own("gD", "Find references"),
    host("gt", EditorCommand::NextTab, "Next tab"),
    host("gT", EditorCommand::PreviousTab, "Previous tab"),
    own("gx", "Open the URL under the cursor"),
    own("ge", "End of the previous word"),
    own("gE", "End of the previous WORD"),
    own("gi", "Insert where the last insert ended"),
    own("gI", "Insert at column 0"),
    own("gJ", "Join lines without a space"),
    own("gv", "Select the last selection again"),
    own("g*", "Search the word, also inside others"),
    own("g#", "Search the word backward, also inside others"),
    own("gu", "Lowercase (then a motion)"),
    own("gU", "Uppercase (then a motion)"),
    own("g~", "Toggle the case (then a motion)"),
];

/// The prefix the pending keys wait on, with what may come before it: `Space`, `3g`, `"ag`.
pub(super) fn waiting_prefix(keys: &[VimKey]) -> Option<Prefix> {
    let keys = match keys {
        [VimKey::Char('"'), VimKey::Char(_), rest @ ..] => rest,
        _ => keys,
    };
    match take_count(keys).1 {
        [VimKey::Char(' ')] => Some(Prefix::Leader),
        [VimKey::Char('g')] => Some(Prefix::G),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::parse::{Parse, parse},
        *,
    };

    /// `gj`, ...: on lines that do not wrap, they are the same as the key without `g`.
    const G_NOT_LISTED: &str = "jk0^$";

    fn listed_keys(prefix: Prefix, not_listed: &str) -> Vec<char> {
        let mut keys: Vec<char> = prefix.bindings().iter().flat_map(|binding| binding.key().chars()).collect();
        keys.extend(not_listed.chars());
        keys.sort_unstable();
        keys
    }

    /// Every printable key the parser knows after the prefix.
    fn parsed_keys(prefix: char) -> Vec<char> {
        let mut keys: Vec<char> =
            (' '..='~').filter(|&key| parse(&[VimKey::Char(prefix), VimKey::Char(key)]) != Parse::Invalid).collect();
        keys.sort_unstable();
        keys
    }

    #[test]
    fn the_lists_have_every_key_of_the_parser() {
        assert_eq!(listed_keys(Prefix::Leader, ""), parsed_keys(' '));
        assert_eq!(listed_keys(Prefix::G, G_NOT_LISTED), parsed_keys('g'));
    }

    #[test]
    fn a_host_is_offered_what_it_acts_on() {
        let keys = |prefix: Prefix, host: &[EditorCommand]| prefix.hints(host).map(WhichKey::key).collect::<Vec<_>>();
        assert_eq!(keys(Prefix::Leader, &[]), ["f", "w"]);
        assert_eq!(keys(Prefix::Leader, &[EditorCommand::CloseTab]), ["f", "w", "c"]);
        assert!(!keys(Prefix::G, &[]).contains(&"t"));
        assert!(keys(Prefix::G, &[EditorCommand::NextTab]).contains(&"t"));
        assert!(keys(Prefix::G, &[]).contains(&"d"), "the language answers that");
    }

    #[test]
    fn what_waits_on_a_prefix() {
        let keys = |s: &str| s.chars().map(VimKey::Char).collect::<Vec<_>>();
        assert_eq!(waiting_prefix(&keys(" ")), Some(Prefix::Leader));
        assert_eq!(waiting_prefix(&keys("g")), Some(Prefix::G));
        assert_eq!(waiting_prefix(&keys("3g")), Some(Prefix::G));
        assert_eq!(waiting_prefix(&keys("\"ag")), Some(Prefix::G));
        assert_eq!(waiting_prefix(&keys("dg")), None, "a motion of an operator");
        assert_eq!(waiting_prefix(&keys("")), None);
        assert_eq!(waiting_prefix(&keys("d")), None);
    }
}
