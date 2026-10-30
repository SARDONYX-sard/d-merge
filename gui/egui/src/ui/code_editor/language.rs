//! The contract between the editor and a language ("what does this text mean?").
//!
//! The editor knows nothing about any language. A language plugs in by implementing
//! [`Language`]; every method has a "no feature" default, so a plain-text language is just
//! `impl Language for PlainText {}`.
//!
//! # Coordinates
//! `column`s are 0-based **char** indices inside one line (same as egui's `CCursor`, not bytes).

use std::ops::Range;

use egui::Color32;

/// Language services used by [`super::CodeEditor`].
///
/// Methods receive **one line** (no `\n`) unless stated otherwise, because every feature the
/// editor offers is line-local. Document-wide analysis (diagnostics) is done by the caller, who
/// can cache it and pass it with [`super::CodeEditor::diagnostics`].
pub(crate) trait Language {
    /// Colored ranges of `line`. Uncovered chars use the default text color.
    ///
    /// e.g., `"0.5 animmotion"` -> `[(0..3, orange), (4..14, blue)]`
    fn highlight(&self, line: &str, visuals: &egui::Visuals) -> Vec<(Range<usize>, Color32)> {
        let _ = (line, visuals);
        Vec::new()
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

    /// Formats the **whole document**. `None` = this language has no formatter.
    fn format(&self, text: &str) -> Option<String> {
        let _ = text;
        None
    }
}

/// How serious a [`Diagnostic`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Severity {
    Error,
    Warning,
}

impl Severity {
    /// Theme color (underline, icon, message).
    pub(crate) const fn color(self, visuals: &egui::Visuals) -> Color32 {
        match self {
            Self::Error => visuals.error_fg_color,
            Self::Warning => visuals.warn_fg_color,
        }
    }

    /// Short icon for lists.
    pub(crate) const fn icon(self) -> &'static str {
        match self {
            Self::Error => "✖",
            Self::Warning => "⚠",
        }
    }
}

/// An underlined problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Diagnostic {
    /// 0-based line.
    pub(crate) line: usize,
    /// Columns inside the line. An empty range still underlines one char.
    pub(crate) columns: Range<usize>,
    pub(crate) severity: Severity,
    pub(crate) message: String,
}

/// Result of [`Language::complete`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CompletionList {
    /// Columns replaced by the chosen item (usually the word being typed).
    pub(crate) replace: Range<usize>,
    pub(crate) items: Vec<CompletionItem>,
}

/// One completion candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletionItem {
    /// Shown in the list.
    pub(crate) label: String,
    /// Small glyph before the label (e.g., `ƒ` for functions).
    pub(crate) icon: &'static str,
    /// Plain text to insert (snippets must already be expanded by the language).
    pub(crate) insert: String,
    /// Char range inside `insert` to select after inserting (e.g., a placeholder).
    /// `None` puts the cursor after the inserted text.
    pub(crate) select: Option<Range<usize>>,
    /// Markdown shown next to the list.
    pub(crate) documentation: String,
}

/// Result of [`Language::signature_help`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignatureHelp {
    /// e.g., `animmotion <x: f32> <y: f32> <z: f32>`
    pub(crate) label: String,
    /// Parameter names; the active one is emphasized.
    pub(crate) params: Vec<String>,
    /// Index into `params`.
    pub(crate) active: usize,
    /// Extra Markdown.
    pub(crate) documentation: Option<String>,
}
