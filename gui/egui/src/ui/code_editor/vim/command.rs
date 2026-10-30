//! Parses pending Normal/Visual-mode keys into a [`Command`].
//!
//! # Grammar (the supported subset of Vim)
//! ```text
//! command  := [count] ( motion | operator [count] target | simple )
//! target   := motion | operator (same char: `dd`, `yy`, `cc`) | ("i" | "a") "w"
//! motion   := h j k l w b e 0 ^ $ G gg  (+ arrow keys)
//! operator := d c y
//! simple   := x X D C Y p P i a I A o O J ~ r<char> u <C-r> v V :
//! ```
//! e.g., `3j`, `dw`, `2dd`, `ciw`, `y$`, `r1`
//!
//! Keys accumulate until they form a complete command ([`Parse::Done`]) or cannot ([`Parse::Invalid`]).

use super::keys::VimKey;

/// Where a motion moves the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Motion {
    Left,
    Right,
    Down,
    Up,
    /// `w`
    WordForward,
    /// `b`
    WordBackward,
    /// `e`
    WordEnd,
    /// `0`
    LineStart,
    /// `^`
    FirstNonBlank,
    /// `$`
    LineEnd,
    /// `gg` (with a count: that line)
    FileStart,
    /// `G` (with a count: that line)
    FileEnd,
}

/// `d`, `c`, `y`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Operator {
    Delete,
    Change,
    Yank,
}

/// What an operator acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    Motion(Motion),
    /// `dd`, `cc`, `yy`
    Line,
    /// `iw` (`around: false`) / `aw` (`around: true`)
    Word {
        around: bool,
    },
}

/// Commands that are not "operator + target".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Simple {
    /// `x`
    DeleteChar,
    /// `X`
    DeleteCharBefore,
    /// `D`
    DeleteToEnd,
    /// `C`
    ChangeToEnd,
    /// `Y`
    YankLine,
    /// `p`
    PasteAfter,
    /// `P`
    PasteBefore,
    /// `i`
    Insert,
    /// `a`
    Append,
    /// `I`
    InsertLineStart,
    /// `A`
    AppendLineEnd,
    /// `o`
    OpenBelow,
    /// `O`
    OpenAbove,
    /// `J`
    JoinLines,
    /// `~`
    ToggleCase,
    /// `r<char>`
    Replace(char),
    /// `u`
    Undo,
    /// `Ctrl+R`
    Redo,
    /// `v`
    Visual,
    /// `V`
    VisualLine,
    /// `:`
    CommandLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Command {
    Move(Motion),
    Operate(Operator, Target),
    Simple(Simple),
}

/// Result of parsing the pending keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Parse {
    /// A full command. `count` is at least 1; `explicit_count` tells whether one was typed
    /// (`G` vs `5G`).
    Done { command: Command, count: usize, explicit_count: bool },
    /// Wait for more keys (e.g., after `d` or `g`).
    Incomplete,
    /// Not a command; drop the keys.
    Invalid,
}

pub(super) fn parse(keys: &[VimKey]) -> Parse {
    let (count1, after_count) = take_count(keys);

    let Some((&first, rest)) = after_count.split_first() else { return Parse::Incomplete };
    let done = |command, count2: Option<usize>| {
        let explicit = count1.is_some() || count2.is_some();
        let count = count1.unwrap_or(1) * count2.unwrap_or(1);
        Parse::Done { command, count, explicit_count: explicit }
    };

    // ── operator + target ────────────────────────────────────────────────────
    if let Some(op) = operator(first) {
        let (count2, rest) = take_count(rest);
        return match rest {
            [] | [VimKey::Char('i' | 'a')] => Parse::Incomplete,
            [key, ..] if operator(*key) == Some(op) => {
                done(Command::Operate(op, Target::Line), count2)
            }
            [VimKey::Char(c @ ('i' | 'a')), VimKey::Char('w')] => {
                done(Command::Operate(op, Target::Word { around: *c == 'a' }), count2)
            }
            _ => match motion(rest) {
                MotionParse::Motion(m) => done(Command::Operate(op, Target::Motion(m)), count2),
                MotionParse::NeedMore => Parse::Incomplete,
                MotionParse::NotMotion => Parse::Invalid,
            },
        };
    }

    // ── motion ───────────────────────────────────────────────────────────────
    match motion(after_count) {
        MotionParse::Motion(m) => return done(Command::Move(m), None),
        MotionParse::NeedMore => return Parse::Incomplete,
        MotionParse::NotMotion => {}
    }

    // ── simple ───────────────────────────────────────────────────────────────
    let simple = match first {
        VimKey::Redo => Simple::Redo,
        VimKey::Char('r') => match rest {
            [] => return Parse::Incomplete,
            [VimKey::Char(c), ..] => Simple::Replace(*c),
            _ => return Parse::Invalid,
        },
        VimKey::Char(c) => match c {
            'x' => Simple::DeleteChar,
            'X' => Simple::DeleteCharBefore,
            'D' => Simple::DeleteToEnd,
            'C' => Simple::ChangeToEnd,
            'Y' => Simple::YankLine,
            'p' => Simple::PasteAfter,
            'P' => Simple::PasteBefore,
            'i' => Simple::Insert,
            'a' => Simple::Append,
            'I' => Simple::InsertLineStart,
            'A' => Simple::AppendLineEnd,
            'o' => Simple::OpenBelow,
            'O' => Simple::OpenAbove,
            'J' => Simple::JoinLines,
            '~' => Simple::ToggleCase,
            'u' => Simple::Undo,
            'v' => Simple::Visual,
            'V' => Simple::VisualLine,
            ':' => Simple::CommandLine,
            _ => return Parse::Invalid,
        },
        _ => return Parse::Invalid,
    };
    done(Command::Simple(simple), None)
}

