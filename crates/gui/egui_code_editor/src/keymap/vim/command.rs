//! The commands of Normal / Visual mode (what [`super::parse`] turns keys into).
//!
//! # Grammar (the supported subset of Vim)
//! ```text
//! command  := ["<reg>] [count] ( motion | operator [count] target | simple )
//! target   := motion | operator again (`dd`, `>>`, `gUU`) | ("i" | "a") object
//! motion   := h j k l w W b B e E ge gE 0 ^ $ | + - _ <CR> gg G { } % f t F T ; , ' `
//!             H M L n N <C-d> <C-u> <C-f> <C-b> (+ arrows, Home, End, PageUp/Down)
//! operator := d c y > < gu gU g~
//! object   := w W p " ' ` ( ) b [ ] { } B < >
//! simple   := x X D C Y s S p P i a I A gi gI o O J gJ ~ r<c> u <C-r> . v V gv : / ? * # g* g#
//!             K gx <C-a> <C-x> m<c> q<c> q @<c> @@ <C-o> <C-i> zz zt zb z. z<CR> z- <C-e> <C-y>
//! ```
//! e.g., `3j`, `dw`, `2dd`, `ci(`, `"ayy`, `>ip`, `gUiw`, `dt,`, `qa ... q`, `@a`

/// Where a motion moves the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Motion {
    Left,
    Right,
    Down,
    Up,
    /// `w` / `W` (`big`: words are separated by blanks only)
    WordForward {
        big: bool,
    },
    /// `b` / `B`
    WordBackward {
        big: bool,
    },
    /// `e` / `E`
    WordEnd {
        big: bool,
    },
    /// `ge` / `gE`
    WordEndBackward {
        big: bool,
    },
    /// `0`
    LineStart,
    /// `^`
    FirstNonBlank,
    /// `$`
    LineEnd,
    /// `|`: column `count`
    Column,
    /// `+`, `Enter`: first non-blank of the next line
    NextLineStart,
    /// `-`: first non-blank of the previous line
    PrevLineStart,
    /// `_`: first non-blank of the line `count - 1` below
    CurrentLineStart,
    /// `gg` (with a count: that line)
    FileStart,
    /// `G` (with a count: that line)
    FileEnd,
    /// `}`
    ParagraphForward,
    /// `{`
    ParagraphBackward,
    /// `%`: the matching bracket
    MatchPair,
    /// `f<c>` / `t<c>` (`forward`), `F<c>` / `T<c>`; `till` stops before the char
    Find {
        forward: bool,
        till: bool,
        ch: char,
    },
    /// `;` / `,` (`reverse`)
    RepeatFind {
        reverse: bool,
    },
    /// `'<c>` (`linewise`) / `` `<c> ``
    Mark {
        name: char,
        linewise: bool,
    },
    /// `H`
    ScreenTop,
    /// `M`
    ScreenMiddle,
    /// `L`
    ScreenBottom,
    /// `Ctrl+D` / `Ctrl+U`
    HalfPage {
        down: bool,
    },
    /// `Ctrl+F` / `Ctrl+B`
    Page {
        down: bool,
    },
    /// `n` / `N` (`reverse`)
    SearchNext {
        reverse: bool,
    },
}

impl Motion {
    /// Jumps are recorded in the jump list (`Ctrl+O` goes back), like Vim.
    pub(super) const fn is_jump(self) -> bool {
        matches!(
            self,
            Self::FileStart
                | Self::FileEnd
                | Self::ParagraphForward
                | Self::ParagraphBackward
                | Self::MatchPair
                | Self::Mark { .. }
                | Self::ScreenTop
                | Self::ScreenMiddle
                | Self::ScreenBottom
                | Self::SearchNext { .. }
        )
    }
}

/// `d`, `c`, `y`, `>`, `<`, `gu`, `gU`, `g~`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Operator {
    Delete,
    Change,
    Yank,
    Indent,
    Outdent,
    Lower,
    Upper,
    ToggleCase,
}

/// Text objects after `i` / `a`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ObjectKind {
    /// `w`
    Word,
    /// `W`
    BigWord,
    /// `p`
    Paragraph,
    /// `"`, `'`, `` ` ``
    Quote(char),
    /// `(` `)` `b`, `[` `]`, `{` `}` `B`, `<` `>`: (open, close)
    Pair(char, char),
}

/// What an operator acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    Motion(Motion),
    /// `dd`, `cc`, `yy`, `>>`, `gUU`, ...
    Line,
    /// `iw`, `a(`, ... (`around`: the `a` form)
    Object {
        kind: ObjectKind,
        around: bool,
    },
}

/// Where `zz` / `zt` / `zb` put the cursor line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScrollTo {
    Center,
    Top,
    Bottom,
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
    /// `gi`: where Insert mode was left last time
    InsertAtLastInsert,
    /// `gI`: column 0
    InsertAtColumnZero,
    /// `o`
    OpenBelow,
    /// `O`
    OpenAbove,
    /// `J` (`spaces`) / `gJ`
    JoinLines { spaces: bool },
    /// `~`
    ToggleCase,
    /// `r<char>`
    Replace(char),
    /// `u`
    Undo,
    /// `Ctrl+R`
    Redo,
    /// `.`
    Repeat,
    /// `v`
    Visual,
    /// `Ctrl+V`
    VisualBlock,
    /// `V`
    VisualLine,
    /// `gv`
    Reselect,
    /// `:`
    CommandLine,
    /// `/`
    SearchForward,
    /// `?`
    SearchBackward,
    /// `*` / `#` (`whole`), `g*` / `g#`: search the word under the cursor
    SearchWord { forward: bool, whole: bool },
    /// `K`: show hover docs at the cursor (like `vim.lsp.buf.hover`).
    Hover,
    /// `gx`: open the URL under the cursor.
    OpenUrl,
    /// `Ctrl+A` (`up`) / `Ctrl+X`: add / subtract `count` to the number at or after the cursor
    Increment { up: bool },
    /// `m<c>`
    SetMark(char),
    /// `q<c>`: start recording a macro (`q` stops)
    Record(char),
    /// `@<c>` (`@@`: the last one)
    Play(char),
    /// `Ctrl+O` (`older`) / `Ctrl+I`, `Tab`
    Jump { older: bool },
    /// `zz`, `zt`, `zb` (+ `z.`, `z<CR>`, `z-`, which also go to the first non-blank)
    ScrollCursor { to: ScrollTo, first_non_blank: bool },
    /// `Ctrl+E` (`down`) / `Ctrl+Y`: scroll the view by `count` lines
    ScrollLines { down: bool },
    /// Keys that do nothing here (e.g., `Ctrl+W`, window commands).
    Nop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Command {
    Move(Motion),
    Operate(Operator, Target),
    Simple(Simple),
}
