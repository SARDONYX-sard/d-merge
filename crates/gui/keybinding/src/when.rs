//! `when`: where a binding works, as VS Code writes it. e.g. `editorFocus && vim.mode == 'normal'`
//!
//! ```text
//! expr    := or
//! or      := and ('||' and)*
//! and     := unary ('&&' unary)*
//! unary   := '!' unary | primary
//! primary := '(' expr ')' | name ('==' | '!=') value | name
//! value   := 'text' | "text" | word
//! ```
//! A name alone is true if it is a true flag or a non-empty text; a name the context does not
//! know is false. The names are those of [`crate::KeyContext`].

/// A value of the context: a flag (`editorFocus`) or a text (`vim.mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value<'a> {
    Bool(bool),
    Text(&'a str),
}

/// What a `when` is evaluated against: the names it may use.
pub trait Context {
    /// The value of `name`, `None` if it has none.
    fn value(&self, name: &str) -> Option<Value<'_>>;
}

/// A parsed `when`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum When {
    /// `name`: true if it is true / not empty.
    Is(String),
    /// `name == value` (`equal`), `name != value`.
    Compare {
        name: String,
        value: String,
        equal: bool,
    },
    Not(Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
}

impl When {
    /// Reads `text`.
    ///
    /// # Errors
    /// What in it is not of the grammar of the module docs, and where.
    pub fn parse(text: &str) -> Result<Self, String> {
        let tokens = tokenize(text)?;
        let mut parser = Parser { tokens: &tokens, at: 0 };
        let when = parser.or()?;
        match parser.tokens.get(parser.at) {
            None => Ok(when),
            Some(token) => Err(format!("`{token}` is not expected here")),
        }
    }

    /// Is it true in `context`?
    pub fn eval(&self, context: &dyn Context) -> bool {
        match self {
            Self::Is(name) => match context.value(name) {
                Some(Value::Bool(on)) => on,
                Some(Value::Text(text)) => !text.is_empty(),
                None => false,
            },
            Self::Compare { name, value, equal } => {
                let same = match context.value(name) {
                    Some(Value::Text(text)) => text == value,
                    Some(Value::Bool(on)) => value == if on { "true" } else { "false" },
                    None => false,
                };
                same == *equal
            }
            Self::Not(inner) => !inner.eval(context),
            Self::And(a, b) => a.eval(context) && b.eval(context),
            Self::Or(a, b) => a.eval(context) || b.eval(context),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Word(String),
    /// A quoted text: never a name.
    Quoted(String),
    And,
    Or,
    Not,
    Equal,
    NotEqual,
    Open,
    Close,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Word(word) => f.write_str(word),
            Self::Quoted(text) => write!(f, "'{text}'"),
            Self::And => f.write_str("&&"),
            Self::Or => f.write_str("||"),
            Self::Not => f.write_str("!"),
            Self::Equal => f.write_str("=="),
            Self::NotEqual => f.write_str("!="),
            Self::Open => f.write_str("("),
            Self::Close => f.write_str(")"),
        }
    }
}

fn tokenize(text: &str) -> Result<Vec<Token>, String> {
    let is_word = |c: char| c.is_alphanumeric() || matches!(c, '.' | '_' | '-' | ':');
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        let next = chars.peek().map(|&(_, next)| next);
        let token = match (c, next) {
            (c, _) if c.is_whitespace() => continue,
            ('&', Some('&')) => Token::And,
            ('|', Some('|')) => Token::Or,
            ('=', Some('=')) => Token::Equal,
            ('!', Some('=')) => Token::NotEqual,
            ('!', _) => Token::Not,
            ('(', _) => Token::Open,
            (')', _) => Token::Close,
            ('\'' | '"', _) => {
                let rest = &text[at + 1..];
                let end = rest.find(c).ok_or_else(|| format!("the quote at {at} is not closed"))?;
                for _ in 0..rest[..=end].chars().count() {
                    chars.next();
                }
                tokens.push(Token::Quoted(rest[..end].to_owned()));
                continue;
            }
            (c, _) if is_word(c) => {
                let rest = &text[at..];
                let end = rest.find(|c: char| !is_word(c)).unwrap_or(rest.len());
                for _ in 1..rest[..end].chars().count() {
                    chars.next();
                }
                tokens.push(Token::Word(rest[..end].to_owned()));
                continue;
            }
            (c, _) => return Err(format!("`{c}` at {at} is not part of a `when`")),
        };
        if matches!(token, Token::And | Token::Or | Token::Equal | Token::NotEqual) {
            chars.next(); // the second char of the operator
        }
        tokens.push(token);
    }
    Ok(tokens)
}

