//! Conversions between the three ways the editor talks about a position:
//!
//! - **char index** into the whole text: what egui's `CCursor` uses
//! - **line + column** (both 0-based, column in chars): what [`super::Language`] methods use
//! - **byte offset**: what `String` slicing needs
//!
//! e.g., in `"ab\n␀c"` the `c` is char index 4, `LinePos { line: 1, column: 1 }`, byte offset 6.

/// 0-based line and char column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinePos {
    pub(crate) line: usize,
    pub(crate) column: usize,
}

impl LinePos {
    /// Char index -> line/column. Indices past the end clamp to the end.
    pub(crate) fn from_char_index(text: &str, index: usize) -> Self {
        let mut pos = Self { line: 0, column: 0 };
        for c in text.chars().take(index) {
            if c == '\n' {
                pos.line += 1;
                pos.column = 0;
            } else {
                pos.column += 1;
            }
        }
        pos
    }

    /// Line/column -> char index. A column past the line end clamps to the line end.
    pub(crate) fn to_char_index(self, text: &str) -> usize {
        let mut index = 0;
        for (line_no, line) in text.split('\n').enumerate() {
            if line_no == self.line {
                return index + self.column.min(line.chars().count());
            }
            index += line.chars().count() + 1; // + '\n'
        }
        index.saturating_sub(1)
    }
}

/// Char index -> byte offset (clamped to the text end).
pub(crate) fn char_to_byte(text: &str, index: usize) -> usize {
    text.char_indices().nth(index).map_or(text.len(), |(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let text = "ab\n␀c";
        let pos = LinePos::from_char_index(text, 4);
        assert_eq!(pos, LinePos { line: 1, column: 1 });
        assert_eq!(pos.to_char_index(text), 4);
        assert_eq!(char_to_byte(text, 4), 6);
    }
}
