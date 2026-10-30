//! Low-level scanning helpers shared by every line parser.
//!
//! Each helper either consumes something and returns it, or consumes nothing and returns `None`.
//! This "optional piece" style is what lets the parsers accept half-typed lines.
//!
//! # Two positions
//! The cursor tracks both a **byte** offset (to borrow `&str` slices from the line without copying)
//! and a **char** column (what [`Span`] exposes to the GUI).
//! e.g., after reading `"0.5 ␀"` the byte offset is 7 but the column is 5.

use super::nodes::Field;
use crate::Span;

/// A position inside one line.
pub(crate) struct Cursor<'a> {
    line: &'a str,
    /// Byte offset into `line`.
    byte: usize,
    /// Char column (0-based).
    col: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) const fn new(line: &'a str) -> Self {
        Self { line, byte: 0, col: 0 }
    }

    #[inline]
    pub(crate) fn peek(&self) -> Option<char> {
        self.line[self.byte..].chars().next()
    }

    /// Current char column.
    #[inline]
    pub(crate) const fn col(&self) -> usize {
        self.col
    }

    #[inline]
    pub(crate) const fn is_eof(&self) -> bool {
        self.byte >= self.line.len()
    }

    /// Consumes one char (no-op at the end of the line).
    #[inline]
    pub(crate) fn bump(&mut self) {
        if let Some(c) = self.peek() {
            self.byte += c.len_utf8();
            self.col += 1;
        }
    }

    /// Consumes chars while `pred` holds.
    fn bump_while(&mut self, pred: impl Fn(char) -> bool) {
        while self.peek().is_some_and(&pred) {
            self.bump();
        }
    }

    /// `<space0>` / `<space1>`: consumes whitespace, returns `None` if there was none.
    pub(crate) fn space(&mut self) -> Option<Span> {
        let start = self.col;
        self.bump_while(char::is_whitespace);
        (self.col > start).then(|| Span::new(start, self.col))
    }

    /// A number like `-1.5e3`.
    ///
    /// Accepts the same loose shape as the old TS parser: `[+-]? [0-9.]* ([eE][+-]?[0-9]*)?`.
    /// A shape that is not a valid float (e.g., `1.2.3` or a lone `-`) still yields a field with `NaN`,
    /// so the editor can point at the bad token instead of reporting it as "missing".
    pub(crate) fn number(&mut self) -> Option<Field<f64>> {
        let (start_byte, start_col) = (self.byte, self.col);
        if matches!(self.peek(), Some('+' | '-')) {
            self.bump();
        }
        self.bump_while(|c| c.is_ascii_digit() || c == '.');
        if matches!(self.peek(), Some('e' | 'E')) {
            self.bump();
            if matches!(self.peek(), Some('+' | '-')) {
                self.bump();
            }
            self.bump_while(|c| c.is_ascii_digit());
        }

        if self.col == start_col {
            return None;
        }
        let value = self.line[start_byte..self.byte].parse::<f64>().unwrap_or(f64::NAN);
        Some(Field { value, span: Span::new(start_col, self.col) })
    }

    /// Consumes `literal` if the line continues with it.
    ///
    /// `ignore_case`: e.g., `trackname:` is accepted for `trackName:` (ASCII only).
    pub(crate) fn literal(&mut self, literal: &str, ignore_case: bool) -> Option<Span> {
        let rest = self.line[self.byte..].get(..literal.len())?;
        let same = if ignore_case { rest.eq_ignore_ascii_case(literal) } else { rest == literal };
        if !same {
            return None;
        }

        let start = self.col;
        self.byte += literal.len();
        self.col += literal.chars().count();
        Some(Span::new(start, self.col))
    }

    /// Consumes chars until one of `stops` (or the end). Returns `None` if nothing was consumed.
    ///
    /// e.g., `until(&['|'])` on `"SGVB|x"` returns `"SGVB"` and stops before `|`.
    pub(crate) fn until(&mut self, stops: &[char]) -> Option<Field<&'a str>> {
        let (start_byte, start_col) = (self.byte, self.col);
        self.bump_while(|c| !stops.contains(&c));
        (self.col > start_col).then(|| Field {
            value: &self.line[start_byte..self.byte],
            span: Span::new(start_col, self.col),
        })
    }

    /// Consumes the rest of the line.
    #[inline]
    pub(crate) fn rest(&mut self) -> Option<Field<&'a str>> {
        self.until(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_shapes() {
        assert_eq!(Cursor::new("-1.5e3 x").number().map(|f| f.value), Some(-1500.0));
        assert!(Cursor::new("1.2.3").number().is_some_and(|f| f.value.is_nan()));
        assert_eq!(Cursor::new("abc").number(), None);
    }

    #[test]
    fn literal_case() {
        assert!(Cursor::new("trackname:").literal("trackName:", true).is_some());
        assert!(Cursor::new("trackname:").literal("trackName:", false).is_none());
    }

    #[test]
    fn columns_are_chars() {
        let mut c = Cursor::new("␀ab");
        c.bump();
        assert_eq!(c.col(), 1);
        assert_eq!(c.rest().map(|f| (f.value, f.span)), Some(("ab", Span::new(1, 3))));
    }
}
