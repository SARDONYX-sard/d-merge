//! What opens over the editor, each part with its logic (`mod.rs`) and its drawing (`ui`) together:
//!
//! | popup          | opens on                                   |
//! | -------------- | ------------------------------------------ |
//! | `completion`   | typing, `Ctrl+Space`                       |
//! | `hover`        | the pointer resting on a char, Vim `K`     |
//! | `find`         | `Ctrl+F`, `Ctrl+G` (the normal keymap)     |
//! | `quick_fix`    | `Ctrl+.`                                   |
//! | `rename`       | `F2`                                       |
//! | `palette`      | `Ctrl+Shift+P`                             |
//! | `which_key`    | a key waiting for the next (Vim `g`, ...)  |
//! | `context_menu` | a right click                              |
//!
//! All in [`widget_frame`]; [`AnchoredPopup`] is one opened by a button of the host's.

pub(crate) mod completion;
pub(crate) mod context_menu;
pub(crate) mod find;
mod frame;
pub(crate) mod hover;
pub(crate) mod palette;
pub(crate) mod quick_fix;
pub(crate) mod rename;
pub(crate) mod which_key;

pub use self::frame::{AnchoredPopup, widget_frame};
