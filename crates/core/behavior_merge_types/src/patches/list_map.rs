//! A map that keeps, for each key, every value put at it: the operations of every mod on one array.
//!
//! ```text
//! insert(#0106/../modifiers, push [#a$1], priority 3)
//! insert(#0106/../modifiers, replace [1..2], priority 7)
//!   -> #0106/../modifiers: [push [#a$1] (3), replace [1..2] (7)]   all of them applied, by priority
//! ```
//!
//! # Layout
//!
//! ```text
//! keys:   the keys, and where each one is: see `KeyIndex`
//! values: [ (position, value), .. ]   every value of every key, in the order they came
//! ```
//! - A key holds no list while it is filled: no list grows a value at a time, and a key with one value
//!   costs no list at all until the end.
//! - [`ListMap::into_groups`] makes each key's list once, at its size, as what applies them wants one
//!   list a key (`json_patch::apply_seq_by_priority`).
//! - Not concurrent, on purpose: see [`super::priority_map`].

use std::hash::{BuildHasher, Hash};

use super::key_index::{KeyIndex, group_by_key, group_refs_by_key};

/// See the module docs.
#[derive(Debug, Clone, Default)]
pub struct ListMap<K, V, S = fast_hash::RandomState> {
    keys: KeyIndex<K, S>,
    /// `(key's position, value)`, in the order they came.
    values: Vec<(u32, V)>,
}

impl<K: Hash + Eq, V, S: BuildHasher> ListMap<K, V, S> {
    /// Puts `value` at `key`, after those there.
    pub fn insert(&mut self, key: K, value: V) {
        let (at, _) = self.keys.find_or_insert(key);
        self.values.push((at, value));
    }

    /// Takes in every key and value of `other`, after those of `self`.
    ///
    /// Each key of `other` is hashed again: a map's hasher has a seed of its own, so a hash `other` kept
    /// means nothing here.
    pub fn merge(&mut self, other: Self) {
        self.keys.reserve(other.keys.len());
        self.values.reserve(other.values.len());

        // Where each key of `other` is in `self` now.
        let moved_to: Vec<u32> = other.keys.into_keys().map(|key| self.keys.find_or_insert(key).0).collect();
        self.values.extend(other.values.into_iter().map(|(at, value)| (moved_to[at as usize], value)));
    }
}

impl<K, V, S> ListMap<K, V, S> {
    /// How many keys.
    pub const fn len(&self) -> usize {
        self.keys.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.keys.len() == 0
    }

    /// Each key with its values, in the order the keys came, the values in the order they came. Only
    /// for what reads them (writing the map as JSON, a test): it groups them first.
    pub fn groups(&self) -> impl Iterator<Item = (&K, Vec<&V>)> {
        let groups = group_refs_by_key(self.keys.len(), &self.values);
        self.keys.keys().zip(groups)
    }

    /// Each key with its values, the map gone: each list made once, at its size.
    pub fn into_groups(self) -> impl Iterator<Item = (K, Vec<V>)> {
        let groups = group_by_key(self.keys.len(), self.values);
        self.keys.into_keys().zip(groups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Map = ListMap<&'static str, u32>;

    #[test]
    fn every_value_is_kept_in_the_order_it_came() {
        let mut map = Map::default();
        map.insert("b", 1);
        map.insert("a", 2);
        map.insert("b", 3);

        assert_eq!(map.len(), 2);
        let groups: Vec<_> = map.into_groups().collect();
        assert_eq!(groups, [("b", vec![1, 3]), ("a", vec![2])]);
    }

    #[test]
    fn each_list_is_made_at_its_size() {
        let mut map = Map::default();
        for value in 0..5 {
            map.insert("a", value);
        }
        let (_, values) = map.into_groups().next().unwrap();
        assert_eq!(values.capacity(), 5);
    }

    #[test]
    fn a_merge_puts_the_others_after() {
        let (mut map, mut other) = (Map::default(), Map::default());
        map.insert("a", 1);
        other.insert("b", 2);
        other.insert("a", 3);

        map.merge(other);
        let groups: Vec<_> =
            map.groups().map(|(key, values)| (*key, values.into_iter().copied().collect::<Vec<_>>())).collect();
        assert_eq!(groups, [("a", vec![1, 3]), ("b", vec![2])]);
    }
}
