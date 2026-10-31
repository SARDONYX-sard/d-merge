//! Workspace-wide hash collection aliases.
//!
//! All crates in this workspace use these types instead of using `std` or a specific hasher crate directly,
//! so the hasher can be replaced by changing only [`RandomState`].
//!
//! Direct use of the underlying types is denied by `disallowed-types` in `.clippy.toml`.
#![allow(clippy::disallowed_types)] // This is the only place allowed to use the underlying types.

/// The [`core::hash::BuildHasher`] used by all hash collections in this workspace.
///
/// NOTE: Not suitable for persisted hashes(e.g. saved fingerprints), because the seed may be random.
pub type RandomState = rapidhash::fast::RandomState;

/// [`std::collections::HashMap`] with [`RandomState`].
///
/// # Note
/// Use `HashMap::default()` instead of `HashMap::new()`.
pub type HashMap<K, V, S = RandomState> = std::collections::HashMap<K, V, S>;

/// [`std::collections::HashSet`] with [`RandomState`].
///
/// # Note
/// Use `HashSet::default()` instead of `HashSet::new()`.
pub type HashSet<T, S = RandomState> = std::collections::HashSet<T, S>;

pub use std::collections::{hash_map, hash_set};

/// [`indexmap::IndexMap`] with [`RandomState`].
#[cfg(feature = "indexmap")]
pub type IndexMap<K, V, S = RandomState> = indexmap::IndexMap<K, V, S>;

/// [`indexmap::IndexSet`] with [`RandomState`].
#[cfg(feature = "indexmap")]
pub type IndexSet<T, S = RandomState> = indexmap::IndexSet<T, S>;

/// [`dashmap::DashMap`] with [`RandomState`].
#[cfg(feature = "dashmap")]
pub type DashMap<K, V, S = RandomState> = dashmap::DashMap<K, V, S>;

/// [`dashmap::DashSet`] with [`RandomState`].
#[cfg(feature = "dashmap")]
pub type DashSet<K, S = RandomState> = dashmap::DashSet<K, S>;
