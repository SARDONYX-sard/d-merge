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
mod editor;
mod inline_diff;
pub mod keymap;
pub mod language;
pub mod languages;
mod popups;
mod request;
mod search;
pub mod text_pos;
mod views;

pub(crate) use self::editor::cursor::scroll_request_id;
pub use self::{
    command::{EditorCommand, EditorFeatures},
    editor::{
        CodeEditor, CodeEditorOutput, Navigation,
        cursor::{editor_id, place_cursor, set_cursor},
        layout::{line_job, line_job_at},
        state::CodeEditorState,
    },
    inline_diff::{CharChange, DiffHunk, InlineDiff},
    keymap::{Keymap, KeymapState, StatusLine, StatusMode, VimOptions},
    language::{
        Comment, CompletionItem, CompletionList, Diagnostic, Indent, Language, Location, QuickFix, References,
        Severity, SignatureHelp,
    },
    popups::{
        AnchoredPopup,
        palette::{EditorAction, EditorLabels},
        widget_frame,
    },
    request::EditorRequest,
    views::{
        breadcrumbs::{Breadcrumbs, Crumb},
        diff_view::{DiffView, DiffViewOutput},
        nav_history::NavHistory,
        problems,
        problems::{ProblemGroup, ProblemsCursor, ProblemsKey, ProblemsPanel},
        references::{PeekAsk, PeekInput, ReferenceList},
        search_view::{SearchHit, SearchLabels, SearchView},
        status::{StatusBar, StatusBarOutput, status_line},
    },
};
