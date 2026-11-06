//! [`StrArena`]: strings that are borrowed for as long as the arena lives.
//!
//! # Why
//! Generated text is often needed in many places of what borrows from one owner.
//! e.g. a class name `#Mod$1` is the key of a patch, its `__ptr`, and in the fields that refer to it.
//! With `String`, each of them is a copy; with this, a `&'a str`.

use std::sync::{
    OnceLock,
    atomic::{AtomicUsize, Ordering},
};

/// How many strings a chunk holds. A chunk is a heap block, so it costs little to have some more.
const CHUNK_LEN: usize = 64;

/// A string that was added stays where it is until the arena is dropped: a chunk is never moved or
/// grown, a new one is linked behind it. Therefore [`Self::keep`] takes `&self`.
///
/// It takes no lock, and may be used from many threads.
#[derive(Default)]
pub struct StrArena {
    len: AtomicUsize,
    first: Chunk,
}

struct Chunk {
    /// Boxed: a list of chunks inside of each other would be one large block.
    slots: Box<[OnceLock<String>; CHUNK_LEN]>,
    next: OnceLock<Box<Self>>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self { slots: Box::new(std::array::from_fn(|_| OnceLock::new())), next: OnceLock::new() }
    }
}

impl StrArena {
    /// Creates an empty arena. Its first chunk is allocated here.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves `text` in, and borrows it for as long as the arena.
    ///
    /// # Example
    /// ```
    /// let arena = stdx::arena::StrArena::new();
    /// let a = arena.keep(format!("#Mod${}", 1));
    /// let b = arena.keep(format!("#Mod${}", 2)); // `a` stays valid
    /// assert_eq!([a, b], ["#Mod$1", "#Mod$2"]);
    /// ```
    pub fn keep(&self, text: String) -> &str {
        let index = self.len.fetch_add(1, Ordering::Relaxed);
        let mut chunk = &self.first;
        for _ in 0..index / CHUNK_LEN {
            chunk = chunk.next.get_or_init(Box::default);
        }
        // Each index is given out once, so that this slot is empty.
        chunk.slots[index % CHUNK_LEN].get_or_init(|| text)
    }

    /// The number of strings that were added.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed)
    }

    /// Whether nothing was added.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl std::fmt::Debug for StrArena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StrArena").field("len", &self.len()).finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_was_kept_stays_past_the_end_of_a_chunk() {
        let arena = StrArena::new();
        assert!(arena.is_empty());
        let count = CHUNK_LEN * 3 + 1;
        let kept: Vec<&str> = (0..count).map(|i| arena.keep(i.to_string())).collect();
        assert_eq!(arena.len(), count);
        for (i, text) in kept.iter().enumerate() {
            assert_eq!(*text, i.to_string());
        }
    }

    #[test]
    fn threads_get_their_own_slots() {
        let arena = StrArena::new();
        let per_thread = CHUNK_LEN * 2;
        std::thread::scope(|scope| {
            let threads: Vec<_> = (0..8)
                .map(|thread| {
                    let arena = &arena;
                    scope.spawn(move || {
                        (0..per_thread).map(|i| (arena.keep(format!("{thread}/{i}")), thread, i)).collect::<Vec<_>>()
                    })
                })
                .collect();
            for thread in threads {
                for (text, thread, i) in thread.join().unwrap() {
                    assert_eq!(text, format!("{thread}/{i}"));
                }
            }
        });
        assert_eq!(arena.len(), per_thread * 8);
    }
}
