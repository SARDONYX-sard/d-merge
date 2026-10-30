//! Text positions.

/// A range of `char` columns inside a single line. `end` is exclusive.
///
/// e.g., in `"0.5 animmotion"`, `animmotion` is `Span { start: 4, end: 14 }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// First column (0-based, inclusive).
    pub start: usize,
    /// Last column (0-based, exclusive).
    pub end: usize,
}

impl Span {
    /// Creates a span. `end` is exclusive.
    #[inline]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Number of `char`s covered.
    #[inline]
    pub const fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// `true` if the span covers no `char`.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Is `column` inside the span? Both ends are **inclusive**,
    /// so a cursor placed right after the last char still counts (hover feels natural that way).
    #[inline]
    pub const fn touches(&self, column: usize) -> bool {
        self.start <= column && column <= self.end
    }

    /// Attaches a line number.
    #[inline]
    pub const fn on_line(self, line: usize) -> LineSpan {
        LineSpan { line, span: self }
    }
}

/// A [`Span`] plus its 0-based line number. Used by document-wide results (e.g., diagnostics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineSpan {
    /// 0-based line number.
    pub line: usize,
    /// Columns inside that line.
    pub span: Span,
}
