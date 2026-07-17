//! SVG path `d`-attribute parser.

/// Parse an SVG path `d` attribute string into a list of path commands.
pub fn parse_path_data(d: &str) -> Vec<super::path_command::PathCommand> {
    let tokens = tokenize(d);
    commands_from_tokens(&tokens)
}

// ── Tokenizer ───────────────────────────────────────────────────

#[derive(Debug)]
enum Token {
    Command(char),
    Number(f32),
}

fn tokenize(d: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = d.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        if ch.is_ascii_whitespace() || ch == ',' {
            i += 1;
            continue;
        }

        if is_command(ch) {
            tokens.push(Token::Command(ch));
            i += 1;
            continue;
        }

        if ch == '-' || ch == '+' || ch == '.' || ch.is_ascii_digit() {
            let start = i;
            let mut has_dot = false;

            // Leading sign
            if ch == '-' || ch == '+' {
                i += 1;
            }

            // Integer part
            while i < len && chars[i].is_ascii_digit() {
                i += 1;
            }

            // Decimal part
            if i < len && chars[i] == '.' {
                has_dot = true;
                i += 1;
                while i < len && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }

            // Exponent
            if i < len && (chars[i] == 'e' || chars[i] == 'E') {
                i += 1;
                if i < len && (chars[i] == '+' || chars[i] == '-') {
                    i += 1;
                }
                while i < len && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }

            let s: String = chars[start..i].iter().collect();
            if let Ok(v) = s.parse::<f32>() {
                tokens.push(Token::Number(v));
            }

            // Handle implicit decimal like ".5" immediately following a number
            // e.g. "1.5.5" → 1.5 then 0.5
            if !has_dot {
                let _ = has_dot; // keep binding for clarity
            }
            continue;
        }

        i += 1;
    }

    tokens
}

fn is_command(ch: char) -> bool {
    matches!(
        ch,
        'M' | 'm'
            | 'L' | 'l'
            | 'H' | 'h'
            | 'V' | 'v'
            | 'C' | 'c'
            | 'S' | 's'
            | 'Q' | 'q'
            | 'T' | 't'
            | 'A' | 'a'
            | 'Z' | 'z'
    )
}

// ── Token → Commands ────────────────────────────────────────────

fn commands_from_tokens(tokens: &[Token]) -> Vec<super::path_command::PathCommand> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut current_cmd = 'M';

    while i < tokens.len() {
        match &tokens[i] {
            Token::Command(c) => {
                current_cmd = *c;
                i += 1;

                if current_cmd == 'Z' || current_cmd == 'z' {
                    out.push(super::path_command::PathCommand::Close);
                    continue;
                }

                if let Some((cmd, consumed)) = parse_one_command(current_cmd, &tokens[i..]) {
                    out.push(cmd);
                    i += consumed;
                    // After M, implicit repeats become L
                    if current_cmd == 'M' {
                        current_cmd = 'L';
                    } else if current_cmd == 'm' {
                        current_cmd = 'l';
                    }
                }
            }
            Token::Number(_) => {
                // Implicit repeat of current command
                if let Some((cmd, consumed)) = parse_one_command(current_cmd, &tokens[i..]) {
                    out.push(cmd);
                    i += consumed;
                } else {
                    i += 1; // skip unparseable
                }
            }
        }
    }

    out
}

fn take_number(tokens: &[Token], idx: usize) -> Option<f32> {
    match tokens.get(idx) {
        Some(Token::Number(v)) => Some(*v),
        _ => None,
    }
}

fn take_flag(tokens: &[Token], idx: usize) -> Option<bool> {
    take_number(tokens, idx).map(|v| v != 0.0)
}

