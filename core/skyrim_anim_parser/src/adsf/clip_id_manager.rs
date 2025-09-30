use bitvec::prelude::*;

/// # Why is this necessary?
/// Since `clip_id` is undecided at the mod patch stage, it is replaced with the Nemesis variable. If the variable exists, it needs to be replaced with an unused id.
///
/// In other words, it is used at the serialization stage of animationdatasinglefile.txt.
pub struct ClipIdManager {
    used_ids: BitVec,
    current: usize,
}

impl ClipIdManager {
    /// i16::MAX
    pub const MAX_ID: usize = 32767;

    pub fn new() -> Self {
        Self {
            used_ids: bitvec![0; Self::MAX_ID + 1], // 0..=32767
            current: Self::MAX_ID,
        }
    }

    /// register used id
    pub fn register(&mut self, id: usize) {
        if id <= Self::MAX_ID {
            self.used_ids.set(id, true);
        }
    }

    /// 32767 -> 0
    pub fn next_id(&mut self) -> Option<usize> {
        while self.current > 0 {
            if !self.used_ids[self.current] {
                self.used_ids.set(self.current, true);
                return Some(self.current);
            }
            self.current -= 1;
        }

        // zero check
        if !self.used_ids[0] {
            self.used_ids.set(0, true);
            return Some(0);
        }

        // used all
        None
    }

    /// reset for test
    pub fn reset(&mut self) {
        self.used_ids.fill(false);
        self.current = Self::MAX_ID;
    }
}

impl Default for ClipIdManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clip_id_decrementing() {
        let mut manager = ClipIdManager::new();

        // fist id
        assert_eq!(manager.next_id(), Some(32767));
        assert_eq!(manager.next_id(), Some(32766));

        // manual
        manager.register(32765);
        assert_eq!(manager.next_id(), Some(32764));

        // reset
        manager.reset();
        assert_eq!(manager.next_id(), Some(32767));
    }
}
