//! Without the `vim` feature: what the editor asks of Vim (see `vim/`), doing nothing, so that the
//! rest is built the same. [`super::Keymap::Vim`] is not offered then ([`super::Keymap::ALL`],
//! [`super::Keymap::from_vim`]), and acts as `Normal` if it is asked for.
#![allow(
    dead_code,
    clippy::unused_self,
    clippy::unnecessary_wraps,
    clippy::missing_const_for_fn,
    clippy::needless_pass_by_ref_mut,
    reason = "the API of `vim/`, as it is, doing nothing"
)]

use super::{KeymapOutput, StatusLine, StatusMode, View};
use crate::search::Matcher;

pub(crate) mod which_key {
    //! What a which-key list waits on: nothing without Vim.

    use crate::EditorCommand;

    /// The keys a which-key list waits on.
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

        /// What the list shows: nothing.
        pub(crate) fn hints(self, host_commands: &[EditorCommand]) -> impl Iterator<Item = &'static WhichKey> {
            let _ = (self, host_commands);
            std::iter::empty()
        }
    }

    /// A key after a [`Prefix`], and what it does.
    #[derive(Debug)]
    pub(crate) struct WhichKey {
        pub(crate) text: &'static str,
    }

    impl WhichKey {
        pub(crate) const fn key(&self) -> &'static str {
            self.text
        }
    }

    /// The `g` keys Vim has of its own: none.
    pub fn own_g_keys() -> impl Iterator<Item = (&'static str, &'static str)> {
        std::iter::empty()
    }
}

/// The mode: Insert, where every key types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Mode {
    Normal,
    #[default]
    Insert,
}

/// User-configurable Vim behavior: kept for the hosts' settings, used by nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct VimOptions {
    pub insert_escapes: Vec<[char; 2]>,
    pub escape_timeout: f64,
    pub indent: crate::Indent,
    pub comment: Option<crate::Comment>,
}

impl Default for VimOptions {
    fn default() -> Self {
        Self { insert_escapes: Vec::new(), escape_timeout: 0.5, indent: crate::Indent::default(), comment: None }
    }
}

/// Per-editor Vim state: none but the options.
#[derive(Debug, Default)]
pub(crate) struct VimState {
    pub(crate) options: VimOptions,
}

impl VimState {
    pub(crate) const fn mode(&self) -> Mode {
        Mode::Insert
    }

    pub(crate) const fn mode_name(&self) -> &'static str {
        "insert"
    }

    pub(crate) fn which_key_prefix(&self) -> Option<which_key::Prefix> {
        None
    }

    pub(crate) fn status(&self) -> StatusLine {
        StatusLine { mode: StatusMode::Insert, text: String::new() }
    }

    pub(crate) fn block_cursor(&self, _current: Option<usize>, _text: &str) -> Option<usize> {
        None
    }

    pub(crate) fn search_highlight(&self) -> Option<Matcher> {
        None
    }

    pub(crate) fn block_ranges(&self, _text: &str) -> Option<Vec<std::ops::Range<usize>>> {
        None
    }

    pub(super) fn handle(&mut self, ui: &egui::Ui, id: egui::Id, text: &mut String, _: (View, bool)) -> KeymapOutput {
        let _ = (ui, id, text);
        KeymapOutput::default()
    }
}
