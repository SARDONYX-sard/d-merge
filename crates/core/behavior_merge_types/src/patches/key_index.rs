//! The keys of a map and where each one is: what [`super::PriorityMap`] and [`super::ListMap`] share.
//!
//! ```text
//! keys:  [ (hash, key), .. ]     in the order they came; only appended to
//! table: hash table of positions in `keys` (u32), found by a key's hash
//! ```
//! A map keeps what it holds of each key in arrays of its own, at the key's position.
//!
//! - A key is hashed once. Its hash is kept beside it: when the table grows, nothing is hashed again
//!   (a key is a `JsonPath`, a list of strings).
//! - Positions are the order the keys came, so what comes out of a map is the same each run.
//! - Not concurrent, on purpose: see [`super::priority_map`].

use std::hash::{BuildHasher, Hash};

use hashbrown::{HashTable, hash_table::Entry};

/// A position in [`KeyIndex`]: a template has far fewer than 4 billion patches.
#[expect(clippy::cast_possible_truncation, reason = "a template has far fewer than 4 billion patches")]
pub(super) const fn position(at: usize) -> u32 {
    at as u32
}

/// See the module docs.
#[derive(Debug, Clone, Default)]
pub(super) struct KeyIndex<K, S = fast_hash::RandomState> {
    keys: Vec<(u64, K)>,
    /// Positions in `keys`, by the key's hash.
    table: HashTable<u32>,
    hasher: S,
}

impl<K: Hash + Eq, S: BuildHasher + Default> KeyIndex<K, S> {
    /// Room for `keys` keys without growing.
    pub(super) fn with_capacity(keys: usize) -> Self {
        Self { keys: Vec::with_capacity(keys), table: HashTable::with_capacity(keys), hasher: S::default() }
    }
}

impl<K: Hash + Eq, S: BuildHasher> KeyIndex<K, S> {
    /// The position of `key`, and whether it came just now (`true`) or was there (`false`).
    pub(super) fn find_or_insert(&mut self, key: K) -> (u32, bool) {
        let hash = self.hasher.hash_one(&key);
        let Self { keys, table, .. } = self;

        match table.entry(hash, |&at| keys[at as usize].1 == key, |&at| keys[at as usize].0) {
            Entry::Occupied(occupied) => (*occupied.get(), false),
            Entry::Vacant(vacant) => {
                let at = position(keys.len());
                vacant.insert(at);
                keys.push((hash, key));
                (at, true)
            }
        }
    }

    /// Puts a key that is not here yet, without comparing it to the others: checked in debug builds
    /// only. Returns its position.
    pub(super) fn insert_unique(&mut self, key: K) -> u32 {
        let hash = self.hasher.hash_one(&key);
        let Self { keys, table, .. } = self;
        debug_assert!(
            table.find(hash, |&at| keys[at as usize].1 == key).is_none(),
            "a key thought to be new is here already"
        );

        let at = position(keys.len());
        keys.push((hash, key));
        table.insert_unique(hash, at, |&at| keys[at as usize].0);
        at
    }

    /// Room for `more` keys without growing.
    pub(super) fn reserve(&mut self, more: usize) {
        let Self { keys, table, .. } = self;
        keys.reserve(more);
        table.reserve(more, |&at| keys[at as usize].0);
    }
}

impl<K, S> KeyIndex<K, S> {
    pub(super) const fn len(&self) -> usize {
        self.keys.len()
    }

    /// The keys, in the order they came.
    pub(super) fn keys(&self) -> impl Iterator<Item = &K> {
        self.keys.iter().map(|(_, key)| key)
    }

    /// The keys, in the order they came, the index gone.
    pub(super) fn into_keys(self) -> impl Iterator<Item = K> {
        self.keys.into_iter().map(|(_, key)| key)
    }
}

/// `values` (each with the position of its key) by key: one list a key, for `keys` keys, each made at
/// its size at once. In each list, the values are in the order they came.
pub(super) fn group_by_key<V>(keys: usize, values: Vec<(u32, V)>) -> Vec<Vec<V>> {
    let mut counts = vec![0_usize; keys];
    for (at, _) in &values {
        counts[*at as usize] += 1;
    }

    let mut groups: Vec<Vec<V>> = counts.into_iter().map(Vec::with_capacity).collect();
    for (at, value) in values {
        groups[at as usize].push(value);
    }
    groups
}

/// [`group_by_key`] of references, for what only reads: e.g. a map being written as JSON.
pub(super) fn group_refs_by_key<V>(keys: usize, values: &[(u32, V)]) -> Vec<Vec<&V>> {
    let mut counts = vec![0_usize; keys];
    for (at, _) in values {
        counts[*at as usize] += 1;
    }

    let mut groups: Vec<Vec<&V>> = counts.into_iter().map(Vec::with_capacity).collect();
    for (at, value) in values {
        groups[*at as usize].push(value);
    }
    groups
}
