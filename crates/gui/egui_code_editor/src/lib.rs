//! A small VS Code–like code editor widget for egui, independent of any language.
//!
//! - syntax highlighting, diagnostics underlines, hover, completion popup, signature help,
//!   inlay hints and formatting, all provided by a [`Language`] implementation
//! - key binding styles ([`Keymap`]): plain editing or Vim (Emacs can be added in [`keymap`])
//!
//! # Usage (egui builder style)
//! ```ignore
//! // Kept by the caller, one per document:
//! let mut state = CodeEditorState::default();
//!
//! let output = CodeEditor::new("my_editor", &mut text)
//!     .language(&MyLanguage)          // optional; plain text without it
//!     .diagnostics(&diagnostics)      // optional; computed by the caller when text changes
//!     .features(EditorFeatures::default())
//!     .keymap(Keymap::Vim)
//!     .show(ui, &mut state);
//!
//! if output.changed { /* re-run diagnostics */ }
//! if output.command == Some(EditorCommand::Save) { /* `:w` in Vim */ }
//! if let Some(navigation) = output.navigation { /* open what `Language::definition` found */ }
//! if output.action == Some("save") { /* picked in the `Ctrl+Shift+P` palette (`.actions(..)`) */ }
//! ```
//!
//! # One frame
//! 1. the keymap (e.g., Vim) and the completion popup consume their keys **before** the `TextEdit`
//! 2. `TextEdit` is shown with a custom layouter (colors + diagnostic underlines)
//! 3. a picked completion / `Shift+Alt+F` formatting is applied to the text
//! 4. overlays are drawn (cursor line, block cursor, hover, signature help, inlay hints)

mod command;
mod comment;
mod completion;
mod cursor;
mod diff_view;
mod edit_ops;
mod editor;
mod find;
mod hover;
mod inline_diff;
pub mod keymap;
pub mod language;
pub mod languages;
mod layout;
mod nav_history;
mod palette;
pub mod problems;
mod quick_fix;
mod references;
mod request;
mod search;
mod search_view;
mod state;
mod status;
pub mod text_pos;
mod ui;

pub(crate) use self::cursor::scroll_request_id;
pub use self::{
    command::{EditorCommand, EditorFeatures},
    cursor::{editor_id, set_cursor},
    diff_view::{DiffView, DiffViewOutput},
    editor::{CodeEditor, CodeEditorOutput, Navigation},
    inline_diff::{CharChange, DiffHunk, InlineDiff},
    keymap::{Keymap, KeymapState, StatusLine, StatusMode, VimOptions},
    language::{
        Comment, CompletionItem, CompletionList, Diagnostic, Indent, Language, Location, QuickFix, References,
        Severity, SignatureHelp,
    },
    layout::line_job,
    nav_history::NavHistory,
    palette::{EditorAction, EditorLabels},
    problems::{ProblemGroup, ProblemsCursor, ProblemsKey, ProblemsPanel},
    references::ReferenceList,
    request::EditorRequest,
    search_view::{SearchHit, SearchLabels, SearchView},
    state::CodeEditorState,
    status::{StatusBar, StatusBarOutput, status_line},
};
