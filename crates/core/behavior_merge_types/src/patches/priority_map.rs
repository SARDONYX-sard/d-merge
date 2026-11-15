//! A map that keeps, for each key, the value of the highest priority and the values that lost to it.
//!
//! Made for the patches of one template: one path from several mods is a conflict, which the highest
//! priority wins. The losers are kept too: they are not applied, but a conflict check shows them.
//!
//! ```text
//! insert(#0106/../enable, true,  priority 3)  -> #0106/../enable: true (3)
//! insert(#0106/../enable, false, priority 7)  -> #0106/../enable: false (7)   lost there: true (3)
//! insert(#0106/../enable, 1,     priority 5)  -> #0106/../enable: false (7)   lost there: true (3), 1 (5)
//! ```
//!
//! # Not concurrent, on purpose
//!
//! It takes `&mut self` and has no lock. A template's patches always have one writer at a time:
//! `behavior_merge` gives each template to one thread, and `fnis_patch_gen` writes through the entry
//! of the outer map of templates, which is that template's lock already. A lock here would only be
//! taken again for nothing.
//!
//! # Layout
//!
//! ```text
//! keys:    the keys, and where each one is: see `KeyIndex`
//! winners: [ value, .. ]               at the key's position
//! losers:  [ (position, value), .. ]   every loser of every key, in the order they lost
//! ```
//! - A key holds no list of its own: there is one list of losers for the whole map, which takes no
//!   memory until a first conflict. Most keys have none.
//! - The winners are one array in the order the keys came, so what comes out of the map is the same
//!   each run, and applying them walks memory in a row.

use std::hash::{BuildHasher, Hash};

use super::key_index::{KeyIndex, group_by_key, group_refs_by_key};

/// A value that knows its priority: the higher one wins a key. On equal priorities, the one inserted
/// first stays the winner.
pub trait Prioritized {
    fn priority(&self) -> usize;
}

impl Prioritized for json_patch::ValueWithPriority<'_> {
    #[inline]
    fn priority(&self) -> usize {
        self.priority
    }
}

/// One key with everything at it: what [`PriorityMap::into_entries`] gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriorityEntry<K, V> {
    pub key: K,
    pub winner: V,
    /// In the order they lost.
    pub losers: Vec<V>,
}

/// See the module docs.
#[derive(Debug, Clone, Default)]
pub struct PriorityMap<K, V, S = fast_hash::RandomState> {
    keys: KeyIndex<K, S>,
    /// At each key's position.
    winners: Vec<V>,
    /// `(key's position, value)`: everything that lost, in the order it lost.
    losers: Vec<(u32, V)>,
}

impl<K: Hash + Eq, V: Prioritized, S: BuildHasher + Default> PriorityMap<K, V, S> {
    /// Room for `keys` keys without growing.
    pub fn with_capacity(keys: usize) -> Self {
        Self { keys: KeyIndex::with_capacity(keys), winners: Vec::with_capacity(keys), losers: Vec::new() }
    }
}

impl<K: Hash + Eq, V: Prioritized, S: BuildHasher> PriorityMap<K, V, S> {
    /// Puts `value` at `key`. Where the key is already, the higher priority is the winner, and the other
    /// is one of its losers.
    pub fn insert(&mut self, key: K, value: V) {
        self.insert_at(key, value);
    }

    /// [`Self::insert`], returning the key's position.
    fn insert_at(&mut self, key: K, value: V) -> u32 {
        let (at, is_new) = self.keys.find_or_insert(key);
        if is_new {
            self.winners.push(value);
        } else {
            self.compete(at, value);
        }
        at
    }

    /// `value` against the winner of the key at `at`: the higher priority stays, the other loses.
    fn compete(&mut self, at: u32, value: V) {
        let winner = &mut self.winners[at as usize];
        let loser = if value.priority() > winner.priority() { std::mem::replace(winner, value) } else { value };
        self.losers.push((at, loser));
    }

    /// Puts keys that are not in the map yet: what adds only, e.g. FNIS's own objects. No key is compared,
    /// so a key twice is two keys: checked in debug builds only.
    pub fn extend_unique(&mut self, pairs: impl IntoIterator<Item = (K, V)>) {
        let pairs = pairs.into_iter();
        self.reserve(pairs.size_hint().0);

        for (key, value) in pairs {
            self.keys.insert_unique(key);
            self.winners.push(value);
        }
    }

    /// Takes in every key and value of `other`, as [`Self::insert`] does.
    ///
    /// Each key of `other` is hashed again: a map's hasher has a seed of its own, so a hash `other` kept
    /// means nothing here. A merge is of whole maps of templates, which is rare.
    pub fn merge(&mut self, other: Self) {
        self.reserve(other.winners.len());

        // Where each key of `other` is in `self` now: its losers go there, without their key.
        let moved_to: Vec<u32> =
            other.keys.into_keys().zip(other.winners).map(|(key, winner)| self.insert_at(key, winner)).collect();
        for (at, loser) in other.losers {
            self.compete(moved_to[at as usize], loser);
        }
    }

