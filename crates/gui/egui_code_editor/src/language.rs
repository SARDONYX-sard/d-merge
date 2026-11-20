//! The contract between the editor and a language ("what does this text mean?").
//!
//! The editor knows nothing about any language. A language plugs in by implementing
//! [`Language`]; every method has a "no feature" default, so a plain-text language is just
//! `impl Language for PlainText {}`.
//!
//! # Coordinates
//! `column`s are 0-based **char** indices inside one line (same as egui's `CCursor`, not bytes).

use std::{borrow::Cow, ops::Range};

use egui::Color32;

/// How the language comments a line out, for `Ctrl+/` and Vim's `gc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comment {
    /// A mark to the end of the line. e.g. `//`, `#`
    Line(&'static str),
    /// Open and close marks around the line. e.g. `<!--`, `-->`
    Block(&'static str, &'static str),
}

/// One level of indentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indent {
    /// A tab character, as wide as this many columns. e.g., Havok XML
    Tab(usize),
    /// This many spaces.
    Spaces(usize),
}

impl Default for Indent {
    #[inline]
    fn default() -> Self {
        Self::Spaces(4)
    }
}

impl std::fmt::Display for Indent {
    /// e.g., `Tab: 4`, `Spaces: 4`
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tab(width) => write!(f, "Tab: {width}"),
            Self::Spaces(width) => write!(f, "Spaces: {width}"),
        }
    }
}

/// Language services used by [`super::CodeEditor`].
///
/// Methods receive **one line** (no `\n`) unless stated otherwise, because every feature the
/// editor offers is line-local. Document-wide analysis (diagnostics) is done by the caller, who
/// can cache it and pass it with [`super::CodeEditor::diagnostics`].
pub trait Language {
    /// Colored ranges of `line`. Uncovered chars use the default text color.
    ///
    /// e.g., `"0.5 animmotion"` -> `[(0..3, orange), (4..14, blue)]`
    fn highlight(&self, line: &str, visuals: &egui::Visuals) -> Vec<(Range<usize>, Color32)> {
        let _ = (line, visuals);
        Vec::new()
    }

    /// Like [`Self::highlight`], for languages whose lines mean what their position says (e.g. the
    /// n-th line of a record is its speed). `line_no` is 0-based.
    fn highlight_at(&self, line_no: usize, line: &str, visuals: &egui::Visuals) -> Vec<(Range<usize>, Color32)> {
        let _ = line_no;
        self.highlight(line, visuals)
    }

    /// Completion candidates at `column`.
    fn complete(&self, line: &str, column: usize) -> CompletionList {
        let _ = (line, column);
        CompletionList::default()
    }

    /// Typing one of these chars opens the completion popup (identifier chars always do).
    fn completion_triggers(&self) -> &[char] {
        &[]
    }

    /// Markdown explaining the token at `column`.
    fn hover(&self, line: &str, column: usize) -> Option<String> {
        let _ = (line, column);
        None
    }

    /// Like [`Self::hover`], with the **whole document** for languages whose docs depend on the
    /// surrounding lines (e.g., which array the hovered element is in). `line` is 0-based.
    fn hover_at(&self, text: &str, line: usize, column: usize) -> Option<String> {
        self.hover(text.split('\n').nth(line)?, column)
    }

    /// LSP `textDocument/definition`: where the symbol at `line` / `column` of the **whole document**
    /// `text` is defined. Used by `F12`, `Ctrl+click`, Vim `gd` and the right-click menu.
    fn definition(&self, text: &str, line: usize, column: usize) -> Option<Location<'_>> {
        let _ = (text, line, column);
        None
    }

    /// LSP `textDocument/references`: who uses the symbol at `line` / `column`.
    /// `None` = no symbol there; an empty list = a symbol nobody uses.
    /// Used by `Shift+F12`, Vim `gD` and the right-click menu.
    fn references(&self, text: &str, line: usize, column: usize) -> Option<References<'_>> {
        let _ = (text, line, column);
        None
    }

    /// LSP `textDocument/prepareRename`: the char columns of `line` holding the symbol at `column`,
    /// if it can be renamed (`F2`). e.g. `$eventID[moveStart]$` -> the columns of `moveStart`
    fn prepare_rename(&self, text: &str, line: usize, column: usize) -> Option<Range<usize>> {
        let _ = (text, line, column);
        None
    }

    /// LSP `textDocument/rename`: the edits, in every document, that give the symbol at `line` /
    /// `column` of the **whole document** `text` the name `new_name`.
    ///
    /// # Errors
    /// Why it can't (e.g. the name is taken), shown under the name being typed.
    fn rename(
        &self,
        text: &str,
        line: usize,
        column: usize,
        new_name: &str,
    ) -> Result<Vec<DocumentEdit<'_>>, Cow<'_, str>> {
        let _ = (text, line, column, new_name);
        Err(Cow::Borrowed("Nothing to rename here"))
    }

    /// The signature being typed at `column`.
    fn signature_help(&self, line: &str, column: usize) -> Option<SignatureHelp> {
        let _ = (line, column);
        None
    }

    /// Labels drawn at the end of `line` (e.g., parameter names).
    fn inlay_hints(&self, line: &str) -> Vec<String> {
        let _ = line;
        Vec::new()
    }

    /// Like [`Self::inlay_hints`], for languages whose lines mean what their position says.
    /// `line_no` is 0-based.
    fn inlay_hints_at(&self, line_no: usize, line: &str) -> Vec<Cow<'_, str>> {
        let _ = line_no;
        self.inlay_hints(line).into_iter().map(Cow::Owned).collect()
    }

    /// How a line is commented out (`Ctrl+/`, Vim `gcc`). `None`: it can't be.
    fn comment(&self) -> Option<Comment> {
        None
    }

    /// What one level of indentation is: what `Tab` and Vim's `>` insert, shown in the status bar.
    fn indent(&self) -> Indent {
        Indent::default()
    }

    /// Formats the **whole document**. `None` = this language has no formatter.
    fn format(&self, text: &str) -> Option<String> {
        let _ = text;
        None
    }

    /// Quick fixes for the problems at `line` / `column` of the **whole document** `text`
    /// (`Ctrl+.`, like VS Code).
    fn quick_fixes(&self, text: &str, line: usize, column: usize) -> Vec<QuickFix> {
        let _ = (text, line, column);
        Vec::new()
    }
}

