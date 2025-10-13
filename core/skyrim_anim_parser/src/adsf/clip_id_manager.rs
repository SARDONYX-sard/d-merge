//! Clip ID manager
//!
//! # Why is this necessary?
//! Since `clip_id` is undecided at the mod patch stage, it is replaced with the Nemesis variable. If the variable exists, it needs to be replaced with an unused id.
//!
//! In other words, it is used at the serialization stage of animationdatasinglefile.txt.
use bitvec::prelude::*;

/// Manages unique clip IDs for animation serialization.
///
/// This manager starts allocating IDs *after* the known maximum used ID.
/// Each call to `next_id()` returns the next unused ID, incrementing
/// until it reaches `i16::MAX`. After that, it returns `None`.
pub struct ClipIdManager {
    used_ids: BitVec,
    current: usize,
}

impl ClipIdManager {
    /// The maximum valid clip ID (i16::MAX).
    const MAX_ID: usize = i16::MAX as usize;

    /// Creates a new manager, starting from an existing maximum ID. (e.g., 1655).
    ///
    /// The next available ID will start from `start_from + 1`.
    fn new(start_from: usize) -> Self {
        Self {
            used_ids: bitvec![0; Self::MAX_ID + 1],
            current: start_from,
        }
    }

    /// Creates a new manager using the vanilla Skyrim SE baseline.
    ///
    /// This starts from `1655`, which is the **last used clip ID**
    /// found in the vanilla *Skyrim Special Edition* file
    /// `animationdatasinglefile.txt`.
    ///
    /// The next available ID will therefore be `1656`.
    pub fn new_vanilla() -> Self {
        Self::new(1655)
    }

    /// Registers an existing used ID.
    ///
    /// * `id` - The clip ID to mark as used.
    pub fn register(&mut self, id: usize) {
        if id <= Self::MAX_ID {
            self.used_ids.set(id, true);
        }
    }

    /// Returns the next available unused ID, incrementing upward.
    ///
    /// # Returns
    /// * `Some(id)` - The next unused ID.
    /// * `None` - If all possible IDs are used or the limit (`i16::MAX`) is reached.
    pub fn next_id(&mut self) -> Option<usize> {
        while self.current < Self::MAX_ID {
            self.current += 1;
            if !self.used_ids[self.current] {
                self.used_ids.set(self.current, true);
                return Some(self.current);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clip_id_incrementing() {
        let mut manager = ClipIdManager::new(1655);

        // Normal increment
        assert_eq!(manager.next_id(), Some(1656));
        assert_eq!(manager.next_id(), Some(1657));

        // Skip registered IDs
        manager.register(1658);
        assert_eq!(manager.next_id(), Some(1659));

        // Stop at MAX
        manager.current = ClipIdManager::MAX_ID - 1;
        assert_eq!(manager.next_id(), Some(ClipIdManager::MAX_ID));
        assert_eq!(manager.next_id(), None);
    }
}
