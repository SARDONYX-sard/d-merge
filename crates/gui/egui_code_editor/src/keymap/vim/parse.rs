//! Parses pending Normal/Visual-mode keys into a [`Command`] (see [`super::command`] for the grammar).
//!
//! Keys accumulate until they form a complete command ([`Parse::Done`]) or cannot
//! ([`Parse::Invalid`]). e.g., `d` -> `Incomplete`, `d2w` -> `Done`, `Q` -> `Invalid`

use super::{
    command::{Command, Motion, ObjectKind, Operator, ScrollTo, Simple, Target},
    keys::VimKey,
};

/// Result of parsing the pending keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Parse {
    /// A full command. `count` is at least 1; `explicit_count` tells whether one was typed
    /// (`G` vs `5G`). `register`: the `"x` prefix.
    Done { command: Command, count: usize, explicit_count: bool, register: Option<char> },
    /// Wait for more keys (e.g., after `d`, `g` or `f`).
    Incomplete,
    /// Not a command; drop the keys.
    Invalid,
}

pub(super) fn parse(keys: &[VimKey]) -> Parse {
    // `"a` register prefix.
    let (register, keys) = match keys {
        [VimKey::Char('"')] => return Parse::Incomplete,
        [VimKey::Char('"'), VimKey::Char(r), rest @ ..] => (Some(*r), rest),
        _ => (None, keys),
    };
    let (count1, after_count) = take_count(keys);
    if after_count.is_empty() {
        return Parse::Incomplete;
    }
    let done = |command, count2: Option<usize>| {
        let explicit_count = count1.is_some() || count2.is_some();
        let count = count1.unwrap_or(1) * count2.unwrap_or(1);
        Parse::Done { command, count, explicit_count, register }
    };

    // ── operator + target ────────────────────────────────────────────────────
    match operator(after_count) {
        OpParse::Op(op, len) => {
            let op_keys = &after_count[..len];
            let (count2, rest) = take_count(&after_count[len..]);
            return match target(op_keys, rest) {
                Step::Done(target) => done(Command::Operate(op, target), count2),
                Step::NeedMore => Parse::Incomplete,
                Step::Invalid => Parse::Invalid,
            };
        }
        OpParse::None => {}
    }

    // ── motion ───────────────────────────────────────────────────────────────
    match motion(after_count) {
        Step::Done(m) => return done(Command::Move(m), None),
        Step::NeedMore => return Parse::Incomplete,
        Step::Invalid => {}
    }

    // ── simple ───────────────────────────────────────────────────────────────
    match simple(after_count) {
        Step::Done(command) => done(command, None),
        Step::NeedMore => Parse::Incomplete,
        Step::Invalid => Parse::Invalid,
    }
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

/// One parsing step: a result, "wait", or "not this".
enum Step<T> {
    Done(T),
    NeedMore,
    Invalid,
}

enum OpParse {
    /// The operator and how many keys it took (`d`: 1, `gu`: 2).
    Op(Operator, usize),
    None,
}

/// `g` alone is not an operator yet: the motion parser waits for the next key (`gu`, `gg`, ...).
const fn operator(keys: &[VimKey]) -> OpParse {
    let op = match keys {
        [VimKey::Char('d'), ..] => Operator::Delete,
        [VimKey::Char('c'), ..] => Operator::Change,
        [VimKey::Char('y'), ..] => Operator::Yank,
        [VimKey::Char('>'), ..] => Operator::Indent,
        [VimKey::Char('<'), ..] => Operator::Outdent,
        [VimKey::Char('g'), VimKey::Char('u'), ..] => return OpParse::Op(Operator::Lower, 2),
        [VimKey::Char('g'), VimKey::Char('U'), ..] => return OpParse::Op(Operator::Upper, 2),
        [VimKey::Char('g'), VimKey::Char('~'), ..] => return OpParse::Op(Operator::ToggleCase, 2),
        _ => return OpParse::None,
    };
    OpParse::Op(op, 1)
}

/// What follows an operator (`op_keys`, e.g. `[g, u]`).
fn target(op_keys: &[VimKey], rest: &[VimKey]) -> Step<Target> {
    let Some(&last) = op_keys.last() else { return Step::Invalid };
    match rest {
        [] => return Step::NeedMore,
        // Doubled: `dd`, `>>`, `guu`, `gugu`
        [key] if *key == last => return Step::Done(Target::Line),
        _ if rest == op_keys => return Step::Done(Target::Line),
        [VimKey::Char('i' | 'a')] => return Step::NeedMore,
        [VimKey::Char(c @ ('i' | 'a')), VimKey::Char(k), ..] => {
            return match object_kind(*k) {
                Some(kind) => Step::Done(Target::Object { kind, around: *c == 'a' }),
                None => Step::Invalid,
            };
        }
        _ => {}
    }
    match motion(rest) {
        Step::Done(m) => Step::Done(Target::Motion(m)),
        Step::NeedMore => Step::NeedMore,
        Step::Invalid => Step::Invalid,
    }
}

const fn object_kind(key: char) -> Option<ObjectKind> {
    Some(match key {
        'w' => ObjectKind::Word,
        'W' => ObjectKind::BigWord,
        'p' => ObjectKind::Paragraph,
        '"' | '\'' | '`' => ObjectKind::Quote(key),
        '(' | ')' | 'b' => ObjectKind::Pair('(', ')'),
        '[' | ']' => ObjectKind::Pair('[', ']'),
        '{' | '}' | 'B' => ObjectKind::Pair('{', '}'),
        '<' | '>' => ObjectKind::Pair('<', '>'),
        _ => return None,
    })
}

const fn motion(keys: &[VimKey]) -> Step<Motion> {
    use VimKey::{Char, Ctrl};
    let motion = match keys {
        [Char('h') | VimKey::Left | VimKey::Backspace, ..] => Motion::Left,
        [Char('l' | ' ') | VimKey::Right, ..] => Motion::Right,
        // No wrapping, so display lines are lines: `gj` = `j`, `g0` = `0`, ...
        [Char('j') | VimKey::Down | Ctrl('n'), ..] | [Char('g'), Char('j') | VimKey::Down, ..] => {
            Motion::Down
        }
        [Char('k') | VimKey::Up | Ctrl('p'), ..] | [Char('g'), Char('k') | VimKey::Up, ..] => {
            Motion::Up
        }
        [Char('w'), ..] => Motion::WordForward { big: false },
        [Char('W'), ..] => Motion::WordForward { big: true },
        [Char('b'), ..] => Motion::WordBackward { big: false },
        [Char('B'), ..] => Motion::WordBackward { big: true },
        [Char('e'), ..] => Motion::WordEnd { big: false },
        [Char('E'), ..] => Motion::WordEnd { big: true },
        [Char('0') | VimKey::Home, ..] | [Char('g'), Char('0'), ..] => Motion::LineStart,
        [Char('^'), ..] | [Char('g'), Char('^'), ..] => Motion::FirstNonBlank,
        [Char('$') | VimKey::End, ..] | [Char('g'), Char('$'), ..] => Motion::LineEnd,
        [Char('|'), ..] => Motion::Column,
        [Char('+') | VimKey::Enter, ..] => Motion::NextLineStart,
        [Char('-'), ..] => Motion::PrevLineStart,
        [Char('_'), ..] => Motion::CurrentLineStart,
        [Char('G'), ..] => Motion::FileEnd,
        [Char('}'), ..] => Motion::ParagraphForward,
        [Char('{'), ..] => Motion::ParagraphBackward,
        [Char('%'), ..] => Motion::MatchPair,
        [Char(';'), ..] => Motion::RepeatFind { reverse: false },
        [Char(','), ..] => Motion::RepeatFind { reverse: true },
        [Char('H'), ..] => Motion::ScreenTop,
        [Char('M'), ..] => Motion::ScreenMiddle,
        [Char('L'), ..] => Motion::ScreenBottom,
        [Char('n'), ..] => Motion::SearchNext { reverse: false },
        [Char('N'), ..] => Motion::SearchNext { reverse: true },
        [Ctrl('d'), ..] => Motion::HalfPage { down: true },
        [Ctrl('u'), ..] => Motion::HalfPage { down: false },
        [Ctrl('f') | VimKey::PageDown, ..] => Motion::Page { down: true },
        [Ctrl('b') | VimKey::PageUp, ..] => Motion::Page { down: false },

        // Two keys: `f<c>`, `'<c>`, `gg`, ...
        [Char('f' | 't' | 'F' | 'T' | '\'' | '`' | 'g')] => return Step::NeedMore,
        [Char(k @ ('f' | 't' | 'F' | 'T')), Char(ch), ..] => {
            Motion::Find { forward: matches!(k, 'f' | 't'), till: matches!(k, 't' | 'T'), ch: *ch }
        }
        [Char(k @ ('\'' | '`')), Char(name), ..] => {
            Motion::Mark { name: *name, linewise: *k == '\'' }
        }
        [Char('g'), Char('g'), ..] => Motion::FileStart,
        [Char('g'), Char('e'), ..] => Motion::WordEndBackward { big: false },
        [Char('g'), Char('E'), ..] => Motion::WordEndBackward { big: true },
        _ => return Step::Invalid,
    };
    Step::Done(motion)
}

const fn simple(keys: &[VimKey]) -> Step<Command> {
    use VimKey::{Char, Ctrl};
    let simple = match keys {
        // Shorthands for operator commands, like Vim's own definitions.
        [Char('s'), ..] => {
            return Step::Done(Command::Operate(Operator::Change, Target::Motion(Motion::Right)));
        }
        [Char('S'), ..] => return Step::Done(Command::Operate(Operator::Change, Target::Line)),

        [Char('x') | VimKey::Delete, ..] => Simple::DeleteChar,
        [Char('X'), ..] => Simple::DeleteCharBefore,
        [Char('D'), ..] => Simple::DeleteToEnd,
        [Char('C'), ..] => Simple::ChangeToEnd,
        [Char('Y'), ..] => Simple::YankLine,
        [Char('p'), ..] => Simple::PasteAfter,
        [Char('P'), ..] => Simple::PasteBefore,
        [Char('i'), ..] => Simple::Insert,
        [Char('a'), ..] => Simple::Append,
        [Char('I'), ..] => Simple::InsertLineStart,
        [Char('A'), ..] => Simple::AppendLineEnd,
        [Char('o'), ..] => Simple::OpenBelow,
        [Char('O'), ..] => Simple::OpenAbove,
        [Char('J'), ..] => Simple::JoinLines { spaces: true },
        [Char('~'), ..] => Simple::ToggleCase,
        [Char('u'), ..] => Simple::Undo,
        [Ctrl('r'), ..] => Simple::Redo,
        [Char('.'), ..] => Simple::Repeat,
        [Char('v'), ..] => Simple::Visual,
        [Ctrl('v'), ..] => Simple::VisualBlock,
        [Char('V'), ..] => Simple::VisualLine,
        [Char(':'), ..] => Simple::CommandLine,
        [Char('/'), ..] => Simple::SearchForward,
        [Char('?'), ..] => Simple::SearchBackward,
        [Char('*'), ..] => Simple::SearchWord { forward: true, whole: true },
        [Char('#'), ..] => Simple::SearchWord { forward: false, whole: true },
        [Char('K'), ..] => Simple::Hover,
        [Ctrl('a'), ..] => Simple::Increment { up: true },
        [Ctrl('x'), ..] => Simple::Increment { up: false },
        [Ctrl('o'), ..] => Simple::Jump { older: true },
        [Ctrl('i') | VimKey::Tab, ..] => Simple::Jump { older: false },
        [Ctrl('e'), ..] => Simple::ScrollLines { down: true },
        [Ctrl('y'), ..] => Simple::ScrollLines { down: false },
        [Ctrl('w'), ..] => Simple::Nop,

        // Two keys
        [Char('r' | 'm' | 'q' | '@' | 'g' | 'z')] => return Step::NeedMore,
        [Char('r'), Char(c), ..] => Simple::Replace(*c),
        [Char('m'), Char(c), ..] => Simple::SetMark(*c),
        [Char('q'), Char(c), ..] => Simple::Record(*c),
        [Char('@'), Char(c), ..] => Simple::Play(*c),
        [Char('g'), Char('i'), ..] => Simple::InsertAtLastInsert,
        [Char('g'), Char('I'), ..] => Simple::InsertAtColumnZero,
        [Char('g'), Char('J'), ..] => Simple::JoinLines { spaces: false },
        [Char('g'), Char('v'), ..] => Simple::Reselect,
        [Char('g'), Char('x'), ..] => Simple::OpenUrl,
        [Char('g'), Char('d'), ..] => Simple::GoTo { references: false },
        [Char('g'), Char('D'), ..] => Simple::GoTo { references: true },
        [Char('g'), Char('*'), ..] => Simple::SearchWord { forward: true, whole: false },
        [Char('g'), Char('#'), ..] => Simple::SearchWord { forward: false, whole: false },
        [Char('z'), key, ..] => {
            let (to, first_non_blank) = match key {
                Char('z') => (ScrollTo::Center, false),
                Char('.') => (ScrollTo::Center, true),
                Char('t') => (ScrollTo::Top, false),
                VimKey::Enter => (ScrollTo::Top, true),
                Char('b') => (ScrollTo::Bottom, false),
                Char('-') => (ScrollTo::Bottom, true),
                _ => return Step::Invalid,
            };
            Simple::ScrollCursor { to, first_non_blank }
        }
        _ => return Step::Invalid,
    };
    Step::Done(Command::Simple(simple))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(s: &str) -> Vec<VimKey> {
        s.chars().map(VimKey::Char).collect()
    }

    fn command(s: &str) -> Command {
        match parse(&keys(s)) {
            Parse::Done { command, .. } => command,
            other => panic!("{s}: {other:?}"),
        }
    }

    #[test]
    fn counts_and_operators() {
        assert_eq!(
            parse(&keys("3j")),
            Parse::Done {
                command: Command::Move(Motion::Down),
                count: 3,
                explicit_count: true,
                register: None
            }
        );
        assert!(matches!(
            parse(&keys("2dd")),
            Parse::Done { command: Command::Operate(Operator::Delete, Target::Line), count: 2, .. }
        ));
        assert_eq!(
            command("ciw"),
            Command::Operate(
                Operator::Change,
                Target::Object { kind: ObjectKind::Word, around: false }
            )
        );
        assert_eq!(command("gUU"), Command::Operate(Operator::Upper, Target::Line));
        assert_eq!(command("gugu"), Command::Operate(Operator::Lower, Target::Line));
        assert_eq!(command(">>"), Command::Operate(Operator::Indent, Target::Line));
        assert_eq!(
            command("da("),
            Command::Operate(
                Operator::Delete,
                Target::Object { kind: ObjectKind::Pair('(', ')'), around: true }
            )
        );
        assert_eq!(
            command("dt,"),
            Command::Operate(
                Operator::Delete,
                Target::Motion(Motion::Find { forward: true, till: true, ch: ',' })
            )
        );
    }

    #[test]
    fn registers_and_two_key_commands() {
        assert!(matches!(
            parse(&keys("\"a2yy")),
            Parse::Done { register: Some('a'), count: 2, .. }
        ));
        assert_eq!(command("qa"), Command::Simple(Simple::Record('a')));
        assert_eq!(command("@@"), Command::Simple(Simple::Play('@')));
        assert_eq!(command("gx"), Command::Simple(Simple::OpenUrl));
        assert_eq!(command("gg"), Command::Move(Motion::FileStart));
        assert_eq!(
            command("zt"),
            Command::Simple(Simple::ScrollCursor { to: ScrollTo::Top, first_non_blank: false })
        );
    }

    #[test]
    fn incomplete_and_invalid() {
        for pending in ["d", "g", "r", "f", "\"", "\"a", "di", "gu", "z", "'"] {
            assert_eq!(parse(&keys(pending)), Parse::Incomplete, "{pending}");
        }
        assert_eq!(parse(&keys("Q")), Parse::Invalid);
        assert_eq!(parse(&keys("gq")), Parse::Invalid);
        assert_eq!(parse(&keys("diz")), Parse::Invalid);
        assert_eq!(command("0"), Command::Move(Motion::LineStart)); // `0` is a motion, not a count
    }
}
