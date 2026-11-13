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
