//! # Patch Maps
//!
//! This module provides two map structures for managing JSON patches:
//!
//! - [`OnePatchMap`]: Maintains a mapping from JSON paths to a **single** value with a priority, and
//!   the values that lost to it. A [`PriorityMap`].
//! - [`SeqPatchMap`]: Maintains a mapping from JSON paths to a **list** of values. A [`ListMap`].
//!
//! Neither is concurrent: a template has one writer at a time. See `priority_map`.
//!
//! These are ideal for use-cases where patches must be merged, prioritized, or combined from multiple threads.

use json_patch::{JsonPath, ValueWithPriority};

use super::{
    list_map::ListMap,
    priority_map::{PriorityEntry, PriorityMap},
};

/// A combined borrowed structure that holds both [`OnePatchMap`] and [`SeqPatchMap`].
///
/// This is useful when you want to manage both *single-value patches*
/// and *multi-value patches* in the same context.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HkxPatchMaps<'a> {
    /// Stores one value per path (highest priority wins).
    #[serde(bound(deserialize = "OnePatchMap<'a>: serde::Deserialize<'de>"))]
    pub one: OnePatchMap<'a>,
    /// Stores multiple values per path.
    #[serde(bound(deserialize = "SeqPatchMap<'a>: serde::Deserialize<'de>"))]
    pub seq: SeqPatchMap<'a>,
}

impl<'a> HkxPatchMaps<'a> {
    #[inline]
    pub const fn len(&self) -> usize {
        self.one.len() + self.seq.len()
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn merge(&mut self, other: Self) {
        self.one.merge(other.one);
        self.seq.merge(other.seq);
    }
}

/// One value per JSON path: the patch of the highest priority, and the ones that lost to it. See
/// [`PriorityMap`]: one writer at a time, which the map of templates around it already makes sure of.
#[derive(Debug, Clone, Default)]
pub struct OnePatchMap<'a> {
    map: PriorityMap<JsonPath<'a>, ValueWithPriority<'a>>,
}

impl<'a> OnePatchMap<'a> {
    /// Inserts a patch. Where the path is already, the higher priority is the one applied, and the
    /// other is kept as a loser: see [`Self::overridden`].
    ///
    /// A new object goes by its id alone (`/#aaaa$1`, its class in the value: see
    /// `nemesis_xml::patch`). So two mods adding one id as other classes (e.g. two versions of one
    /// mod) meet at one path: only the highest priority is the object.
    #[inline]
    pub fn insert(&mut self, key: JsonPath<'a>, new_value: ValueWithPriority<'a>) {
        self.map.insert(key, new_value);
    }

    /// Takes in every patch of `other`, as [`Self::insert`] does.
    #[inline]
    pub fn merge(&mut self, other: Self) {
        self.map.merge(other.map);
    }

    /// Puts patches at paths that are not in the map yet: what only adds, e.g. FNIS's own objects. See
    /// [`PriorityMap::extend_unique`].
    #[inline]
    pub fn extend_unique(&mut self, patches: impl IntoIterator<Item = (JsonPath<'a>, ValueWithPriority<'a>)>) {
        self.map.extend_unique(patches);
    }

    /// The patches that lost at their paths, put back as read from a file: see `PatchJson`.
    pub fn insert_losers(&mut self, losers: SeqPatchMap<'a>) {
        for (path, values) in losers.into_groups() {
            for value in values {
                self.map.insert(path.clone(), value);
            }
        }
    }

    /// The number of paths.
    #[inline]
    pub const fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if no patch is stored.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// What lost at each path: not applied, but a conflict check shows them. Written as
    /// `{ path: [patch, ..] }`, only the paths with any.
    #[inline]
    pub const fn overridden(&self) -> Overridden<'_, 'a> {
        Overridden(self)
    }

    /// The patch applied at each path, the map gone: in the order the paths came.
    #[inline]
    pub fn into_winners(self) -> impl Iterator<Item = (JsonPath<'a>, ValueWithPriority<'a>)> {
        self.map.into_winners()
    }

