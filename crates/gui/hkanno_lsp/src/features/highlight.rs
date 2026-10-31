//! Syntax highlighting.
//!
//! Two layers, like the former Monaco setup:
//! 1. **basic**: a cheap word scanner (comments, numbers, well-known event names).
//!    Always on, so even unparseable lines get some color.
//! 2. **semantic** (optional): parser-based tokens drawn **over** the basic layer.
//!    e.g., the parameters of `PIE.@SGVB|IsAttacking|1` become string / number.
//! 3. **JSON** (always): the `{...}` body of `SpecialFrames_Invincible{...}` /
//!    `SCAR_ActionData{...}`: keys, strings, numbers, literals and punctuation.

use crate::{
    Span,
    parser::{LineNode, Syntax, parse_line},
};

/// Color category of a token (the GUI maps each to a theme color).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// `# ...`
    Comment,
    /// `0.5`, `-1`
    Number,
    /// `"..."`, track names, annotation text
    String,
    /// `animmotion`, `SpecialFrames_Invincible`
    Identifier,
    /// `PIE`, `trackName:`
    Variable,
    /// PIE instruction name, e.g., `SGVB`
    Function,
    /// PIE prefix `@` / `$` / `!`
    Keyword,
    /// PIE `|` separator
    Operator,
}

/// Non-overlapping, sorted token ranges for one line.
///
/// `semantic`: enables the parser-based layer (the "semantic tokens" LSP option).
///
/// # Example
/// ```
/// use hkanno_lsp::{Span, features::highlight::{highlight_line, TokenKind}};
///
/// let tokens = highlight_line("# comment", false);
/// assert_eq!(tokens, [(Span::new(0, 9), TokenKind::Comment)]);
/// ```
pub fn highlight_line(line: &str, semantic: bool) -> Vec<(Span, TokenKind)> {
    // One slot per char; later layers overwrite earlier ones. Lines are short, so this is cheap
    // and much simpler than merging overlapping ranges.
    let mut kinds: Vec<Option<TokenKind>> = vec![None; line.chars().count()];
    let mut paint = |span: Span, kind: TokenKind| {
        let end = span.end.min(kinds.len());
        for slot in kinds.iter_mut().take(end).skip(span.start) {
            *slot = Some(kind);
        }
    };

    for (span, kind) in basic_tokens(line) {
        paint(span, kind);
    }
    if semantic {
        for (span, kind) in semantic_tokens(line) {
            paint(span, kind);
        }
    }
    for (span, kind) in json_tokens(line) {
        paint(span, kind);
    }

    // Compress runs of the same kind back into spans.
    let mut out: Vec<(Span, TokenKind)> = Vec::new();
    for (col, kind) in kinds.into_iter().enumerate() {
        let Some(kind) = kind else { continue };
        match out.last_mut() {
            Some((span, last)) if *last == kind && span.end == col => span.end = col + 1,
            _ => out.push((Span::new(col, col + 1), kind)),
        }
    }
    out
}

/// Parser-based tokens (layer 2).
pub fn semantic_tokens(line: &str) -> Vec<(Span, TokenKind)> {
    use TokenKind::*;

    let mut tokens = Vec::new();
    let mut push = |span: Option<Span>, kind| {
        if let Some(span) = span {
            tokens.push((span, kind));
        }
    };

    match parse_line(line, Syntax::Extended) {
        LineNode::Pie(node) => {
            push(node.event.map(|e| e.span), Variable);
            if let Some(ins) = node.instruction {
                push(Some(ins.prefix.span), Keyword);
                push(ins.name.map(|n| n.span), Function);
                for item in ins.parameters.into_iter().flat_map(|p| p.items) {
                    push(item.separator, Operator);
                    if let Some(value) = item.value {
                        let kind = if starts_like_float(value.value) { Number } else { String };
                        push(Some(value.span), kind);
                    }
                }
            }
        }
        LineNode::TrackName(node) => {
            push(node.literal, Variable);
            push(node.name.map(|n| n.span), String);
        }
        LineNode::Motion(node) => {
            push(node.event, Identifier);
            for axis in [node.x, node.y, node.z] {
                push(axis.map(|f| f.span), Number);
            }
        }
        LineNode::Rotation(node) => {
            push(node.event, Identifier);
            push(node.degrees.map(|f| f.span), Number);
        }
        LineNode::IFrame(node) => push(node.event, Identifier),
        LineNode::Text(node) => {
            push(node.time.map(|f| f.span), Number);
            // SCAR events are names (+ JSON, see `json_tokens`), not free text.
            let is_scar = node.text.is_some_and(|t| t.value.trim_start().starts_with("SCAR_"));
            if !is_scar {
                push(node.text.map(|f| f.span), String);
            }
        }
        LineNode::Comment(_) => {}
    }
    tokens
}

