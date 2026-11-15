//! `<template>.patch.json`: the patches of one template, as the merge collected them.
//!
//! The merge writes it when asked (`DebugOptions::output_patch_json`), and the conflict check of the
//! patch page reads it back. Both go through [`PatchJson`], so a field renamed here is renamed for
//! both, and nothing reads a name of its own.
//!
//! ```json
//! {
//!   "one": { "#0106/hkbModifierList/enable": { "patch": { "action": .., "value": false }, "priority": 7 } },
//!   "seq": { "#0106/hkbModifierList/modifiers": [ { "patch": { "action": { "kind": "seq_push" }, .. } } ] },
//!   "overridden": { "#0106/hkbModifierList/enable": [ { "patch": { .. "value": true }, "priority": 3 } ] }
//! }
//! ```

use super::{OnePatchMap, SeqPatchMap};

/// See the module docs.
///
/// Generic over how the maps are held: the merge writes references to its own
/// (`PatchJson<&OnePatchMap, &SeqPatchMap, Overridden>`), and a reader owns what it read
/// ([`ReadPatchJson`]).
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PatchJson<One, Seq, Lost> {
    /// One value per path: the highest priority's. See [`OnePatchMap`].
    pub one: One,
    /// Arrays: what every mod does to them. See [`SeqPatchMap`].
    pub seq: Seq,
    /// What lost at each path of `one`: not applied. See [`OnePatchMap::overridden`].
    pub overridden: Lost,
}

/// A [`PatchJson`] as read from its file. What lost is put back into `one` with
/// [`OnePatchMap::insert_losers`].
pub type ReadPatchJson<'a> = PatchJson<OnePatchMap<'a>, SeqPatchMap<'a>, SeqPatchMap<'a>>;
