//! # Patch Maps
//!
//! This module provides two map structures for managing JSON patches:
//!
//! - [`OnePatchMap`]: Maintains a mapping from JSON paths to a **single** value with a priority.
//! - [`SeqPatchMap`]: Maintains a mapping from JSON paths to a **list** of values, supporting parallel insertion and extension.
//!
//! Both maps utilize `DashMap` for concurrent access and integrate with the `json_patch` crate.
//!
//! These are ideal for use-cases where patches must be merged, prioritized, or combined from multiple threads.

use dashmap::DashMap;
use json_patch::{JsonPath, ValueWithPriority};
use rayon::prelude::*;

/// A combined borrowed structure that holds both [`OnePatchMap`] and [`SeqPatchMap`].
///
/// This is useful when you want to manage both *single-value patches*
/// and *multi-value patches* in the same context.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HkxPatchMaps<'a> {
    /// Stores one value per path (highest priority wins).
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "OnePatchMap<'a>: serde::Deserialize<'de>"))
    )]
    pub one: OnePatchMap<'a>,
    /// Stores multiple values per path.
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "SeqPatchMap<'a>: serde::Deserialize<'de>"))
    )]
    pub seq: SeqPatchMap<'a>,
}

impl<'a> HkxPatchMaps<'a> {
    #[inline]
    pub fn len(&self) -> usize {
        self.one.0.len() + self.seq.0.len()
    }

    #[inline]
    pub(crate) fn merge(&self, other: Self) {
        self.one.merge(other.one);
        self.seq.merge(other.seq);
    }
}

/// A map that stores a **single** value for each JSON path,
/// ensuring that only the value with the highest priority is kept.
#[derive(Debug, Clone, Default)]
pub struct OnePatchMap<'a>(pub DashMap<JsonPath<'a>, ValueWithPriority<'a>>);

impl<'a> OnePatchMap<'a> {
    /// Constructs a new, empty [`OnePatchMap`].
    #[inline]
    #[allow(unused)]
    pub fn new() -> Self {
        Self(DashMap::new())
    }

    /// Inserts a value for the given JSON path.
    ///
    /// If an existing value is present, its priority is compared with
    /// the new value. The value with the **highest** priority is kept.
    ///
    /// # Parameters
    ///
    /// - `key`: The JSON path associated with the value.
    /// - `new_value`: The value with an associated priority.
    pub fn insert(&self, key: JsonPath<'a>, new_value: ValueWithPriority<'a>) {
        if let Some(mut existing) = self.0.get_mut(&key) {
            let new_priority = new_value.priority;
            let existing_priority = existing.priority;

            if new_priority > existing_priority {
                tracing::info!(
                    "Conflict Path {key:?}: priority {new_priority} ➔ {existing_priority} (overwritten)",
                );
                *existing = new_value;
            }
        } else {
            self.0.insert(key, new_value);
        }
    }

    /// Merges another `OnePatchMap` into this one by comparing priorities and keeping the highest.
    pub(crate) fn merge(&self, other: Self) {
        for (path, new_val) in other.0 {
            match self.0.entry(path) {
                dashmap::Entry::Occupied(mut occ) => {
                    let existing = occ.get_mut();
                    if new_val.priority > existing.priority {
                        *existing = new_val;
                    }
                }
                dashmap::Entry::Vacant(v) => {
                    v.insert(new_val);
                }
            }
        }
    }
}

/// A map that stores **multiple** values per JSON path,
/// allowing parallel insertion and extension.
#[derive(Debug, Clone, Default)]
pub struct SeqPatchMap<'a>(pub DashMap<JsonPath<'a>, Vec<ValueWithPriority<'a>>>);

impl<'a> SeqPatchMap<'a> {
    /// Constructs a new, empty [`SeqPatchMap`].
    #[inline]
    #[allow(unused)]
    pub fn new() -> Self {
        Self(DashMap::new())
    }

    /// Inserts a value for the given JSON path.
    ///
    /// - If the path already has an entry, the new value is pushed to the list.
    /// - Otherwise, a new list is created with the value.
    pub fn insert(&self, key: JsonPath<'a>, new_value: ValueWithPriority<'a>) {
        match self.0.entry(key) {
            dashmap::Entry::Occupied(mut existing) => {
                existing.get_mut().push(new_value);
            }
            dashmap::Entry::Vacant(v) => {
                v.insert(vec![new_value]);
            }
        }
    }

    /// Extends the list of values for the given JSON path.
    pub fn merge(&self, other: Self) {
        for (path, other_vals) in other.0 {
            match self.0.entry(path) {
                dashmap::Entry::Occupied(mut occ) => {
                    occ.get_mut().par_extend(other_vals);
                }
                dashmap::Entry::Vacant(v) => {
                    v.insert(other_vals);
                }
            }
        }
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

#[cfg(feature = "serde")]
impl<'a> serde::Serialize for OnePatchMap<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for item in self.0.iter() {
            let joined = item.key().join("/"); // JsonPath -> "a/b/c"
            map.serialize_entry(&joined, item.value())?;
        }
        map.end()
    }
}

#[cfg(feature = "serde")]
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
                let map = DashMap::new();
                while let Some((key, value)) =
                    access.next_entry::<String, ValueWithPriority<'de>>()?
                {
                    let json_path = key
                        .split('/')
                        .map(|s| std::borrow::Cow::Owned(s.to_string()))
                        .collect();
                    map.insert(json_path, value);
                }
                Ok(OnePatchMap(map))
            }
        }

        deserializer.deserialize_map(Visitor {
            marker: std::marker::PhantomData,
        })
    }
}

#[cfg(feature = "serde")]
impl<'a> serde::Serialize for SeqPatchMap<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for item in self.0.iter() {
            let joined = item.key().join("/"); // "a/b/c"
            map.serialize_entry(&joined, item.value())?;
        }
        map.end()
    }
}

#[cfg(feature = "serde")]
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
                let map = DashMap::new();

                while let Some((key, value)) =
                    access.next_entry::<String, Vec<ValueWithPriority<'de>>>()?
                {
                    let json_path = key
                        .split('/')
                        .map(|s| std::borrow::Cow::Owned(s.to_string()))
                        .collect();
                    map.insert(json_path, value);
                }

                Ok(SeqPatchMap(map))
            }
        }

        deserializer.deserialize_map(Visitor {
            marker: std::marker::PhantomData,
        })
    }
}