/// Leading digits (`0` alone is the `0` motion, not a count).
fn take_count(keys: &[VimKey]) -> (Option<usize>, &[VimKey]) {
    let digits = keys
        .iter()
        .enumerate()
        .take_while(
            |(i, k)| matches!(k, VimKey::Char(c) if c.is_ascii_digit() && !(*i == 0 && *c == '0')),
        )
        .count();
    if digits == 0 {
        return (None, keys);
    }
    let count = keys[..digits]
        .iter()
        .filter_map(|k| match k {
            VimKey::Char(c) => c.to_digit(10),
            _ => None,
        })
        .fold(0_usize, |acc, d| acc.saturating_mul(10).saturating_add(d as usize));
    (Some(count.max(1)), &keys[digits..])
}

const fn operator(key: VimKey) -> Option<Operator> {
    match key {
        VimKey::Char('d') => Some(Operator::Delete),
        VimKey::Char('c') => Some(Operator::Change),
        VimKey::Char('y') => Some(Operator::Yank),
        _ => None,
    }
}

/// Result of trying to read a motion.
enum MotionParse {
    Motion(Motion),
    /// A prefix such as `g` (of `gg`).
    NeedMore,
    NotMotion,
}

const fn motion(keys: &[VimKey]) -> MotionParse {
    let motion = match keys {
        [VimKey::Char('h') | VimKey::Left | VimKey::Backspace, ..] => Motion::Left,
        [VimKey::Char('l') | VimKey::Right, ..] => Motion::Right,
        [VimKey::Char('j') | VimKey::Down | VimKey::Enter, ..] => Motion::Down,
        [VimKey::Char('k') | VimKey::Up, ..] => Motion::Up,
        [VimKey::Char('w'), ..] => Motion::WordForward,
        [VimKey::Char('b'), ..] => Motion::WordBackward,
        [VimKey::Char('e'), ..] => Motion::WordEnd,
        [VimKey::Char('0'), ..] => Motion::LineStart,
        [VimKey::Char('^'), ..] => Motion::FirstNonBlank,
        [VimKey::Char('$'), ..] => Motion::LineEnd,
        [VimKey::Char('G'), ..] => Motion::FileEnd,
        [VimKey::Char('g')] => return MotionParse::NeedMore,
        [VimKey::Char('g'), VimKey::Char('g'), ..] => Motion::FileStart,
        _ => return MotionParse::NotMotion,
    };
    MotionParse::Motion(motion)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(s: &str) -> Vec<VimKey> {
        s.chars().map(VimKey::Char).collect()
    }

    #[test]
    fn counts_and_operators() {
        assert_eq!(
            parse(&keys("3j")),
            Parse::Done { command: Command::Move(Motion::Down), count: 3, explicit_count: true }
        );
        assert_eq!(
            parse(&keys("2dd")),
            Parse::Done {
                command: Command::Operate(Operator::Delete, Target::Line),
                count: 2,
                explicit_count: true
            }
        );
        assert_eq!(
            parse(&keys("ciw")),
            Parse::Done {
                command: Command::Operate(Operator::Change, Target::Word { around: false }),
                count: 1,
                explicit_count: false
            }
        );
    }

    #[test]
    fn incomplete_and_invalid() {
        assert_eq!(parse(&keys("d")), Parse::Incomplete);
        assert_eq!(parse(&keys("g")), Parse::Incomplete);
        assert_eq!(parse(&keys("r")), Parse::Incomplete);
        assert_eq!(parse(&keys("Q")), Parse::Invalid);
        assert_eq!(parse(&keys("0")), parse(&keys("0"))); // `0` is a motion, not a count
        assert!(matches!(
            parse(&keys("0")),
            Parse::Done { command: Command::Move(Motion::LineStart), .. }
        ));
    }
}
