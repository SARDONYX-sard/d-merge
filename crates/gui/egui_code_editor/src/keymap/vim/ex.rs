//! Ex commands (`:` prompt).
//!
//! ```text
//! :w  :write  :wq  :x        save (the host decides what that means)
//! :<n>                       go to line n
//! :[range]s/pat/rep/[g]      substitute (plain text, smartcase; `g`: every match in a line)
//! :[range]sort[!] [u]        sort lines (`!`: reverse, `u`: drop duplicates)
//! :[range]d  :[range]y       delete / yank lines
//! :u  :undo  :red  :redo
//! :noh                       hide the search highlight
//! :reg  :registers           show the registers
//! :delm[arks] {a-z}          delete marks
//! ```
//! `range`: `%` (all lines), `'<,'>` (the last Visual selection), `n`, `n,m`, `.`, `$`
//! e.g., `:%s/old/new/g`, `:'<,'>sort`, `:3,5d`

use super::{Mode, VimState, buffer::Buffer, command::Operator, exec::Exec, prompt::LastSearch};
use crate::{EditorCommand, search::Matcher};

/// 0-based, inclusive line range.
type Lines = (usize, usize);

impl VimState {
    pub(super) fn run_ex(&mut self, line: &str, exec: &mut Exec) {
        let current = exec.buf.line_of(self.cursor);
        let last = exec.buf.line_count() - 1;
        let (range, command) = self.parse_range(line, &exec.buf);
        let command = command.trim();

        match command {
            "" => {
                // `:5` -> line 5
                if let Some((_, to)) = range {
                    self.cursor = exec.buf.first_non_blank(exec.buf.nth_line_start(to));
                    self.marks.push_jump(self.cursor);
                }
            }
            "w" | "write" | "wq" | "x" | "update" | "up" => {
                exec.output.command = Some(EditorCommand::Save);
            }
            "noh" | "nohl" | "nohlsearch" => self.hlsearch = false,
            "u" | "undo" => self.undo_redo(true, exec),
            "red" | "redo" => self.undo_redo(false, exec),
            "reg" | "registers" | "di" | "display" => self.message = Some(self.registers.list()),
            "d" | "delete" | "y" | "yank" => {
                let lines = range.unwrap_or((current, current));
                self.ex_delete_or_yank(lines, command.starts_with('d'), exec);
            }
            _ => self.run_ex_with_args(command, range, (current, last), exec),
        }
    }

    fn run_ex_with_args(
        &mut self,
        command: &str,
        range: Option<Lines>,
        (current, last): (usize, usize),
        exec: &mut Exec,
    ) {
        if let Some(args) = strip_command(command, &["substitute", "s"]) {
            self.substitute(range.unwrap_or((current, current)), args, exec);
        } else if let Some(args) = strip_command(command, &["sort", "sor"]) {
            self.sort(range.unwrap_or((0, last)), args, exec);
        } else if let Some(names) = strip_command(command, &["delmarks", "delm"]) {
            self.marks.delete(names);
        } else {
            self.message = Some(format!("Not an editor command: {command}"));
        }
    }