    /// Room for `more` keys without growing.
    pub fn reserve(&mut self, more: usize) {
        self.keys.reserve(more);
        self.winners.reserve(more);
    }
}

impl<K, V, S> PriorityMap<K, V, S> {
    /// How many keys.
    pub const fn len(&self) -> usize {
        self.keys.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.keys.len() == 0
    }

    /// Each key and its winner, in the order the keys came.
    pub fn winners(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys.keys().zip(&self.winners)
    }

    /// Each key that has losers, with them: in the order the keys came, the losers in the order they
    /// lost. Only for what reads the losers (a conflict check): it groups them first.
    pub fn losers(&self) -> Vec<(&K, Vec<&V>)> {
        let groups = group_refs_by_key(self.keys.len(), &self.losers);
        self.keys.keys().zip(groups).filter(|(_, lost)| !lost.is_empty()).collect()
    }

    /// Each key and its winner, the map gone: what is applied. The losers are dropped unread.
    pub fn into_winners(self) -> impl Iterator<Item = (K, V)> {
        self.keys.into_keys().zip(self.winners)
    }

    /// Each key with its winner and losers, the map gone.
    pub fn into_entries(self) -> impl Iterator<Item = PriorityEntry<K, V>> {
        let losers = group_by_key(self.keys.len(), self.losers);
        self.keys.into_keys().zip(self.winners).zip(losers).map(|((key, winner), losers)| PriorityEntry {
            key,
            winner,
            losers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Prioritized for (usize, &'static str) {
        fn priority(&self) -> usize {
            self.0
        }
    }

    type Map = PriorityMap<&'static str, (usize, &'static str)>;

    fn entry(map: Map, key: &str) -> PriorityEntry<&'static str, (usize, &'static str)> {
        map.into_entries().find(|entry| entry.key == key).unwrap()
    }

    /// The module docs' example.
    #[test]
    fn the_highest_priority_wins_and_the_others_lose_to_it() {
        let mut map = Map::default();
        map.insert("enable", (3, "true"));
        map.insert("enable", (7, "false"));
        map.insert("enable", (5, "1"));
        map.insert("other", (1, "x"));

        assert_eq!(map.len(), 2);
        assert_eq!(map.losers(), [(&"enable", vec![&(3, "true"), &(5, "1")])]);
        let enable = entry(map, "enable");
        assert_eq!(enable.winner, (7, "false"));
        assert_eq!(enable.losers, [(3, "true"), (5, "1")]);
    }

    #[test]
    fn on_equal_priorities_the_first_stays() {
        let mut map = Map::default();
        map.insert("a", (3, "first"));
        map.insert("a", (3, "second"));
        let a = entry(map, "a");
        assert_eq!((a.winner, a.losers), ((3, "first"), vec![(3, "second")]));
    }

    /// The order the keys came: the same each run.
    #[test]
    fn keys_come_out_in_the_order_they_came() {
        let mut map = Map::default();
        for key in ["c", "a", "b", "a"] {
            map.insert(key, (1, "v"));
        }
        let keys: Vec<_> = map.winners().map(|(key, _)| *key).collect();
        assert_eq!(keys, ["c", "a", "b"]);
    }

    /// A merge decides each key again: a loser of `other` can win in `self`.
    #[test]
    fn a_merge_is_as_if_all_was_inserted_into_one() {
        let (mut map, mut other) = (Map::default(), Map::default());
        map.insert("a", (5, "self"));
        other.insert("a", (9, "other high"));
        other.insert("a", (7, "other low"));
        other.insert("b", (1, "only other"));

        map.merge(other);
        let a = entry(map.clone(), "a");
        assert_eq!(a.winner, (9, "other high"));
        assert_eq!(a.losers, [(5, "self"), (7, "other low")]);
        assert_eq!(entry(map, "b").winner, (1, "only other"));
    }

    #[test]
    fn unique_keys_are_put_as_they_are() {
        let mut map = Map::default();
        map.insert("a", (1, "a"));
        map.extend_unique([("b", (1, "b")), ("c", (1, "c"))]);
        map.insert("b", (9, "b again")); // found, though it came without being compared

        assert_eq!(map.len(), 3);
        assert_eq!(entry(map, "b").winner, (9, "b again"));
    }

    /// Growing the index uses the hashes kept: every key is still found after many.
    #[test]
    fn keys_are_found_after_the_index_grows() {
        let mut map: PriorityMap<usize, (usize, &'static str)> = PriorityMap::default();
        for at in 0..10_000 {
            map.insert(at % 1000, (at, "v"));
        }

        assert_eq!(map.len(), 1000);
        let entries: Vec<_> = map.into_entries().collect();
        assert!(entries.iter().all(|entry| entry.losers.len() == 9));
        // The winner of key `k` is the last of `k`, `k + 1000`, .. `k + 9000`.
        assert!(entries.iter().all(|entry| entry.winner.0 == entry.key + 9000));
    }
}