    /// Each path with the patch applied and those that lost, the map gone.
    #[inline]
    pub fn into_entries(self) -> impl Iterator<Item = PriorityEntry<JsonPath<'a>, ValueWithPriority<'a>>> {
        self.map.into_entries()
    }
}

/// See [`OnePatchMap::overridden`].
#[derive(Debug, Clone, Copy)]
pub struct Overridden<'m, 'a>(&'m OnePatchMap<'a>);

/// Every operation on each array, by its JSON path: all of them are applied, in priority order. See
/// [`ListMap`]: one writer at a time, as for [`OnePatchMap`].
#[derive(Debug, Clone, Default)]
pub struct SeqPatchMap<'a> {
    map: ListMap<JsonPath<'a>, ValueWithPriority<'a>>,
}

impl<'a> SeqPatchMap<'a> {
    /// Puts an operation on the array at `key`, after those there.
    #[inline]
    pub fn insert(&mut self, key: JsonPath<'a>, new_value: ValueWithPriority<'a>) {
        self.map.insert(key, new_value);
    }

    /// Takes in every operation of `other`, after those of `self`.
    #[inline]
    pub fn merge(&mut self, other: Self) {
        self.map.merge(other.map);
    }

    /// The number of arrays.
    #[inline]
    pub const fn len(&self) -> usize {
        self.map.len()
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Each array with its operations, in the order the arrays came. For what only reads them.
    #[inline]
    pub fn groups(&self) -> impl Iterator<Item = (&JsonPath<'a>, Vec<&ValueWithPriority<'a>>)> {
        self.map.groups()
    }

    /// Each array with its operations, the map gone: what is applied.
    #[inline]
    pub fn into_groups(self) -> impl Iterator<Item = (JsonPath<'a>, Vec<ValueWithPriority<'a>>)> {
        self.map.into_groups()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// serde implementations
//
// Why:
// We implement `Serialize` and `Deserialize` manually because the `JsonPath<'a>` key
// is a `Vec<Cow<'a, str>>`. In JSON, map keys must be strings, so we encode
// the path as a slash-separated `String` (e.g., `"#0001/hkbProjectData/[0:10]"`).
// Without this manual implementation, `serde` would try to encode the key
// as an array (`["#0001", "hkbProjectData", "[0:10]"]`), which is not
// valid for JSON object keys.

impl<'a> serde::Serialize for OnePatchMap<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(self.map.len()))?;
        for (path, winner) in self.map.winners() {
            map.serialize_entry(&path.join("/"), winner)?; // JsonPath -> "a/b/c"
        }
        map.end()
    }
}

impl serde::Serialize for Overridden<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let losers = self.0.map.losers();
        let mut map = serializer.serialize_map(Some(losers.len()))?;
        for (path, lost) in losers {
            map.serialize_entry(&path.join("/"), &lost)?;
        }
        map.end()
    }
}

impl<'de: 'a, 'a> serde::Deserialize<'de> for OnePatchMap<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<'a> {
            marker: std::marker::PhantomData<&'a ()>,
        }

        impl<'de: 'a, 'a> serde::de::Visitor<'de> for Visitor<'a> {
            type Value = OnePatchMap<'a>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("a map with slash-separated json paths as keys")
            }

            fn visit_map<M>(self, mut access: M) -> Result<OnePatchMap<'a>, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                let mut patches = OnePatchMap::default();
                while let Some((key, value)) = access.next_entry::<String, ValueWithPriority<'de>>()? {
                    let json_path = key.split('/').map(|s| std::borrow::Cow::Owned(s.to_string())).collect();
                    patches.insert(json_path, value);
                }
                Ok(patches)
            }
        }

        deserializer.deserialize_map(Visitor { marker: std::marker::PhantomData })
    }
}

impl<'a> serde::Serialize for SeqPatchMap<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(self.map.len()))?;
        for (path, operations) in self.map.groups() {
            map.serialize_entry(&path.join("/"), &operations)?; // JsonPath -> "a/b/c"
        }
        map.end()
    }
}

