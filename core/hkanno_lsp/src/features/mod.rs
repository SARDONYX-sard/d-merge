//! Editor features. Each module is independent and only depends on [`crate::parser`].
//!
//! | Feature          | Input                 | Output                                  |
//! |------------------|-----------------------|-----------------------------------------|
//! | diagnostics      | whole document        | errors / warnings with ranges           |
//! | code_action      | one diagnostic        | quick-fix text edit                     |
//! | completion       | line + cursor         | candidates + the range they replace     |
//! | hover            | line + cursor         | Markdown                                |
//! | signature        | line + cursor         | the "function signature" being typed    |
//! | highlight        | line                  | colored token ranges                    |
//! | format           | whole document        | normalized text                         |
//! | inlay_hint       | line                  | parameter-name labels                   |

pub mod code_action;
pub mod completion;
pub mod diagnostics;
pub mod format;
pub mod highlight;
pub mod hover;
pub mod inlay_hint;
pub mod signature;

/// Converts a char column into a byte offset of `line` (clamped to the line end).
///
/// e.g., `byte_offset("␀a", 1) == 3`
pub(crate) fn byte_offset(line: &str, column: usize) -> usize {
    line.char_indices().nth(column).map_or(line.len(), |(i, _)| i)
}