struct Parser<'a> {
    tokens: &'a [Token],
    at: usize,
}

impl Parser<'_> {
    fn next(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.at);
        self.at += 1;
        token
    }

    fn eat(&mut self, token: &Token) -> bool {
        let found = self.tokens.get(self.at) == Some(token);
        self.at += usize::from(found);
        found
    }

    fn or(&mut self) -> Result<When, String> {
        let mut left = self.and()?;
        while self.eat(&Token::Or) {
            left = When::Or(Box::new(left), Box::new(self.and()?));
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<When, String> {
        let mut left = self.unary()?;
        while self.eat(&Token::And) {
            left = When::And(Box::new(left), Box::new(self.unary()?));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<When, String> {
        if self.eat(&Token::Not) {
            return Ok(When::Not(Box::new(self.unary()?)));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<When, String> {
        match self.next().cloned() {
            Some(Token::Open) => {
                let inner = self.or()?;
                if self.eat(&Token::Close) { Ok(inner) } else { Err("a `(` is not closed".to_owned()) }
            }
            Some(Token::Word(name)) => {
                let equal = if self.eat(&Token::Equal) {
                    true
                } else if self.eat(&Token::NotEqual) {
                    false
                } else {
                    return Ok(When::Is(name));
                };
                match self.next() {
                    Some(Token::Word(value) | Token::Quoted(value)) => {
                        Ok(When::Compare { name, value: value.clone(), equal })
                    }
                    _ => Err(format!("`{name}` is compared with nothing")),
                }
            }
            Some(token) => Err(format!("`{token}` is not expected here")),
            None => Err("it ends too early".to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Map(&'static [(&'static str, Value<'static>)]);
    impl Context for Map {
        fn value(&self, name: &str) -> Option<Value<'_>> {
            self.0.iter().find(|(known, _)| *known == name).map(|(_, value)| *value)
        }
    }

    const CONTEXT: Map = Map(&[
        ("editorFocus", Value::Bool(true)),
        ("inputFocus", Value::Bool(false)),
        ("vim.mode", Value::Text("normal")),
    ]);

    fn eval(text: &str) -> bool {
        When::parse(text).unwrap().eval(&CONTEXT)
    }

    #[test]
    fn whens_are_evaluated_as_vs_code_does() {
        assert!(eval("editorFocus"));
        assert!(!eval("inputFocus"));
        assert!(!eval("unknownName"), "an unknown name is false");
        assert!(eval("editorFocus && vim.mode == 'normal'"));
        assert!(eval("vim.mode == normal"));
        assert!(eval(r#"vim.mode != "insert""#));
        assert!(eval("!inputFocus && editorFocus"));
        assert!(eval("inputFocus || editorFocus"));
        // `&&` before `||`, as VS Code.
        assert!(eval("editorFocus || inputFocus && unknownName"));
        assert!(!eval("(editorFocus || inputFocus) && unknownName"));
        assert!(eval("!!editorFocus"));
        assert!(eval("editorFocus == true"));
    }

    #[test]
    fn what_is_not_a_when_is_told() {
        for bad in ["", "&& a", "a &&", "(a", "a b", "a == ", "a ='x'", "'open"] {
            assert!(When::parse(bad).is_err(), "{bad}");
        }
    }
}
