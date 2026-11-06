//! A changed block of a clip's or a motion's patch, as it is written. What its lines mean depends on
//! where it is, which is for the parser of each to say.

use nemesis_patch_syntax::{
    comment::{close_comment_line, take_raw_block},
    cursor::Cursor,
};
use winnow::combinator::opt;

use crate::adsf::patch::de::error::Error;

/// What a block does.
#[derive(Debug, Clone, Copy)]
pub(super) enum Op {
    Add,
    Replace,
    Remove,
    /// Array push.
    SeqPush,
}

/// The operation of a patch. A push has none, which is the same as adding.
pub(super) const fn to_json_patch_op(op: Op) -> json_patch::Op {
    match op {
        Op::Replace => json_patch::Op::Replace,
        Op::Remove => json_patch::Op::Remove,
        Op::Add | Op::SeqPush => json_patch::Op::Add,
    }
}

/// See [`next_block`].
pub(super) struct RawBlock<'a> {
    /// From `OPEN` to `ORIGINAL` or `CLOSE`, without the comments.
    pub(super) text: &'a str,
    pub(super) op: Op,
    /// How many lines the block has after `ORIGINAL`, if it has that.
    pub(super) original_len: Option<usize>,
}

/// The block at `cursor`, if one starts here.
///
/// `ignore_close`: the parser goes on through the lines after `ORIGINAL`, as they are the template's:
/// the `CLOSE` after them is passed here when it comes. `op_without_original`: what a block that has
/// lines and no `ORIGINAL` does.
pub(super) fn next_block<'a>(
    cursor: &mut Cursor<'a, Error>,
    ignore_close: &mut bool,
    op_without_original: Op,
) -> Result<Option<RawBlock<'a>>, Error> {
    if *ignore_close && cursor.parse_next(opt(close_comment_line))?.is_some() {
        *ignore_close = false;
    }
    let Some((text, original_len)) = cursor.parse_next(take_raw_block)? else {
        return Ok(None);
    };
    let has_original = original_len.is_some();
    let op = match (has_original, text.is_empty()) {
        (true, true) => Op::Remove,
        (true, false) => Op::Replace,
        (false, true) => Op::Add,
        (false, false) => op_without_original,
    };
    if has_original {
        *ignore_close = true;
    }
    Ok(Some(RawBlock { text, op, original_len }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_operation_comes_from_what_the_block_has() {
        let op_of = |patch: &'static str, without: Op| {
            let mut cursor = Cursor::<Error>::new(patch);
            let mut ignore_close = false;
            let block = next_block(&mut cursor, &mut ignore_close, without).unwrap().unwrap();
            (block.op, ignore_close)
        };
        let open = "<!-- MOD_CODE ~m~ OPEN -->\n";
        let replace: &'static str = format!("{open}a\n<!-- ORIGINAL -->\nb\n<!-- CLOSE -->\n").leak();
        assert!(matches!(op_of(replace, Op::Add), (Op::Replace, true)));
        let remove: &'static str = format!("{open}<!-- ORIGINAL -->\nb\n<!-- CLOSE -->\n").leak();
        assert!(matches!(op_of(remove, Op::Add), (Op::Remove, true)));
        let push: &'static str = format!("{open}a\n<!-- CLOSE -->\n").leak();
        assert!(matches!(op_of(push, Op::SeqPush), (Op::SeqPush, false)));
        let empty: &'static str = format!("{open}<!-- CLOSE -->\n").leak();
        assert!(matches!(op_of(empty, Op::SeqPush), (Op::Add, false)));
    }

    #[test]
    fn the_close_after_the_original_is_passed() {
        let mut cursor = Cursor::<Error>::new("<!-- CLOSE -->\nrest");
        let mut ignore_close = true;
        assert!(next_block(&mut cursor, &mut ignore_close, Op::Add).unwrap().is_none());
        assert!(!ignore_close);
        assert_eq!(cursor.input, "rest");
    }
}
