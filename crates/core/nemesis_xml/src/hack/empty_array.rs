//! Pushing to an array that is **empty** in the template.
//!
//! An empty array is written on one line, so its elements have nowhere to go in a line-based patch.
//! Mods write it in one of two ways. In both, `numelements="0"` says that the template has no
//! elements, so all of them are new: it is a push, which lets several mods add to the same array.
//!
//! The form where the line is replaced, which is valid XML.
//! ```xml
//! <!-- MOD_CODE ~id~ OPEN -->
//! <hkparam name="boneWeights" numelements="2">
//!     1.0 1.0
//! </hkparam>
//! <!-- ORIGINAL -->
//! <hkparam name="boneWeights" numelements="0"></hkparam>
//! <!-- CLOSE -->
//! ```
//!
//! The form where the line is kept and the elements follow it, which is Nemesis' usual syntax and the
//! one the patch editor writes (e.g., `Precision Creatures`). It is not valid XML, so it needs
//! [`crate::hack::HackOptions`]. Pandora v4 reports it as an error.
//! ```xml
//! <hkparam name="boneWeights" numelements="0"></hkparam>
//! <!-- MOD_CODE ~colisc~ OPEN -->
//!     1.0 1.0
//! </hkparam>
//! <!-- CLOSE -->
//! ```
//! What tells it apart is the `</hkparam>` nothing opened, right before `CLOSE`. A block after the
//! line of an empty array is usually something else, the next field, whose `</hkparam>` has its
//! `<hkparam>`:
//! ```xml
//! <hkparam name="listeners" numelements="0"></hkparam>
//! <!-- MOD_CODE ~amco~ OPEN -->
//! <hkparam name="enterNotifyEvents">#amco$17</hkparam>
//! <!-- ORIGINAL -->
//! <hkparam name="enterNotifyEvents">null</hkparam>
//! <!-- CLOSE -->
//! ```

use winnow::{ModalResult, Parser, combinator::peek, token::take_until};

use crate::helpers::{
    comment::{CommentKind, close_comment, comment_kind},
    tag::{any_field_start_tag, end_tag},
};

/// After the elements of the usual form: `</hkparam>`, `ORIGINAL`, the empty field, `CLOSE`.
pub(crate) fn original_is_empty_array(field: &str) -> impl FnMut(&mut &str) -> ModalResult<()> {
    move |input: &mut &str| {
        end_tag("hkparam").parse_next(input)?;
        close_comment.verify(|kind| matches!(kind, CommentKind::Original)).parse_next(input)?;
        any_field_start_tag.verify(|(name, len)| *name == field && *len == Some(0)).parse_next(input)?;
        end_tag("hkparam").parse_next(input)?;
        close_comment.verify(|kind| matches!(kind, CommentKind::Close)).void().parse_next(input)
    }
}

/// After the kept line of the second form: its `</hkparam>` and `MOD_CODE`, with the elements coming
/// next. Those are only looked at: up to a `</hkparam>` that nothing opened, then `CLOSE`.
pub(crate) fn closed_then_mod_code(input: &mut &str) -> ModalResult<()> {
    end_tag("hkparam").parse_next(input)?;
    comment_kind.verify(|kind| matches!(kind, CommentKind::ModCode(_))).void().parse_next(input)?;
    peek((
        take_until(0.., "</hkparam>")
            .verify(|elements: &str| !elements.contains("<hkparam") && !elements.contains("<!--")),
        end_tag("hkparam"),
        close_comment.verify(|kind| matches!(kind, CommentKind::Close)),
    ))
    .void()
    .parse_next(input)
}