/// A place in some document, for [`Language::definition`] / [`Language::references`].
///
/// The editor only has one text, so it hands locations to its host to open
/// (see [`crate::Navigation`]). Strings are usually borrowed from the language's own data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location<'a> {
    /// The document, in terms the host understands. e.g., an object id such as `#0106`.
    pub document: Cow<'a, str>,
    /// 0-based line.
    pub line: usize,
    /// One line for lists. e.g., `hkbClipGenerator`
    pub detail: Cow<'a, str>,
}

/// One edit of [`Language::rename`]: `columns` of `line` of `document` become `new_text`.
///
/// The editor has one text only: it hands them to its host, which applies them (see
/// [`Self::apply`]) to each document, the open one too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentEdit<'a> {
    /// The document, in terms the host understands. e.g., an object id such as `#0106`.
    pub document: Cow<'a, str>,
    /// 0-based line.
    pub line: usize,
    /// Char columns to replace.
    pub columns: Range<usize>,
    pub new_text: String,
}

impl DocumentEdit<'_> {
    /// `text` with `edits` (of its document) applied. Edits that overlap, or are off the text, are
    /// left out.
    ///
    /// e.g. `"a b\nc"`, edits `0:2..3 -> "x"` and `1:0..1 -> "y"` -> `"a x\ny"`
    #[must_use]
    pub fn apply<'e>(text: &str, edits: impl IntoIterator<Item = &'e Self>) -> String
    where
        Self: 'e,
    {
        // Byte ranges, last first: each leaves the bytes before it where they were.
        let line_starts: Vec<usize> =
            std::iter::once(0).chain(text.match_indices('\n').map(|(at, _)| at + 1)).collect();
        let byte_at = |line: usize, column: usize| {
            let start = *line_starts.get(line)?;
            let line_text = &text[start..text[start..].find('\n').map_or(text.len(), |end| start + end)];
            let offset = line_text.char_indices().map(|(at, _)| at).chain([line_text.len()]).nth(column)?;
            Some(start + offset)
        };
        let mut ranges: Vec<(Range<usize>, &str)> = edits
            .into_iter()
            .filter_map(|edit| {
                let range = byte_at(edit.line, edit.columns.start)?..byte_at(edit.line, edit.columns.end)?;
                Some((range, edit.new_text.as_str()))
            })
            .collect();
        ranges.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));

        let mut out = text.to_owned();
        let mut limit = text.len();
        for (range, new_text) in ranges {
            if range.end <= limit && range.start <= range.end {
                out.replace_range(range.clone(), new_text);
                limit = range.start;
            }
        }
        out
    }
}

/// Result of [`Language::references`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct References<'a> {
    /// What was asked about, for the title of a list. e.g., `#0106`, `$eventID[moveStart]$`
    pub symbol: Cow<'a, str>,
    pub locations: Vec<Location<'a>>,
}

/// One quick fix: replace `columns` of `line` with `new_text`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickFix {
    /// Menu title, e.g. `Fix IFrame key -> \"Duration\"`.
    pub title: String,
    /// 0-based line.
    pub line: usize,
    /// Char columns to replace.
    pub columns: Range<usize>,
    pub new_text: String,
}

/// How serious a [`Diagnostic`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    /// Theme color (underline, icon, message).
    #[inline]
    pub const fn color(self, visuals: &egui::Visuals) -> Color32 {
        match self {
            Self::Error => visuals.error_fg_color,
            Self::Warning => visuals.warn_fg_color,
        }
    }

    /// Short icon for lists.
    #[inline]
    pub const fn icon(self) -> &'static str {
        match self {
            Self::Error => "✖",
            Self::Warning => "⚠",
        }
    }
}

/// An underlined problem.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    /// 0-based line.
    pub line: usize,
    /// Columns inside the line. An empty range still underlines one char.
    pub columns: Range<usize>,
    pub severity: Severity,
    pub message: String,
}

/// Result of [`Language::complete`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompletionList {
    /// Columns replaced by the chosen item (usually the word being typed).
    pub replace: Range<usize>,
    pub items: Vec<CompletionItem>,
}

/// One completion candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    /// Shown in the list. A fixed text stays borrowed: a list is made at each key.
    pub label: Cow<'static, str>,
    /// Small glyph before the label (e.g., `ƒ` for functions).
    pub icon: &'static str,
    /// Plain text to insert (snippets must already be expanded by the language).
    pub insert: Cow<'static, str>,
    /// Char range inside `insert` to select after inserting (e.g., a placeholder).
    /// `None` puts the cursor after the inserted text.
    pub select: Option<Range<usize>>,
    /// Markdown shown next to the list.
    pub documentation: Cow<'static, str>,
}

/// Result of [`Language::signature_help`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    /// e.g., `animmotion <x: f32> <y: f32> <z: f32>`
    pub label: String,
    /// Parameter names; the active one is emphasized.
    pub params: Vec<String>,
    /// Index into `params`.
    pub active: usize,
    /// Extra Markdown.
    pub documentation: Option<String>,
}