    /// Splits `'<,'>sort` into the range and `sort`.
    fn parse_range<'a>(&self, line: &'a str, buf: &Buffer) -> (Option<Lines>, &'a str) {
        let current = buf.line_of(self.cursor);
        let last = buf.line_count() - 1;
        if let Some(rest) = line.strip_prefix('%') {
            return (Some((0, last)), rest);
        }
        if let Some(rest) = line.strip_prefix("'<,'>") {
            let lines = self.last_visual.map(|(_, a, b)| (a.min(b), a.max(b)));
            return (lines.map(|(a, b)| (buf.line_of(a), buf.line_of(b))), rest);
        }
        let (from, rest) = parse_address(line, current, last);
        let Some(from) = from else { return (None, line) };
        match rest.strip_prefix(',') {
            Some(after_comma) => {
                let (to, rest) = parse_address(after_comma, current, last);
                let to = to.unwrap_or(from);
                (Some((from.min(to), from.max(to))), rest)
            }
            None => (Some((from, from)), rest),
        }
    }

    /// `:d` / `:y` on whole lines.
    fn ex_delete_or_yank(&mut self, (from, to): Lines, delete: bool, exec: &mut Exec) {
        let start = exec.buf.nth_line_start(from);
        let end_line_start = exec.buf.nth_line_start(to);
        self.cursor = start;
        let op = if delete { Operator::Delete } else { Operator::Yank };
        self.operate(op, start..end_line_start, true, exec);
    }

    /// `:s/pat/rep/[g]`
    fn substitute(&mut self, (from, to): Lines, args: &str, exec: &mut Exec) {
        let mut chars = args.chars();
        let Some(delimiter) = chars.next() else {
            self.message = Some("E486: usage :s/pattern/replacement/[g]".to_string());
            return;
        };
        let rest: String = chars.collect();
        let mut parts = rest.split(delimiter);
        let pattern = parts.next().unwrap_or_default().to_string();
        let replacement = parts.next().unwrap_or_default();
        let flags = parts.next().unwrap_or_default();
        let every = flags.contains('g');

        // An empty pattern uses the last search, like Vim.
        let pattern = if pattern.is_empty() {
            self.search.as_ref().map(|s| s.pattern.clone()).unwrap_or_default()
        } else {
            pattern
        };
        let Some(matcher) = Matcher::new(&pattern) else { return };

        let start = exec.buf.nth_line_start(from);
        let end = exec.buf.line_end(exec.buf.nth_line_start(to));
        let block = exec.buf.slice(start..end);

        let (mut count, mut changed_lines) = (0, 0);
        let new_lines: Vec<String> = block
            .split('\n')
            .map(|line| {
                let matches = matcher.find_all(line);
                let matches = if every { &matches[..] } else { &matches[..matches.len().min(1)] };
                if matches.is_empty() {
                    return line.to_string();
                }
                count += matches.len();
                changed_lines += 1;
                replace_ranges(line, matches, replacement)
            })
            .collect();

        if count == 0 {
            self.message = Some(format!("E486: Pattern not found: {pattern}"));
            return;
        }
        exec.edit(self.cursor, start..end, &new_lines.join("\n"));
        self.cursor = exec.buf.first_non_blank(exec.buf.nth_line_start(to));
        self.search = Some(LastSearch { pattern, forward: true, whole_word: false });
        self.message = Some(format!("{count} substitutions on {changed_lines} lines"));
    }

    /// `:sort`, `:sort!` (reverse), `:sort u` (unique).
    fn sort(&mut self, (from, to): Lines, args: &str, exec: &mut Exec) {
        let reverse = args.starts_with('!');
        let unique = args.contains('u');

        let start = exec.buf.nth_line_start(from);
        let end = exec.buf.line_end(exec.buf.nth_line_start(to));
        let block = exec.buf.slice(start..end);
        let mut lines: Vec<&str> = block.split('\n').collect();
        lines.sort_unstable();
        if unique {
            lines.dedup();
        }
        if reverse {
            lines.reverse();
        }
        exec.edit(self.cursor, start..end, &lines.join("\n"));
        self.cursor = start;
        if self.mode != Mode::Normal {
            self.mode = Mode::Normal;
        }
    }
}

/// `5`, `.`, `$` at the start of `s` -> (0-based line, rest).
fn parse_address(s: &str, current: usize, last: usize) -> (Option<usize>, &str) {
    if let Some(rest) = s.strip_prefix('.') {
        return (Some(current), rest);
    }
    if let Some(rest) = s.strip_prefix('$') {
        return (Some(last), rest);
    }
    let digits = s.chars().take_while(char::is_ascii_digit).count();
    match s[..digits].parse::<usize>() {
        Ok(n) => (Some(n.saturating_sub(1).min(last)), &s[digits..]),
        Err(_) => (None, s),
    }
}

/// `s/a/b/` with names `["substitute", "s"]` -> `Some("/a/b/")`. The name must end the command
/// or be followed by a non-letter (so `:sort` is not `:s` + `ort`).
fn strip_command<'a>(command: &'a str, names: &[&str]) -> Option<&'a str> {
    names.iter().find_map(|name| {
        let rest = command.strip_prefix(name)?;
        (!rest.starts_with(|c: char| c.is_ascii_alphabetic())).then(|| rest.trim_start_matches(' '))
    })
}

/// Replaces the char `ranges` of `line` with `with`.
fn replace_ranges(line: &str, ranges: &[std::ops::Range<usize>], with: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut at = 0;
    for range in ranges {
        out.extend(&chars[at..range.start]);
        out.push_str(with);
        at = range.end;
    }
    out.extend(&chars[at..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_and_commands() {
        assert_eq!(parse_address("3,5d", 0, 9), (Some(2), ",5d"));
        assert_eq!(parse_address("$", 0, 9), (Some(9), ""));
        assert_eq!(strip_command("s/a/b/g", &["substitute", "s"]), Some("/a/b/g"));
        assert_eq!(strip_command("sort u", &["substitute", "s"]), None);
        assert_eq!(strip_command("sort u", &["sort"]), Some("u"));
        assert_eq!(replace_ranges("a b a", &[0..1, 4..5], "xy"), "xy b xy");
    }
}