/// Word scanner (layer 1). Mirrors the old Monarch tokenizer.
pub fn basic_tokens(line: &str) -> Vec<(Span, TokenKind)> {
    const IDENTIFIERS: [&str; 7] = [
        "animmotion",
        "animrotation",
        "SoundPlay",
        "SpecialFrames_Invincible",
        "SCAR_ActionData",
        "SCAR_ComboStart",
        "SCAR_UpdateDummy",
    ];

    let chars: Vec<char> = line.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let start = i;
        let c = chars[i];
        let next_is_digit = chars.get(i + 1).is_some_and(char::is_ascii_digit);

        if c == '#' {
            // e.g., `# numAnnotations: 3` -> the rest of the line.
            tokens.push((Span::new(start, chars.len()), TokenKind::Comment));
            break;
        } else if c.is_ascii_digit() || (c == '-' && next_is_digit) {
            // e.g., `-1.5`
            i += 1;
            while chars.get(i).is_some_and(|c| c.is_ascii_digit() || *c == '.') {
                i += 1;
            }
            tokens.push((Span::new(start, i), TokenKind::Number));
        } else if c == '"' {
            // e.g., `"Duration"`; an unclosed quote runs to the end.
            i += 1;
            while chars.get(i).is_some_and(|c| *c != '"') {
                i += 1;
            }
            i = (i + 1).min(chars.len());
            tokens.push((Span::new(start, i), TokenKind::String));
        } else if c.is_alphanumeric() || c == '_' {
            while chars.get(i).is_some_and(|c| c.is_alphanumeric() || *c == '_') {
                i += 1;
            }
            let word: std::string::String = chars[start..i].iter().collect();
            if word == "PIE" || word == "pie" {
                tokens.push((Span::new(start, i), TokenKind::Variable));
            } else if IDENTIFIERS.contains(&word.as_str()) {
                tokens.push((Span::new(start, i), TokenKind::Identifier));
            }
        } else {
            i += 1;
        }
    }
    tokens
}

/// JSON body tokens (layer 3): from the first `{` of an event line to the end.
///
/// e.g., `0.1 SCAR_ActionData{"Type":"RA", "Chance":30}` ->
/// `{` `"Type"`(key) `:` `"RA"`(string) `,` `"Chance"`(key) `:` `30`(number) `}`
pub fn json_tokens(line: &str) -> Vec<(Span, TokenKind)> {
    let chars: Vec<char> = line.chars().collect();
    // Only event payloads, not `#` comments or annotation text with a stray brace.
    let Some(open) = chars.iter().position(|&c| c == '{') else { return Vec::new() };
    if chars[..open].contains(&'#') {
        return Vec::new();
    }

    let mut tokens = Vec::new();
    let mut i = open;
    while i < chars.len() {
        let start = i;
        let c = chars[i];
        match c {
            '{' | '}' | '[' | ']' | ':' | ',' => {
                tokens.push((Span::new(i, i + 1), TokenKind::Operator));
                i += 1;
            }
            '"' => {
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    i += usize::from(chars[i] == '\\') + 1; // skip escaped chars, e.g. `\"`
                }
                i = (i + 1).min(chars.len());
                // A string followed by `:` is a key.
                let next = chars[i..].iter().find(|c| !c.is_whitespace());
                let kind = if next == Some(&':') { TokenKind::Variable } else { TokenKind::String };
                tokens.push((Span::new(start, i), kind));
            }
            c if c == '-' || c.is_ascii_digit() => {
                i += 1;
                while chars
                    .get(i)
                    .is_some_and(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'))
                {
                    i += 1;
                }
                tokens.push((Span::new(start, i), TokenKind::Number));
            }
            c if c.is_ascii_alphabetic() => {
                while chars.get(i).is_some_and(char::is_ascii_alphabetic) {
                    i += 1;
                }
                let word: std::string::String = chars[start..i].iter().collect();
                if matches!(word.as_str(), "true" | "false" | "null") {
                    tokens.push((Span::new(start, i), TokenKind::Keyword));
                }
            }
            _ => i += 1,
        }
    }
    tokens
}

/// JS `!isNaN(parseFloat(s))`: does `s` begin with a number?
///
/// e.g., `"1"`, `"0.5x"`, `"-.5"` -> `true`; `"IsAttacking"` -> `false`.
fn starts_like_float(s: &str) -> bool {
    let s = s.trim_start();
    let s = s.strip_prefix(['+', '-']).unwrap_or(s);
    let s = s.strip_prefix('.').unwrap_or(s);
    s.starts_with(|c: char| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_body() {
        let line = r#"0.1 SCAR_ActionData{"Type":"RA", "Chance":30, "X":true}"#;
        let kinds: Vec<_> = json_tokens(line).into_iter().map(|(_, k)| k).collect();
        use TokenKind::*;
        assert_eq!(
            kinds,
            [
                Operator, Variable, Operator, String, Operator, Variable, Operator, Number,
                Operator, Variable, Operator, Keyword, Operator
            ]
        );
        // The event name is an identifier, not annotation text.
        assert!(highlight_line(line, true).contains(&(Span::new(4, 19), Identifier)));
    }

    #[test]
    fn semantic_overrides_basic() {
        let tokens = highlight_line("0.1 PIE.@SGVB|IsAttacking|1", true);
        let kinds: Vec<_> = tokens.iter().map(|(_, k)| *k).collect();
        assert_eq!(
            kinds,
            [
                TokenKind::Number,   // 0.1
                TokenKind::Variable, // PIE
                TokenKind::Keyword,  // @
                TokenKind::Function, // SGVB
                TokenKind::Operator, // |
                TokenKind::String,   // IsAttacking
                TokenKind::Operator, // |
                TokenKind::Number,   // 1
            ]
        );
    }
}
