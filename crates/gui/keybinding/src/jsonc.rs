//! JSON with comments (`//`, `/* */`) and trailing commas, as VS Code's settings files are: taken
//! out, so that plain JSON is left. Strings are kept as they are.
//!
//! e.g. `[1, // one\n 2,]` -> `[1, \n 2]`

/// `text` as plain JSON. The line breaks of the comments stay, so errors point at the same lines.
pub(crate) fn strip(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push(c);
                while let Some(c) = chars.next() {
                    out.push(c);
                    match c {
                        '\\' => out.extend(chars.next()),
                        '"' => break,
                        _ => {}
                    }
                }
            }
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut last = '\0';
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                    }
                    if last == '*' && c == '/' {
                        break;
                    }
                    last = c;
                }
            }
            ']' | '}' => {
                drop_trailing_comma(&mut out);
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out
}

/// The byte offset of the last `]` outside strings and comments, and of the last such char before it
/// that is not whitespace (`None`: nothing before it).
///
/// e.g. `[1, // x\n]` -> `(8, Some(2))`, the `]` and the `,`
pub(crate) fn last_close_bracket(text: &str) -> Option<(usize, Option<usize>)> {
    let bytes = text.as_bytes();
    let (mut close, mut last, mut before_close) = (None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'"' => {
                at += 1;
                while at < bytes.len() && bytes[at] != b'"' {
                    at += if bytes[at] == b'\\' { 2 } else { 1 };
                }
                last = Some(at); // the closing quote
            }
            b'/' if bytes.get(at + 1) == Some(&b'/') => {
                while at < bytes.len() && bytes[at] != b'\n' {
                    at += 1;
                }
            }
            b'/' if bytes.get(at + 1) == Some(&b'*') => {
                at += 2;
                while at + 1 < bytes.len() && !(bytes[at] == b'*' && bytes[at + 1] == b'/') {
                    at += 1;
                }
                at += 1;
            }
            b']' => {
                close = Some(at);
                before_close = last;
                last = Some(at);
            }
            c if !c.is_ascii_whitespace() => last = Some(at),
            _ => {}
        }
        at += 1;
    }
    close.map(|close| (close, before_close))
}

/// `[1, 2, ` + `]`: the comma before the closing bracket goes (whitespace after it stays).
fn drop_trailing_comma(out: &mut String) {
    let trimmed = out.trim_end().len();
    if out[..trimmed].ends_with(',') {
        out.remove(trimmed - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_and_trailing_commas_go() {
        let text = "[\n  // a comment\n  { \"key\": \"a//b\", /* x */ \"c\": \"d,]\", },\n]";
        let json: serde_json::Value = serde_json::from_str(&strip(text)).unwrap();
        assert_eq!(json[0]["key"], "a//b", "not in a string");
        assert_eq!(json[0]["c"], "d,]");
        assert_eq!(strip("[1,\n]").lines().count(), 2, "the lines stay");
        assert_eq!(strip(r#""a\"//b""#), r#""a\"//b""#, "an escaped quote");
    }
}
