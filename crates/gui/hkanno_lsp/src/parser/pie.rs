//! Parser for Payload Interpreter lines.
//!
//! # Grammar
//! ```text
//! <space0> <time> <space1> <event> . <prefix> <NAME> ( | <param> )*
//! e.g.,     0.1             PIE    . @        SGVB    |IsAttacking|1
//! ```

use super::{cursor::Cursor, nodes::*};
use crate::Span;

pub(super) fn pie(line: &str) -> PieNode<'_> {
    let mut c = Cursor::new(line);
    c.space();
    let time = c.number();
    let space_after_time = c.space();
    let event = c.until(&['.']);

    let dot = (c.peek() == Some('.')).then(|| {
        let start = c.col();
        c.bump();
        Span::new(start, c.col())
    });

    let instruction = c.peek().and_then(PiePrefix::from_char).map(|prefix| {
        let start = c.col();
        c.bump();
        let prefix = Field { value: prefix, span: Span::new(start, c.col()) };
        let name = c.until(&['|']);
        Instruction { prefix, name, parameters: parameters(&mut c) }
    });

    PieNode { time, space_after_time, event, dot, instruction }
}

/// `|a|b|...` until the end of the line. `None` if the line does not continue with `|`.
fn parameters<'a>(c: &mut Cursor<'a>) -> Option<Parameters<'a>> {
    if c.peek() != Some('|') {
        return None;
    }

    let start = c.col();
    let mut items = Vec::new();
    while !c.is_eof() {
        let separator = (c.peek() == Some('|')).then(|| {
            let start = c.col();
            c.bump();
            Span::new(start, c.col())
        });
        items.push(ParameterItem { separator, value: c.until(&['|']) });
    }

    Some(Parameters { items, span: Span::new(start, c.col()) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_instruction() {
        let node = pie("0.1 PIE.@SGVB|IsAttacking|1");
        assert_eq!(node.event.map(|e| e.value), Some("PIE"));
        assert!(node.dot.is_some());

        let ins = node.instruction.expect("instruction");
        assert_eq!(ins.prefix.value, PiePrefix::Native);
        assert_eq!(ins.name.map(|n| n.value), Some("SGVB"));

        let values: Vec<_> = ins
            .parameters
            .expect("parameters")
            .items
            .into_iter()
            .map(|item| item.value.map(|v| v.value))
            .collect();
        assert_eq!(values, [Some("IsAttacking"), Some("1")]);
    }

    #[test]
    fn half_typed() {
        let node = pie("0.1 PIE.");
        assert!(node.dot.is_some());
        assert!(node.instruction.is_none());
    }
}
