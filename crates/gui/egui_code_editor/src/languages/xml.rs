//! XML syntax highlighting (line by line), tuned for Havok XML (`.xml` hkx).
//!
//! ```text
//! <hkparam name="time">0.100000</hkparam>   <!-- comment -->
//! ^^^^^^^^ ^^^^ ^^^^^^ ^^^^^^^^ ^^^^^^^^^   ^^^^^^^^^^^^^^^^
//! tag      attr value  number   tag         comment
//! ```
//! Also colors pointers such as `#0042`. A comment spanning several lines is only colored on
//! its first line (each line is highlighted on its own); Havok XML does not use those.

use std::ops::Range;

use egui::Color32;

use crate::language::Language;

/// XML highlighter. e.g., `CodeEditor::new(..).language(&Xml)` or [`crate::line_job`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Xml;

impl Language for Xml {
    fn highlight(&self, line: &str, visuals: &egui::Visuals) -> Vec<(Range<usize>, Color32)> {
        let palette = Palette::new(visuals.dark_mode);
        let chars: Vec<char> = line.chars().collect();
        let mut out = Vec::new();
        let mut i = 0;

        while i < chars.len() {
            if starts_with(&chars, i, "<!--") {
                let end = find(&chars, i + 4, "-->").map_or(chars.len(), |e| e + 3);
                out.push((i..end, palette.comment));
                i = end;
            } else if chars[i] == '<' {
                i = tag(&chars, i, &palette, &mut out);
            } else {
                i = text_token(&chars, i, &palette, &mut out);
            }
        }
        out
    }
}

/// One Dark / One Light colors (same family as the hkanno editor).
struct Palette {
    punct: Color32,
    tag: Color32,
    attr: Color32,
    string: Color32,
    number: Color32,
    pointer: Color32,
    comment: Color32,
}

impl Palette {
    const fn new(dark: bool) -> Self {
        const fn rgb(dark: bool, d: (u8, u8, u8), l: (u8, u8, u8)) -> Color32 {
            let (r, g, b) = if dark { d } else { l };
            Color32::from_rgb(r, g, b)
        }
        Self {
            punct: rgb(dark, (127, 132, 142), (160, 161, 167)),
            tag: rgb(dark, (224, 108, 117), (228, 86, 73)),
            attr: rgb(dark, (209, 154, 102), (152, 104, 1)),
            string: rgb(dark, (152, 195, 121), (80, 161, 79)),
            number: rgb(dark, (209, 154, 102), (152, 104, 1)),
            pointer: rgb(dark, (97, 175, 239), (64, 120, 242)),
            comment: rgb(dark, (127, 132, 142), (160, 161, 167)),
        }
    }
}

/// `<name attr="value" ...>` / `</name>` / `<?xml ...?>` starting at `start` (`<`).
/// Returns the index after the tag (or the line end for an unclosed tag).
fn tag(chars: &[char], start: usize, p: &Palette, out: &mut Vec<(Range<usize>, Color32)>) -> usize {
    // `<`, `</`, `<?`
    let mut i = start + 1;
    if matches!(chars.get(i), Some('/' | '?')) {
        i += 1;
    }
    out.push((start..i, p.punct));

    let name_end = scan(chars, i, |c| !c.is_whitespace() && !matches!(c, '>' | '/' | '?'));
    out.push((i..name_end, p.tag));
    i = name_end;

    while i < chars.len() {
        match chars[i] {
            '>' => {
                out.push((i..i + 1, p.punct));
                return i + 1;
            }
            '/' | '?' | '=' => {
                out.push((i..i + 1, p.punct));
                i += 1;
            }
            quote @ ('"' | '\'') => {
                let end = chars[i + 1..]
                    .iter()
                    .position(|&c| c == quote)
                    .map_or(chars.len(), |n| i + n + 2);
                out.push((i..end, p.string));
                i = end;
            }
            c if c.is_whitespace() => i += 1,
            _ => {
                let end = scan(chars, i, |c| !c.is_whitespace() && !matches!(c, '=' | '>' | '/'));
                out.push((i..end, p.attr));
                i = end;
            }
        }
    }
    i
}

/// One word of text between tags. e.g., `0.100000`, `#0042`, `(1.0`
fn text_token(
    chars: &[char],
    start: usize,
    p: &Palette,
    out: &mut Vec<(Range<usize>, Color32)>,
) -> usize {
    let is_sep = |c: char| c.is_whitespace() || matches!(c, '<' | '(' | ')' | ',');
    if is_sep(chars[start]) {
        return start + 1;
    }
    let end = scan(chars, start, |c| !is_sep(c));
    let word: String = chars[start..end].iter().collect();
    if word.starts_with('#') && word.len() > 1 {
        out.push((start..end, p.pointer));
    } else if word.parse::<f64>().is_ok() || word == "true" || word == "false" {
        out.push((start..end, p.number));
    }
    end
}

/// End of the run of chars from `start` that satisfy `keep`.
fn scan(chars: &[char], start: usize, keep: impl Fn(char) -> bool) -> usize {
    chars[start..].iter().position(|&c| !keep(c)).map_or(chars.len(), |n| start + n)
}

fn starts_with(chars: &[char], at: usize, pat: &str) -> bool {
    pat.chars().enumerate().all(|(n, p)| chars.get(at + n) == Some(&p))
}

fn find(chars: &[char], from: usize, pat: &str) -> Option<usize> {
    (from..chars.len()).find(|&i| starts_with(chars, i, pat))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str) -> Vec<(String, Color32)> {
        let chars: Vec<char> = line.chars().collect();
        Xml.highlight(line, &egui::Visuals::dark())
            .into_iter()
            .map(|(r, c)| (chars[r].iter().collect(), c))
            .collect()
    }

    #[test]
    fn colors_tag_attr_value_and_number() {
        let p = Palette::new(true);
        let got = kinds(r#"<hkparam name="time">0.5</hkparam>"#);
        assert!(got.contains(&("hkparam".into(), p.tag)));
        assert!(got.contains(&("name".into(), p.attr)));
        assert!(got.contains(&(r#""time""#.into(), p.string)));
        assert!(got.contains(&("0.5".into(), p.number)));
        assert!(got.contains(&("</".into(), p.punct)));
    }

    #[test]
    fn colors_comment_and_pointer() {
        let p = Palette::new(true);
        let got = kinds("#0042 <!-- note -->");
        assert_eq!(got, vec![("#0042".into(), p.pointer), ("<!-- note -->".into(), p.comment)]);
    }
}
