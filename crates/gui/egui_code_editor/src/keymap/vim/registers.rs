//! Registers (`"ayy`, `"ap`, `:reg`).
//!
//! - `""` (unnamed): every yank / delete; also copied to the system clipboard
//! - `"0`: the last yank
//! - `"a`..`"z`: named; `"A`..`"Z` append to them
//! - `"+` / `"*`: the system clipboard (only writing: egui cannot read it on demand,
//!   so pasting from them uses the unnamed register)
//! - `"_`: black hole (discards)

/// Text in a register.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Register {
    pub(super) text: String,
    /// `true` for whole lines (`yy`, `dd`): pasted on its own line.
    pub(super) linewise: bool,
    /// A Visual block (`Ctrl+V` `y`): pasted as a block (one line per line).
    pub(super) block: bool,
}

#[derive(Debug, Default)]
pub(super) struct Registers {
    unnamed: Register,
    last_yank: Register,
    /// `a`..`z`, at most 26 entries.
    named: Vec<(char, Register)>,
}

impl Registers {
    /// Stores a yanked (`is_yank`) or deleted text into register `name` (`None`: unnamed only).
    pub(super) fn store(
        &mut self,
        name: Option<char>,
        register: Register,
        is_yank: bool,
        ctx: &egui::Context,
    ) {
        match name {
            Some('_') => return,
            Some(c) if c.is_ascii_lowercase() => self.set_named(c, register.clone()),
            Some(c) if c.is_ascii_uppercase() => {
                let lower = c.to_ascii_lowercase();
                let mut appended = self.named(lower).cloned().unwrap_or_default();
                if appended.linewise || register.linewise {
                    if !appended.text.is_empty() {
                        appended.text.push('\n');
                    }
                    appended.linewise = true;
                }
                appended.text.push_str(&register.text);
                self.set_named(lower, appended);
            }
            _ => {}
        }
        ctx.copy_text(register.text.clone());
        if is_yank {
            self.last_yank = register.clone();
        }
        self.unnamed = register;
    }

    /// The register to paste from.
    pub(super) fn get(&self, name: Option<char>) -> &Register {
        match name {
            Some('0') => &self.last_yank,
            Some(c) if c.is_ascii_alphabetic() => {
                self.named(c.to_ascii_lowercase()).unwrap_or(&self.unnamed)
            }
            _ => &self.unnamed,
        }
    }

    /// `:reg` listing (one line each), e.g. `"a  foo`.
    pub(super) fn list(&self) -> String {
        let show = |name: char, register: &Register| {
            let text = register.text.replace('\n', "^J");
            let text: String = text.chars().take(40).collect();
            format!("\"{name}  {text}")
        };
        let mut lines = vec![show('"', &self.unnamed), show('0', &self.last_yank)];
        let mut named: Vec<_> = self.named.iter().collect();
        named.sort_by_key(|(c, _)| *c);
        lines.extend(named.into_iter().map(|(c, register)| show(*c, register)));
        lines.join("   ")
    }

    fn named(&self, name: char) -> Option<&Register> {
        self.named.iter().find(|(c, _)| *c == name).map(|(_, register)| register)
    }

    fn set_named(&mut self, name: char, register: Register) {
        match self.named.iter_mut().find(|(c, _)| *c == name) {
            Some((_, slot)) => *slot = register,
            None => self.named.push((name, register)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg(text: &str, linewise: bool) -> Register {
        Register { text: text.to_string(), linewise, block: false }
    }

    #[test]
    fn named_append_and_black_hole() {
        let ctx = egui::Context::default();
        let mut registers = Registers::default();
        registers.store(Some('a'), reg("x", false), true, &ctx);
        registers.store(Some('A'), reg("y", false), true, &ctx);
        assert_eq!(registers.get(Some('a')), &reg("xy", false));
        assert_eq!(registers.get(None), &reg("y", false));

        registers.store(Some('_'), reg("gone", false), false, &ctx);
        assert_eq!(registers.get(None), &reg("y", false));
        assert_eq!(registers.get(Some('0')), &reg("y", false));
    }
}