fn parse_one_command(
    cmd: char,
    tokens: &[Token],
) -> Option<(super::path_command::PathCommand, usize)> {
    match cmd {
        'M' => {
            let x = take_number(tokens, 0)?;
            let y = take_number(tokens, 1)?;
            Some((super::path_command::PathCommand::MoveToAbs(x, y), 2))
        }
        'm' => {
            let x = take_number(tokens, 0)?;
            let y = take_number(tokens, 1)?;
            Some((super::path_command::PathCommand::MoveToRel(x, y), 2))
        }
        'L' => {
            let x = take_number(tokens, 0)?;
            let y = take_number(tokens, 1)?;
            Some((super::path_command::PathCommand::LineToAbs(x, y), 2))
        }
        'l' => {
            let x = take_number(tokens, 0)?;
            let y = take_number(tokens, 1)?;
            Some((super::path_command::PathCommand::LineToRel(x, y), 2))
        }
        'H' => {
            let x = take_number(tokens, 0)?;
            Some((super::path_command::PathCommand::HorizontalAbs(x), 1))
        }
        'h' => {
            let x = take_number(tokens, 0)?;
            Some((super::path_command::PathCommand::HorizontalRel(x), 1))
        }
        'V' => {
            let y = take_number(tokens, 0)?;
            Some((super::path_command::PathCommand::VerticalAbs(y), 1))
        }
        'v' => {
            let y = take_number(tokens, 0)?;
            Some((super::path_command::PathCommand::VerticalRel(y), 1))
        }
        'C' => {
            let x1 = take_number(tokens, 0)?;
            let y1 = take_number(tokens, 1)?;
            let x2 = take_number(tokens, 2)?;
            let y2 = take_number(tokens, 3)?;
            let x = take_number(tokens, 4)?;
            let y = take_number(tokens, 5)?;
            Some((
                super::path_command::PathCommand::CubicAbs(x1, y1, x2, y2, x, y),
                6,
            ))
        }
        'c' => {
            let x1 = take_number(tokens, 0)?;
            let y1 = take_number(tokens, 1)?;
            let x2 = take_number(tokens, 2)?;
            let y2 = take_number(tokens, 3)?;
            let x = take_number(tokens, 4)?;
            let y = take_number(tokens, 5)?;
            Some((
                super::path_command::PathCommand::CubicRel(x1, y1, x2, y2, x, y),
                6,
            ))
        }
        'S' => {
            let x2 = take_number(tokens, 0)?;
            let y2 = take_number(tokens, 1)?;
            let x = take_number(tokens, 2)?;
            let y = take_number(tokens, 3)?;
            Some((
                super::path_command::PathCommand::SmoothCubicAbs(x2, y2, x, y),
                4,
            ))
        }
        's' => {
            let x2 = take_number(tokens, 0)?;
            let y2 = take_number(tokens, 1)?;
            let x = take_number(tokens, 2)?;
            let y = take_number(tokens, 3)?;
            Some((
                super::path_command::PathCommand::SmoothCubicRel(x2, y2, x, y),
                4,
            ))
        }
        'Q' => {
            let x1 = take_number(tokens, 0)?;
            let y1 = take_number(tokens, 1)?;
            let x = take_number(tokens, 2)?;
            let y = take_number(tokens, 3)?;
            Some((
                super::path_command::PathCommand::QuadAbs(x1, y1, x, y),
                4,
            ))
        }
        'q' => {
            let x1 = take_number(tokens, 0)?;
            let y1 = take_number(tokens, 1)?;
            let x = take_number(tokens, 2)?;
            let y = take_number(tokens, 3)?;
            Some((
                super::path_command::PathCommand::QuadRel(x1, y1, x, y),
                4,
            ))
        }
        'T' => {
            let x = take_number(tokens, 0)?;
            let y = take_number(tokens, 1)?;
            Some((super::path_command::PathCommand::SmoothQuadAbs(x, y), 2))
        }
        't' => {
            let x = take_number(tokens, 0)?;
            let y = take_number(tokens, 1)?;
            Some((super::path_command::PathCommand::SmoothQuadRel(x, y), 2))
        }
        'A' => {
            let rx = take_number(tokens, 0)?;
            let ry = take_number(tokens, 1)?;
            let angle = take_number(tokens, 2)?;
            let large_arc = take_flag(tokens, 3)?;
            let sweep = take_flag(tokens, 4)?;
            let x = take_number(tokens, 5)?;
            let y = take_number(tokens, 6)?;
            Some((
                super::path_command::PathCommand::ArcAbs {
                    rx,
                    ry,
                    angle,
                    large_arc,
                    sweep,
                    x,
                    y,
                },
                7,
            ))
        }
        'a' => {
            let rx = take_number(tokens, 0)?;
            let ry = take_number(tokens, 1)?;
            let angle = take_number(tokens, 2)?;
            let large_arc = take_flag(tokens, 3)?;
            let sweep = take_flag(tokens, 4)?;
            let x = take_number(tokens, 5)?;
            let y = take_number(tokens, 6)?;
            Some((
                super::path_command::PathCommand::ArcRel {
                    rx,
                    ry,
                    angle,
                    large_arc,
                    sweep,
                    x,
                    y,
                },
                7,
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_move_line() {
        let cmds = parse_path_data("M 10 20 L 30 40");
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn parse_implicit_lineto_after_move() {
        // "M 0 0 10 10" should produce MoveTo + LineTo
        let cmds = parse_path_data("M 0 0 10 10");
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn parse_close() {
        let cmds = parse_path_data("M 0 0 L 10 0 L 10 10 Z");
        assert_eq!(cmds.len(), 4);
    }

    #[test]
    fn parse_cubic() {
        let cmds = parse_path_data("M 0 0 C 1 2 3 4 5 6");
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn parse_arc() {
        let cmds = parse_path_data("M 0 0 A 10 10 0 0 1 20 20");
        assert_eq!(cmds.len(), 2);
    }
}
