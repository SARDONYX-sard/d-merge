//! Marks (`ma`, `'a`, `` `a ``) and the jump list (`Ctrl+O`, `Ctrl+I`, `''`).
//!
//! Positions are char indices and are not moved by later edits (they are clamped when used).

/// Jumps kept at most (the oldest are dropped).
const MAX_JUMPS: usize = 100;

#[derive(Debug, Default)]
pub(super) struct Marks {
    /// `a`..`z`
    named: Vec<(char, usize)>,
    /// Positions jumped *from*, oldest first.
    jumps: Vec<usize>,
    /// Where `Ctrl+O` / `Ctrl+I` are in `jumps` (`jumps.len()` = after the newest).
    jump_pos: usize,
    /// Position before the latest jump (`''`).
    previous: Option<usize>,
}

impl Marks {
    pub(super) fn set(&mut self, name: char, at: usize) {
        match self.named.iter_mut().find(|(c, _)| *c == name) {
            Some((_, slot)) => *slot = at,
            None => self.named.push((name, at)),
        }
    }

    /// `'a`, or `''` / `` `` `` for the position before the latest jump.
    pub(super) fn get(&self, name: char) -> Option<usize> {
        match name {
            '\'' | '`' => self.previous,
            _ => self.named.iter().find(|(c, _)| *c == name).map(|(_, at)| *at),
        }
    }

    /// `:delm a`
    pub(super) fn delete(&mut self, names: &str) {
        self.named.retain(|(c, _)| !names.contains(*c));
    }

    /// Records `from` before a jump (e.g., `G`, `/foo`, `%`).
    pub(super) fn push_jump(&mut self, from: usize) {
        self.jumps.truncate(self.jump_pos);
        if self.jumps.last() != Some(&from) {
            self.jumps.push(from);
        }
        if self.jumps.len() > MAX_JUMPS {
            self.jumps.remove(0);
        }
        self.jump_pos = self.jumps.len();
        self.previous = Some(from);
    }

    /// `Ctrl+O` (`older`) / `Ctrl+I`: the position to go to, if any.
    pub(super) fn walk(&mut self, older: bool, current: usize) -> Option<usize> {
        if older {
            if self.jump_pos == 0 {
                return None;
            }
            // Leaving the newest end: remember where we are, so `Ctrl+I` can come back.
            if self.jump_pos == self.jumps.len() && self.jumps.last() != Some(&current) {
                self.jumps.push(current);
            }
            self.jump_pos -= 1;
        } else {
            if self.jump_pos + 1 >= self.jumps.len() {
                return None;
            }
            self.jump_pos += 1;
        }
        self.jumps.get(self.jump_pos).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jump_list_round_trip() {
        let mut marks = Marks::default();
        marks.push_jump(10); // e.g., `G` from 10
        marks.push_jump(50); // `gg` from 50
        assert_eq!(marks.walk(true, 0), Some(50));
        assert_eq!(marks.walk(true, 50), Some(10));
        assert_eq!(marks.walk(true, 10), None);
        assert_eq!(marks.walk(false, 10), Some(50));
        assert_eq!(marks.walk(false, 50), Some(0));
        assert_eq!(marks.get('\''), Some(50));
    }
}
