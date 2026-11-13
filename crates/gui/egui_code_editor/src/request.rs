//! The editor's own commands asked for by the host, in place of the editor's keys: for a host whose
//! keys are the user's to change (see [`crate::CodeEditor::requests`]).
//!
//! | request          | the editor's key       | what it does                                  |
//! | ---------------- | ---------------------- | --------------------------------------------- |
//! | `Completion`     | `Ctrl+Space`           | opens the completion list                     |
//! | `Format`         | `Shift+Alt+F`          | formats the document                          |
//! | `QuickFix`       | `Ctrl+.`               | the quick fixes at the cursor                 |
//! | `CommandPalette` | `Ctrl+Shift+P`         | the palette                                   |
//! | `GoToDefinition` | `F12`                  |                                               |
//! | `FindReferences` | `Shift+F12`            |                                               |
//! | `Find`           | `Ctrl+F` (Normal)      | the find bar                                  |
//! | `GoToLine`       | `Ctrl+G` (Normal)      |                                               |
//! | `DeleteLines` ...| `Ctrl+Shift+K` ...     | the line commands of the Normal keymap        |
//! | `ToggleComment`  | `Ctrl+/`               | comments the lines out / in                   |
//!
//! Given requests (even none), the editor reads none of those keys itself.

/// See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorRequest {
    Completion,
    Format,
    QuickFix,
    CommandPalette,
    GoToDefinition,
    FindReferences,
    Find,
    GoToLine,
    DeleteLines,
    MoveLinesUp,
    MoveLinesDown,
    CopyLinesUp,
    CopyLinesDown,
    InsertLineBelow,
    InsertLineAbove,
    ToggleComment,
}

/// Was `request` asked for? `None` requests: the editor reads its own keys.
#[inline]
pub(crate) fn asked(requests: Option<&[EditorRequest]>, request: EditorRequest) -> bool {
    requests.is_some_and(|requests| requests.contains(&request))
}