impl<'de: 'a, 'a> serde::Deserialize<'de> for SeqPatchMap<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<'a> {
            marker: std::marker::PhantomData<&'a ()>,
        }

        impl<'de: 'a, 'a> serde::de::Visitor<'de> for Visitor<'a> {
            type Value = SeqPatchMap<'a>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("a map with slash-separated JSON path keys and value arrays")
            }

            fn visit_map<M>(self, mut access: M) -> Result<SeqPatchMap<'a>, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                let mut map = SeqPatchMap::default();
                while let Some((key, values)) = access.next_entry::<String, Vec<ValueWithPriority<'de>>>()? {
                    let json_path: JsonPath<'a> =
                        key.split('/').map(|s| std::borrow::Cow::Owned(s.to_string())).collect();
                    for value in values {
                        map.insert(json_path.clone(), value);
                    }
                }
                Ok(map)
            }
        }

        deserializer.deserialize_map(Visitor { marker: std::marker::PhantomData })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(priority: usize) -> ValueWithPriority<'static> {
        ValueWithPriority {
            patch: json_patch::JsonPatch {
                action: json_patch::Action::Pure { op: json_patch::Op::Add },
                value: "test".into(),
            },
            priority,
        }
    }

    /// The winner's and the losers' priorities at `path`.
    fn priorities(map: OnePatchMap<'_>, path: &JsonPath<'_>) -> (usize, Vec<usize>) {
        let slot = map.into_entries().find(|entry| entry.key == *path).unwrap();
        (slot.winner.priority, slot.losers.iter().map(|loser| loser.priority).collect())
    }

    #[test]
    fn the_highest_priority_is_kept_and_the_losers_are_remembered() {
        let mut map = OnePatchMap::default();
        let path = json_patch::json_path!["#1", "C", "a"];
        map.insert(path.clone(), value(5));
        map.insert(path.clone(), value(9)); // takes over
        map.insert(path.clone(), value(1)); // too low

        assert_eq!(priorities(map.clone(), &path), (9, vec![5, 1]));

        let mut other = OnePatchMap::default();
        other.insert(path.clone(), value(20));
        map.merge(other);
        assert_eq!(priorities(map, &path), (20, vec![5, 1, 9]));
    }

    /// Two versions of a mod add `#aaaa$1` as other classes: one path, so one object.
    #[test]
    fn an_object_added_as_two_classes_is_one_of_them() {
        let class = |class: &'static str, priority| ValueWithPriority {
            patch: json_patch::JsonPatch {
                action: json_patch::Action::Pure { op: json_patch::Op::Add },
                value: class.into(), // in a patch, `{ class: { ..fields } }`
            },
            priority,
        };
        let mut map = OnePatchMap::default();
        let path = json_patch::json_path!["#aaaa$1"];
        map.insert(path.clone(), class("SomeClass", 9));
        map.insert(path.clone(), class("OtherSomeClass", 3));

        assert_eq!(map.len(), 1);
        assert_eq!(priorities(map, &path), (9, vec![3]), "a conflict a check sees");
    }

    #[test]
    fn should_not_treat_nested_paths_as_class_adds() {
        let mut map = OnePatchMap::default();

        let path1 = json_patch::json_path!["#5176", "ClassA", "field1",];
        let path2 = json_patch::json_path!["#5176", "ClassB", "field2",];

        map.insert(path1.clone(), value(10));
        map.insert(path2.clone(), value(20));

        assert_eq!(map.len(), 2);
        let mut paths: Vec<_> = map.into_winners().map(|(path, _)| path).collect();
        paths.sort();
        assert_eq!(paths, [path1, path2]);
    }

    /// What the old `DashMap` of `SeqPatchMap` did: a list a path, made and grown at each insert.
    type OldSeq = dashmap::DashMap<JsonPath<'static>, Vec<ValueWithPriority<'static>>, fast_hash::RandomState>;

    fn old_seq_insert(map: &OldSeq, path: JsonPath<'static>, value: ValueWithPriority<'static>) {
        match map.entry(path) {
            dashmap::Entry::Occupied(mut existing) => existing.get_mut().push(value),
            dashmap::Entry::Vacant(vacant) => {
                vacant.insert(vec![value]);
            }
        }
    }

    /// `PATCHES` patches of paths as a mod writes them: every `same_path_every`th is at the path of
    /// the one before it.
    fn bench_patches(same_path_every: usize) -> Vec<(JsonPath<'static>, ValueWithPriority<'static>)> {
        use std::borrow::Cow;

        (0..BENCH_PATCHES)
            .map(|at| {
                let object = if at % same_path_every == 0 && at > 0 { at - 1 } else { at };
                let path: JsonPath<'static> = vec![
                    Cow::Owned(format!("#{object:04}")),
                    Cow::Borrowed("hkbStateMachine"),
                    Cow::Borrowed("enable"),
                ];
                (path, value(at % 7))
            })
            .collect()
    }

    /// What a template gets, about: a million patches.
    const BENCH_PATCHES: usize = 1_000_000;

    /// `OnePatchMap` (a `PriorityMap`) against the `DashMap`s it replaced: one in ten patches a
    /// conflict. One writer, as each template has. Prints how long inserting and then taking the winners
    /// out took; run with `--release --ignored --nocapture`.
    #[test]
    #[ignore = "a benchmark: prints timings"]
    fn priority_map_against_dashmaps() {
        use std::time::Instant;

        let patches = bench_patches(10);

        // What `OnePatchMap::insert` was: the winners, and a second map of the losers.
        let old = {
            let patches = patches.clone();
            let start = Instant::now();
            let winners: dashmap::DashMap<JsonPath<'static>, ValueWithPriority<'static>, fast_hash::RandomState> =
                dashmap::DashMap::default();
            let losers = OldSeq::default();
            for (path, value) in patches {
                if let Some(mut existing) = winners.get_mut(&path) {
                    let loser = if value.priority > existing.priority {
                        std::mem::replace(&mut *existing, value)
                    } else {
                        value
                    };
                    drop(existing);
                    old_seq_insert(&losers, path, loser);
                    continue;
                }
                winners.insert(path, value);
            }
            let inserted = start.elapsed();
            let applied = winners.into_iter().count();
            (inserted, start.elapsed(), applied, losers.len())
        };

        let new = {
            let start = Instant::now();
            let mut map = OnePatchMap::default();
            for (path, value) in patches {
                map.insert(path, value);
            }
            let inserted = start.elapsed();
            let lost = map.map.losers().len();
            let applied = map.into_winners().count();
            (inserted, start.elapsed(), applied, lost)
        };

        println!("one: DashMap + DashMap of losers: insert {:?}, + take the winners {:?}", old.0, old.1);
        println!("one: PriorityMap:                 insert {:?}, + take the winners {:?}", new.0, new.1);
        assert_eq!((old.2, old.3), (new.2, new.3), "the same winners and paths with losers");
    }

    /// `SeqPatchMap` (a `ListMap`) against the `DashMap` it replaced: arrays with one to a few
    /// operations (every third is on the array of the one before). Prints how long inserting and then
    /// taking each array's list out took.
    #[test]
    #[ignore = "a benchmark: prints timings"]
    fn list_map_against_dashmap() {
        use std::time::Instant;

        let patches = bench_patches(3);

        let old = {
            let patches = patches.clone();
            let start = Instant::now();
            let map = OldSeq::default();
            for (path, value) in patches {
                old_seq_insert(&map, path, value);
            }
            let inserted = start.elapsed();
            let operations: usize = map.into_iter().map(|(_, values)| values.len()).sum();
            (inserted, start.elapsed(), operations)
        };

        let new = {
            let start = Instant::now();
            let mut map = SeqPatchMap::default();
            for (path, value) in patches {
                map.insert(path, value);
            }
            let inserted = start.elapsed();
            let operations: usize = map.into_groups().map(|(_, values)| values.len()).sum();
            (inserted, start.elapsed(), operations)
        };

        println!("seq: DashMap of lists: insert {:?}, + take the lists {:?}", old.0, old.1);
        println!("seq: ListMap:          insert {:?}, + take the lists {:?}", new.0, new.1);
        assert_eq!(old.2, new.2, "every operation is kept");
    }
}
