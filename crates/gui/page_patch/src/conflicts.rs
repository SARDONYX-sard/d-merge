//! Which mods change the same thing, like MO2's `+` and `-`.
//!
//! The merge knows: it writes its patches as JSON when asked (`DebugOptions::output_patch_json`),
//! and `patch_json_only` stops it there. This reads those files and keeps what several mods disagree
//! on, so changing the load order later only needs [`Conflicts::marks`], no new reading.
//!
//! ```text
//! <output>/.d_merge/.debug/patches/meshes/actors/character/behaviors/0_master.patch.json
//! ```
//! A behavior's file is a `behavior_merge_types::patches::PatchJson`, read with that type:
//! - `one`, a single value: the highest priority wins, the others are in `overridden`.
//! - `seq`, arrays: where two mods write different things to the same element, see [`read_array`].
//!   `seq_push` and adding conflict with nothing: a merge keeps them all.
//!
//! `animationdatasinglefile` and `animationsetdatasinglefile` are lists of every mod's patches
//! (see [`read_anim_data`]), where each field that is not `null` is a change.
//!
//! When the user patch was written, only its own files are read again ([`Conflicts::read_user_patch`]):
//! its values at the conflicts found change, the other mods' do not.
//!
//! An operation can be hidden, see [`HiddenPatches`]. The check does not leave it out of the files,
//! so hiding and showing it needs no new scan: the flags just stop counting it.
//!
//! # Resolved in the user patch
//! The user patch is scanned as a mod ([`USER_PATCH_ID`]) and loads last. A place it has a value is
//! decided there: it is counted as resolved, apart from what is still to do, and it no longer makes
//! the mods there win or lose.
//!
//! ```text
//! #0106/enable  A: true  B: false  user patch: false  -> A and B: resolved 1 (✓, gray)
//! #0107/enable  A: 1     B: 2                         -> A: - 1, B: + 1 (still to do)
//! ```

use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
};

use behavior_merge::HiddenPatches;
use behavior_merge_types::patches::ReadPatchJson;
use fast_hash::HashMap;
use gui_shared::{
    patch::{BulkKind, BulkResolution, ResolveRequest, ResolveSide, USER_PATCH_ID},
    settings::pages::patch::mod_item::ModItem,
};
use json_patch::{Action, Op, ValueWithPriority};
use parking_lot::Mutex;
use simd_json::prelude::{ValueAsArray as _, Writable as _};
use sonic_rs::{JsonContainerTrait as _, JsonValueTrait as _};

/// How much of a value is kept to show. The whole value is in the mod's file.
const PREVIEW_CHARS: usize = 400;

/// A mod the scan ran with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScannedMod {
    /// `ModItem::id`
    pub(crate) id: String,
    pub(crate) name: String,
    /// Its priority in that run: how `one` and `seq` name their mods.
    pub(crate) priority: usize,
    /// Its directory, where its patch files are. `None`: FNIS, whose patches are generated.
    pub(crate) dir: Option<String>,
}

impl ScannedMod {
    /// What hidden operations name it by: its folder. e.g. `D:/mods/a/Nemesis_Engine/mod/para` -> `para`
    pub(crate) fn code(&self) -> Option<&str> {
        Path::new(self.dir.as_deref()?).file_name()?.to_str()
    }

    fn hides(&self, point: &Point, hidden: Option<&HiddenPatches>) -> bool {
        let Some((hidden, code)) = hidden.zip(self.code()) else { return false };
        hidden.contains(code, &point.target, &point.path)
    }
}

/// What one mod puts somewhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Value {
    /// In [`Conflicts::mods`].
    pub(crate) owner: u32,
    pub(crate) hash: u64,
    /// The start of the value.
    pub(crate) preview: Box<str>,
}

/// A place several mods put different values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Point {
    /// What is patched, e.g. `meshes/actors/character/behaviors/0_master.bin`
    pub(crate) target: Box<str>,
    /// e.g. `#0106/hkbModifierList/enable`, `DefaultMale~1/Idle~348/play_back_speed`
    pub(crate) path: Box<str>,
    pub(crate) values: Vec<Value>,
}

/// One mod's flags: `+` it overrides another mod somewhere, `-` another mod overrides it somewhere.
/// Places the user patch decided count apart, as `resolved`: see the module docs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Mark {
    pub(crate) wins: u32,
    pub(crate) loses: u32,
    pub(crate) resolved: u32,
}

impl Mark {
    /// What the mod list shows. `✓`: all its conflicts are resolved in the user patch.
    pub(crate) const fn glyph(self) -> &'static str {
        match (self.wins > 0, self.loses > 0, self.resolved > 0) {
            (true, true, _) => "±",
            (true, false, _) => "+",
            (false, true, _) => "-",
            (false, false, true) => "✓",
            (false, false, false) => "",
        }
    }

    /// Only resolved conflicts: nothing is left to do for this mod.
    pub(crate) const fn is_all_resolved(self) -> bool {
        self.wins == 0 && self.loses == 0 && self.resolved > 0
    }
}

/// What another mod is to the mod picked in the list, like MO2's colors when a mod is selected.
///
/// ```text
/// picked: B (priority 5)
/// A (3): B puts other values than A's at 12 places -> overridden_by_picked 12   (green)
/// C (8): C puts other values than B's at 2 places  -> overrides_picked 2        (red)
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Relation {
    /// Places where the picked mod's value replaces this mod's: it comes later.
    pub(crate) overridden_by_picked: u32,
    /// Places where this mod's value replaces the picked mod's.
    pub(crate) overrides_picked: u32,
    /// Places they put other values, which the user patch decided: neither replaces the other.
    pub(crate) resolved: u32,
}

impl Relation {
    /// How much they meet: what the color and the line are as strong as.
    pub(crate) const fn count(self) -> u32 {
        self.overridden_by_picked + self.overrides_picked + self.resolved
    }

    /// They only meet where the user patch decided: shown in gray.
    pub(crate) const fn is_all_resolved(self) -> bool {
        self.overridden_by_picked == 0 && self.overrides_picked == 0 && self.resolved > 0
    }
}

/// What a scan found. See the module docs.
#[derive(Debug, Default, Clone)]
pub(crate) struct Conflicts {
    pub(crate) mods: Vec<ScannedMod>,
    pub(crate) points: Vec<Point>,
    /// Per mod of [`Self::mods`]: how many of its operations conflict with nothing, so that they are
    /// kept whatever the load order. See [`Self::dead_mods`].
    pub(crate) useful: Vec<u32>,
}

/// The directory the merge writes the JSON to.
pub(crate) fn debug_patches_dir(output: &str) -> PathBuf {
    stdx::path::joined(Path::new(output), &[".d_merge", ".debug", "patches"])
}

impl Conflicts {
    /// Reads every `*.patch.json` under `dir`, written by a run with `mods` enabled. A file that
    /// cannot be read is logged and left out: the rest are still right.
    pub(crate) fn read(dir: &Path, mods: Vec<ScannedMod>) -> Self {
        let mut points = Vec::new();
        let names = Owners::new(&mods);
        let mut folders = vec![dir.to_path_buf()];
        while let Some(folder) = folders.pop() {
            let Ok(entries) = std::fs::read_dir(&folder) else { continue };
            for path in entries.flatten().map(|entry| entry.path()) {
                if path.is_dir() {
                    folders.push(path);
                } else if let Some(target) = target_of_file(dir, &path) {
                    match std::fs::read_to_string(&path) {
                        Ok(json) => read_file(&target, &json, &names, &mut points),
                        Err(err) => tracing::error!("{}: {err}", path.display()),
                    }
                }
            }
        }
        points.sort_unstable_by(|a, b| (&a.target, &a.path).cmp(&(&b.target, &b.path)));
        let useful = names.useful.into_inner();
        Self { mods, points, useful }
    }

    /// The flags of the mods, by `ModItem::id`, for the load order now: `priority_of` gives a mod's
    /// priority, `None` if it is off now.
    ///
    /// The highest priority wins a point. Those with its value are not overridden by it. What is in
    /// `hidden` is as if the mod did not do it. Where the user patch has a value, each other mod there
    /// is `resolved` instead (see the module docs).
    pub(crate) fn marks(
        &self,
        priority_of: impl Fn(&str) -> Option<usize>,
        hidden: Option<&HiddenPatches>,
    ) -> HashMap<String, Mark> {
        let mut marks: HashMap<String, Mark> = HashMap::default();
        for point in &self.points {
            let enabled = point.values.iter().filter_map(|value| {
                let owner = &self.mods[value.owner as usize];
                if owner.hides(point, hidden) {
                    return None;
                }
                priority_of(&owner.id).map(|priority| (priority, owner, value.hash))
            });

            if enabled.clone().any(|(_, owner, _)| owner.id == USER_PATCH_ID) {
                for (_, owner, _) in enabled.filter(|(_, owner, _)| owner.id != USER_PATCH_ID) {
                    marks.entry(owner.id.clone()).or_default().resolved += 1;
                }
                continue;
            }

            let Some((_, winner, hash)) = enabled.clone().max_by_key(|(priority, ..)| *priority) else { continue };
            let losers: Vec<_> = enabled.filter(|(.., other)| *other != hash).collect();
            if losers.is_empty() {
                continue;
            }
            marks.entry(winner.id.clone()).or_default().wins += 1;
            for (_, loser, _) in losers {
                marks.entry(loser.id.clone()).or_default().loses += 1;
            }
        }
        marks
    }

    /// The mods whose patch does nothing in the end, like MO2's lightning: they lose every conflict
    /// they are in, no value of theirs is the one kept, and all else they do is such a conflict.
    ///
    /// `marks`: what [`Self::marks`] gave for the same load order and `hidden`.
    pub(crate) fn dead_mods(
        &self,
        marks: &HashMap<String, Mark>,
        priority_of: impl Fn(&str) -> Option<usize>,
        hidden: Option<&HiddenPatches>,
    ) -> fast_hash::HashSet<String> {
        let mut keeps_a_value: fast_hash::HashSet<&str> = fast_hash::HashSet::default();
        for point in &self.points {
            let enabled = point.values.iter().filter_map(|value| {
                let owner = &self.mods[value.owner as usize];
                if owner.hides(point, hidden) {
                    return None;
                }
                priority_of(&owner.id).map(|priority| (priority, owner, value.hash))
            });
            let Some((.., kept)) = enabled.clone().max_by_key(|(priority, ..)| *priority) else { continue };
            keeps_a_value.extend(enabled.filter(|(.., hash)| *hash == kept).map(|(_, owner, _)| owner.id.as_str()));
        }
        let useful_of = |id: &str| {
            let at = self.mods.iter().position(|scanned| scanned.id == id);
            at.and_then(|at| self.useful.get(at)).copied().unwrap_or(0)
        };
        marks
            .iter()
            .filter(|(id, mark)| {
                let meets_others = mark.loses > 0 || mark.resolved > 0;
                mark.wins == 0 && meets_others && !keeps_a_value.contains(id.as_str()) && useful_of(id) == 0
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// The points `mod_id` is part of.
    pub(crate) fn of_mod(&self, mod_id: &str) -> Vec<&Point> {
        let Some(at) = self.mods.iter().position(|scanned| scanned.id == mod_id) else { return Vec::new() };
        #[expect(clippy::cast_possible_truncation, reason = "far fewer than 4 billion mods")]
        let at = at as u32;
        self.points.iter().filter(|point| point.values.iter().any(|value| value.owner == at)).collect()
    }

    /// What each other mod is to `picked`, for the load order `priority_of` gives (`None`: off). Two
    /// mods at a time, as MO2: where they put other values, the later one replaces the other, whoever
    /// wins the place in the end. What is in `hidden` is as if the mod did not do it.
    pub(crate) fn relations(
        &self,
        picked: &str,
        priority_of: impl Fn(&str) -> Option<usize>,
        hidden: Option<&HiddenPatches>,
    ) -> HashMap<String, Relation> {
        let mut relations: HashMap<String, Relation> = HashMap::default();

        let Some(at) = self.mods.iter().position(|scanned| scanned.id == picked) else { return relations };
        let Some(own_priority) = priority_of(picked) else { return relations };
        let picked_mod = &self.mods[at];
        #[expect(clippy::cast_possible_truncation, reason = "far fewer than 4 billion mods")]
        let at = at as u32;

        for point in &self.points {
            let Some(own) = point.values.iter().find(|value| value.owner == at) else { continue };
            if picked_mod.hides(point, hidden) {
                continue;
            }
            let resolved = self.is_resolved(point, &priority_of, hidden);

            for value in point.values.iter().filter(|value| value.owner != at && value.hash != own.hash) {
                let other = &self.mods[value.owner as usize];
                if other.id == USER_PATCH_ID || other.hides(point, hidden) {
                    continue;
                }
                let Some(priority) = priority_of(&other.id) else { continue };

                let relation = relations.entry(other.id.clone()).or_default();
                if resolved {
                    relation.resolved += 1;
                } else if own_priority > priority {
                    relation.overridden_by_picked += 1;
                } else {
                    relation.overrides_picked += 1;
                }
            }
        }

        relations
    }
}

impl Conflicts {
    /// Does the user patch have a value at `point`, on and not hidden? Then it decides it.
    fn is_resolved(
        &self,
        point: &Point,
        priority_of: &impl Fn(&str) -> Option<usize>,
        hidden: Option<&HiddenPatches>,
    ) -> bool {
        point.values.iter().any(|value| {
            let owner = &self.mods[value.owner as usize];
            owner.id == USER_PATCH_ID && !owner.hides(point, hidden) && priority_of(&owner.id).is_some()
        })
    }
}

impl Conflicts {
    /// The user patch was written: its values at the conflicts found are read again from its own
    /// files (a few), not from every mod's. They are worked out as a scan does: a field's value, or
    /// what it writes to the elements of an array's range. Its animation data is not looked at.
    ///
    /// What it now writes where the other mods agree is no conflict found here: the next scan finds
    /// that one.
    fn read_user_patch(&mut self, user_patch: ScannedMod) {
        let Some(dir) = user_patch.dir.clone() else { return };
        let owner = self.user_patch_owner(user_patch);
        for point in &mut self.points {
            point.values.retain(|value| value.owner != owner);
        }

        // Of each array point: what the user patch writes to its elements, by the template's index.
        let mut arrays: HashMap<usize, BTreeMap<usize, String>> = HashMap::default();
        for (target, file) in user_patch_files(Path::new(&dir)) {
            let Ok(text) = std::fs::read_to_string(&file) else { continue };
            let hacks = Some(behavior_merge::HackOptions::enable_all().into());
            let Ok((patches, _)) = nemesis_xml::patch::parse_nemesis_patch(&text, hacks) else { continue };

            for (path, patch) in patches {
                let path = path.join("/");
                if let Some(elements) = element_texts(&patch) {
                    let of_array = self.points.iter().enumerate().filter(|(_, point)| *point.target == *target);
                    for (at, range) in of_array.filter_map(|(at, point)| Some((at, array_range(&point.path, &path)?))) {
                        let written = elements.iter().filter(|(index, _)| range.contains(index));
                        arrays.entry(at).or_default().extend(written.cloned());
                    }
                } else if matches!(patch.action, Action::Pure { .. })
                    && let Some(point) =
                        self.points.iter_mut().find(|point| *point.target == *target && *point.path == *path)
                {
                    point.values.push(value_of_text(owner, &patch.value.encode()));
                }
            }
        }

        for (at, elements) in arrays {
            let joined = elements.values().map(|text| text.trim_matches('"')).collect::<Vec<_>>().join("\n");
            self.points[at].values.push(value_of_text(owner, &joined));
        }
    }

    /// The user patch's index in [`Self::mods`], added if the scan had it not.
    fn user_patch_owner(&mut self, user_patch: ScannedMod) -> u32 {
        let at = self.mods.iter().position(|scanned| scanned.id == USER_PATCH_ID).unwrap_or_else(|| {
            self.mods.push(user_patch);
            self.useful.push(0);
            self.mods.len() - 1
        });
        #[expect(clippy::cast_possible_truncation, reason = "far fewer than 4 billion mods")]
        let at = at as u32;
        at
    }
}

/// The behavior patch files of the user patch at `dir` (NemesisExt), with their template.
///
/// e.g. `<dir>/meshes/actors/character/behaviors/0_master.bin/#0106.txt` ->
/// `(meshes/actors/character/behaviors/0_master.bin, <that file>)`
fn user_patch_files(dir: &Path) -> Vec<(Box<str>, PathBuf)> {
    let mut files = Vec::new();
    let mut folders = vec![dir.join("meshes")];
    while let Some(folder) = folders.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else { continue };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                folders.push(path);
                continue;
            }
            let is_patch = path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with('#'));
            let target = path.parent().and_then(|parent| parent.strip_prefix(dir).ok());
            if let (true, Some(target)) = (is_patch, target) {
                files.push((target.to_string_lossy().replace('\\', "/").into(), path));
            }
        }
    }
    files
}

/// The range of the array point at `point_path`, if it is one of the array at `array_path`.
///
/// e.g. `#2489/C/generators/[0:2]`, `#2489/C/generators` -> `0..2`
fn array_range(point_path: &str, array_path: &str) -> Option<std::ops::Range<usize>> {
    let range = point_path.strip_prefix(array_path)?.strip_prefix("/[")?.strip_suffix(']')?;
    let (start, end) = range.split_once(':')?;
    Some(start.parse().ok()?..end.parse().ok()?)
}

/// `<dir>/meshes/a/b.patch.json` -> `meshes/a/b.bin`
fn target_of_file(dir: &Path, path: &Path) -> Option<Box<str>> {
    let relative = path.strip_prefix(dir).ok()?.to_str()?.replace('\\', "/");
    Some(format!("{}.bin", relative.strip_suffix(".patch.json")?).into())
}

/// Finds a mod by what the JSON names it with.
struct Owners {
    by_priority: HashMap<usize, u32>,
    /// By its directory in one form.
    by_dir: HashMap<String, u32>,
    /// See [`Conflicts::useful`]. A cell, as a file is read with this borrowed.
    useful: std::cell::RefCell<Vec<u32>>,
}

impl Owners {
    fn new(mods: &[ScannedMod]) -> Self {
        let (mut by_priority, mut by_dir) = (HashMap::default(), HashMap::default());
        #[expect(clippy::cast_possible_truncation, reason = "far fewer than 4 billion mods")]
        for (at, scanned) in mods.iter().enumerate() {
            by_priority.insert(scanned.priority, at as u32);
            if let Some(dir) = &scanned.dir {
                by_dir.insert(dir_key(dir), at as u32);
            }
        }
        Self { by_priority, by_dir, useful: std::cell::RefCell::new(vec![0; mods.len()]) }
    }

    /// One more operation of this mod conflicts with nothing.
    fn note_useful(&self, owner: u32) {
        if let Some(count) = self.useful.borrow_mut().get_mut(owner as usize) {
            *count += 1;
        }
    }

    /// The owner of an operation, by its priority in the run.
    fn of_entry(&self, entry: &ValueWithPriority<'_>) -> Option<u32> {
        self.by_priority.get(&entry.priority).copied()
    }
}

/// The same directory, however it was written. e.g. `D:\a/B` -> `d:/a/b`
fn dir_key(dir: &str) -> String {
    dir.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

fn read_file(target: &str, json: &str, owners: &Owners, points: &mut Vec<Point>) {
    // Animation data is a list of every mod's patches; a behavior, a `PatchJson`.
    let result = if json.trim_start().starts_with('[') {
        sonic_rs::from_str(json).map(|value| read_anim_data(target, &value, owners, points))
    } else {
        sonic_rs::from_str(json).map(|file| read_behavior(target, file, owners, points))
    };
    if let Err(err) = result {
        tracing::error!("{target}: {err}");
    }
}

/// A value in one form, for comparing: flags in any order are the same flags.
fn value_hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    if text.contains('|') {
        let mut flags: Vec<&str> = text.trim_matches('"').split('|').map(str::trim).collect();
        flags.sort_unstable();
        flags.join("|").hash(&mut hasher);
    } else {
        text.hash(&mut hasher);
    }
    hasher.finish()
}

/// What `owner` puts somewhere, `text` being the value as JSON.
fn value_of_text(owner: u32, text: &str) -> Value {
    let preview: String = text.trim_matches('"').chars().take(PREVIEW_CHARS).collect();
    Value { owner, hash: value_hash(text), preview: preview.into() }
}

/// The owner and value of an operation of a behavior.
fn entry_value(entry: &ValueWithPriority<'_>, owners: &Owners) -> Option<Value> {
    let owner = owners.of_entry(entry)?;
    Some(value_of_text(owner, &entry.patch.value.encode()))
}

/// See the module docs. Only what disagrees is kept.
fn read_behavior(target: &str, file: ReadPatchJson<'_>, owners: &Owners, points: &mut Vec<Point>) {
    let mut one = file.one;
    one.insert_losers(file.overridden);

    for slot in one.into_entries() {
        // Nothing lost to it: no conflict.
        if slot.losers.is_empty() {
            if let Some(owner) = owners.of_entry(&slot.winner) {
                owners.note_useful(owner);
            }
            continue;
        }

        let values: Vec<Value> =
            std::iter::once(&slot.winner).chain(&slot.losers).filter_map(|entry| entry_value(entry, owners)).collect();
        push_if_different(target, &slot.key.join("/"), values, owners, points);
    }

    for (path, entries) in file.seq.into_groups() {
        read_array(target, &path.join("/"), &entries, owners, points);
    }
}

/// What an element becomes when a mod removes it.
const REMOVED_ELEMENT: &str = "(removed)";

/// What one mod writes to one element of an array, by the index in the template.
type ElementWrites = BTreeMap<usize, Vec<(u32, String)>>;

/// Adds what `entry` writes, if it replaces or removes a range. See [`read_array`].
fn add_element_writes(entry: &ValueWithPriority<'_>, owners: &Owners, writes: &mut ElementWrites) -> Option<()> {
    let elements = element_texts(&entry.patch)?;
    let owner = owners.of_entry(entry)?;
    for (index, text) in elements {
        writes.entry(index).or_default().push((owner, text));
    }
    Some(())
}

/// What `patch` writes to each element of an array, by its index in the template, if it replaces
/// or removes a range. A replace with fewer values than its range removes the rest.
///
/// e.g. `[1:3]` replaced by `[x]` -> `(1, "x")`, `(2, "(removed)")`
fn element_texts(patch: &json_patch::JsonPatch<'_>) -> Option<Vec<(usize, String)>> {
    let Action::Seq { op: op @ (Op::Replace | Op::Remove), range } = &patch.action else { return None };
    let values = if *op == Op::Replace { patch.value.as_array() } else { None };
    let texts = range.clone().map(|index| {
        let value = values.and_then(|values| values.get(index - range.start));
        (index, value.map_or_else(|| REMOVED_ELEMENT.to_owned(), |value| value.encode()))
    });
    Some(texts.collect())
}

/// The conflicts in one array, as `json_patch`'s `apply_ops_parallel` applies it.
///
/// Replaces and removes write to the elements of the template in priority order, by their original
/// index: a remove only marks its elements, so nothing shifts. So two mods disagree per **element**,
/// and ranges that overlap agree where they write the same. What adds comes after that, also by
/// original index, and is kept from all mods: it conflicts with nothing.
///
/// e.g. `0..2` -> `[a, b]` and `1..3` -> `[b, c]` agree on element 1 and have no conflict;
/// `1..3` -> `[x, c]` disagrees on element 1 only: `<path>/[1:2]`.
fn read_array(target: &str, path: &str, entries: &[ValueWithPriority<'_>], owners: &Owners, points: &mut Vec<Point>) {
    let mut writes = ElementWrites::new();
    for entry in entries {
        // What adds is kept from all mods.
        if add_element_writes(entry, owners, &mut writes).is_none()
            && let Some(owner) = owners.of_entry(entry)
        {
            owners.note_useful(owner);
        }
    }

    let agreed = writes.values().filter(|element| !element_differs(element));
    agreed.flatten().for_each(|(owner, _)| owners.note_useful(*owner));

    let mut group: Option<ElementGroup<'_>> = None;
    for (&index, element) in writes.iter().filter(|(_, element)| element_differs(element)) {
        let owners_here = element_owners(element);
        match &mut group {
            Some(open) if open.end == index && open.owners == owners_here => {
                open.end = index + 1;
                open.elements.push(element);
            }
            _ => {
                if let Some(done) = group.take() {
                    done.push_to(target, path, owners, points);
                }
                group =
                    Some(ElementGroup { start: index, end: index + 1, owners: owners_here, elements: vec![element] });
            }
        }
    }
    if let Some(done) = group {
        done.push_to(target, path, owners, points);
    }
}

/// Do two mods write different things to this element?
fn element_differs(element: &[(u32, String)]) -> bool {
    let first = element.first().map(|(_, text)| value_hash(text));
    element.iter().any(|(_, text)| Some(value_hash(text)) != first)
}

fn element_owners(element: &[(u32, String)]) -> Vec<u32> {
    let mut owners: Vec<u32> = element.iter().map(|(owner, _)| *owner).collect();
    owners.sort_unstable();
    owners.dedup();
    owners
}

/// Elements next to each other that the same mods disagree on: one conflict to look at.
struct ElementGroup<'a> {
    start: usize,
    end: usize,
    owners: Vec<u32>,
    elements: Vec<&'a Vec<(u32, String)>>,
}

impl ElementGroup<'_> {
    fn push_to(self, target: &str, path: &str, owners: &Owners, points: &mut Vec<Point>) {
        let values = self
            .owners
            .iter()
            .map(|&owner| {
                let texts = self.elements.iter().flat_map(|element| element.iter()).filter(|(at, _)| *at == owner);
                let joined = texts.map(|(_, text)| text.trim_matches('"')).collect::<Vec<_>>().join("\n");
                Value {
                    owner,
                    hash: value_hash(&joined),
                    preview: joined.chars().take(PREVIEW_CHARS).collect::<String>().into(),
                }
            })
            .collect();
        push_if_different(target, &format!("{path}/[{}:{}]", self.start, self.end), values, owners, points);
    }
}

/// What does not differ conflicts with nothing: that is useful of each mod there.
fn push_if_different(target: &str, path: &str, mut values: Vec<Value>, owners: &Owners, points: &mut Vec<Point>) {
    // Several places of one mod are one value here: the others say the same about that mod.
    values.sort_unstable_by_key(|value| value.owner);
    values.dedup_by_key(|value| (value.owner, value.hash));

    let owners_differ = values.windows(2).any(|pair| pair[0].owner != pair[1].owner);
    let values_differ = values.windows(2).any(|pair| pair[0].hash != pair[1].hash);
    if owners_differ && values_differ {
        points.push(Point { target: target.into(), path: path.into(), values });
    } else {
        for value in &values {
            owners.note_useful(value.owner);
        }
    }
}

/// The names `animationdatasinglefile.patch.json` / `animationsetdatasinglefile.patch.json` are written
/// with: serde's, of `behavior_merge_types::adsf::AdsfPatch` and `asdsf::AsdsfPatch`. Those types are
/// written only, not read back, so their names are here. A rename there is to be made here too: the
/// tests read real patches of those types, and fail until it is.
mod anim_json {
    /// The fields of `AdsfPatch` / `AsdsfPatch`: the project, the mod (its directory), the patch.
    pub(super) const TARGET: &str = "target";
    pub(super) const MOD_DIR: &str = "id";
    pub(super) const PATCH: &str = "patch";

    /// The variants of `PatchKind` whose patch is a whole file: compared as one value.
    pub(super) const WHOLE_FILES: [&str; 3] = ["SpecifiedAnim", "SpecifiedMotion", "AddAnimSet"];
    /// The variants of `PatchKind` whose patch is fields of a file: compared field by field.
    pub(super) const EDITED_FIELDS: [&str; 3] = ["EditAnim", "EditMotion", "EditAnimSet"];
    /// The field of a variant naming its file. A variant without one is a header: a list merged.
    pub(super) const FILE_NAMES: [&str; 4] = ["name_clip", "clip_id", "index", "file_name"];
    /// The field of a variant holding its patch.
    pub(super) const VARIANT_PATCH: &str = "patch";

    /// A field patched as one value: `json_patch::ValueWithPriority` (`{ "patch": { .., "value": .. } }`).
    pub(super) const ONE_PATCH: &str = "patch";
    pub(super) const ONE_PATCH_VALUE: &str = "value";
    /// The pushes of an array of a field patch (`NonNestedArrayDiff::seq`): they conflict with nothing.
    pub(super) const PUSHES: &str = "seq";
}

/// `animationdatasinglefile.patch.json`, `animationsetdatasinglefile.patch.json`: a list. See
/// [`anim_json`] for the names.
///
/// ```json
/// [ { "target": "DefaultMale~1", "id": "D:\\mods\\a\\Nemesis_Engine\\mod\\a",
///     "patch": { "EditAnim": { "name_clip": "Dodge~644", "priority": 39,
///       "patch": { "name": null, "play_back_speed": "1.1", "trigger_names_patches": { "one": {}, "seq": [] } } } } } ]
/// ```
/// Each field that is not `null` is a change at `<target>/<name>/<field>`. What adds is left out:
/// `AddAnim`, `AddMotion` and the pushes, as the merge keeps them all; headers are lists merged by the
/// merge.
fn read_anim_data(target: &str, json: &sonic_rs::Value, owners: &Owners, points: &mut Vec<Point>) {
    let mut places: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for entry in json.as_array().into_iter().flat_map(|list| list.iter()) {
        let (Some(project), Some(mod_dir), Some(patch)) = (
            entry.get(anim_json::TARGET).and_then(|v| v.as_str()),
            entry.get(anim_json::MOD_DIR).and_then(|v| v.as_str()),
            entry.get(anim_json::PATCH).and_then(|v| v.as_object()),
        ) else {
            continue;
        };
        let Some(&owner) = owners.by_dir.get(&dir_key(mod_dir)) else { continue };

        for (kind, body) in patch.iter() {
            let name = anim_json::FILE_NAMES.iter().find_map(|key| body.get(key).and_then(|v| v.as_str()));
            let Some(name) = name else {
                owners.note_useful(owner); // a header: a list merged by the merge
                continue;
            };

            let mut place = format!("{project}/{name}");
            let fields = body.get(anim_json::VARIANT_PATCH);
            if anim_json::WHOLE_FILES.contains(&kind) {
                if let Some(value) = fields {
                    places.entry(place).or_default().push(sonic_value(owner, value));
                }
            } else if anim_json::EDITED_FIELDS.contains(&kind) {
                if let Some(fields) = fields {
                    flatten(&mut place, fields, owner, &mut places);
                }
            } else {
                owners.note_useful(owner); // what adds
            }
        }
    }

    for (path, values) in places {
        push_if_different(target, &path, values, owners, points);
    }
}

/// What `owner` puts somewhere, from the JSON of animation data.
fn sonic_value(owner: u32, value: &sonic_rs::Value) -> Value {
    value_of_text(owner, &sonic_rs::to_string(value).unwrap_or_default())
}

/// The leaves of a patch, each at `<place>/<field>/..`. See [`read_anim_data`].
///
/// `place` is one buffer for the whole walk: a patch has many fields, and few of them are leaves.
fn flatten(place: &mut String, value: &sonic_rs::Value, owner: u32, places: &mut BTreeMap<String, Vec<Value>>) {
    if value.is_null() {
        return;
    }

    // A field patched as one value: one change.
    if let Some(inner) = value.get(anim_json::ONE_PATCH).and_then(|patch| patch.get(anim_json::ONE_PATCH_VALUE)) {
        places.entry(place.clone()).or_default().push(sonic_value(owner, inner));
        return;
    }

    if let Some(object) = value.as_object() {
        for (key, field) in object.iter().filter(|(key, _)| *key != anim_json::PUSHES) {
            let at = place.len();
            place.push('/');
            place.push_str(key);
            flatten(place, field, owner, places);
            place.truncate(at);
        }
        return;
    }

    places.entry(place.clone()).or_default().push(sonic_value(owner, value));
}

/// The scan's state in the page. See the module docs.
#[derive(Debug, Default)]
pub(crate) struct ConflictScan {
    /// Waiting for the merge to write the files: where they go, and with which mods.
    waiting: Option<(PathBuf, Vec<ScannedMod>)>,
    /// Filled by the worker that reads the files.
    read_result: Arc<Mutex<Option<Conflicts>>>,
    is_reading: bool,
    /// When the check that is running started.
    started: Option<std::time::Instant>,
    /// The last result.
    pub(crate) current: Option<Arc<Conflicts>>,
    /// See [`Conflicts::marks`], for the load order `marks_for` tells.
    marks: HashMap<String, Mark>,
    /// See [`Conflicts::dead_mods`], for the same.
    dead: fast_hash::HashSet<String>,
    marks_for: u64,
    /// Priorities of the mods that are on now.
    priorities: HashMap<String, usize>,
    /// A mod is on now that was not when the files were written.
    stale: bool,
    /// The mod whose window is open, and what it shows.
    pub(crate) inspected: Option<String>,
    pub(crate) filter: String,
    /// The conflict shown in detail: `(target, path)`. The same path can be in two templates, e.g.
    /// the 1st and the 3rd person's `0_master`.
    selected: Option<(Box<str>, Box<str>)>,
    /// How much of the window the list takes, once the bar below it was dragged.
    pub(crate) list_share: Option<f32>,
    /// The patch page's output directory, where [`Self::hidden`] is kept.
    hidden_output: Option<String>,
    /// All that is hidden, see [`crate::hidden`].
    hidden: HiddenPatches,
    /// `apply_user_patch` is on: what is hidden counts.
    hidden_applies: bool,
    /// Changes with [`Self::hidden`], for [`Self::refresh_marks`].
    hidden_revision: u64,
    /// Why what is hidden could not be saved.
    pub(crate) hidden_error: Option<String>,
    /// The mod picked in the list (a click on its row): the others are colored by what they are to it.
    picked: Option<String>,
    /// See [`Conflicts::relations`], for [`Self::picked`] and the load order `relations_for` tells.
    relations: HashMap<String, Relation>,
    /// The highest [`Relation::count`] of [`Self::relations`]: the strongest color.
    relations_max: u32,
    relations_for: u64,
    /// Changes whenever what [`Self::bulk_resolutions`] finds may: a scan, the user patch read again,
    /// the load order or what is hidden.
    revision: u64,
}

impl ConflictScan {
    pub(crate) fn with_hidden(hidden: HiddenPatches) -> Self {
        Self { hidden, hidden_applies: true, ..Self::default() }
    }

    /// The output directory may have changed: its own hidden operations count then.
    pub(crate) fn follow_output(&mut self, output: &str) {
        if self.hidden_output.as_deref() != Some(output) {
            self.hidden_output = Some(output.to_owned());
            self.hidden = crate::hidden::load(output);
            self.hidden_revision += 1;
        }
    }

    fn save_hidden(&mut self) {
        self.hidden_revision += 1;
        self.hidden_error = crate::hidden::save(self.hidden_output.as_deref().unwrap_or_default(), &self.hidden).err();
    }

    /// Is anything hidden?
    pub(crate) fn has_hidden(&self) -> bool {
        !self.hidden.is_empty()
    }

    /// Shows all the hidden operations again.
    pub(crate) fn show_all_hidden(&mut self) {
        self.hidden = HiddenPatches::default();
        self.save_hidden();
    }

    /// Does this mod's patch do nothing in the end? See [`Conflicts::dead_mods`].
    pub(crate) fn is_dead(&self, mod_id: &str) -> bool {
        self.dead.contains(mod_id)
    }

    /// What is hidden, if it counts.
    fn applied_hidden(&self) -> Option<&HiddenPatches> {
        self.hidden_applies.then_some(&self.hidden)
    }

    /// Is this mod's operation at `point` hidden?
    pub(crate) fn is_hidden(&self, owner: &ScannedMod, point: &Point) -> bool {
        owner.hides(point, Some(&self.hidden))
    }

    /// Hides the mod's operation at `point`, or shows it again. It counts at once for the flags, and
    /// for what is generated at the next merge.
    pub(crate) fn toggle_hidden(&mut self, mod_code: &str, point: &Point) {
        self.hidden.toggle(mod_code, &point.target, &point.path);
        self.save_hidden();
    }

    pub(crate) const fn is_scanning(&self) -> bool {
        self.waiting.is_some() || self.is_reading
    }

    pub(crate) const fn is_stale(&self) -> bool {
        self.stale
    }

    /// Can a check follow what changed by itself? There is a result to keep up to date, and the mods
    /// that are on now are the ones it saw.
    pub(crate) const fn can_follow(&self) -> bool {
        self.current.is_some() && !self.stale
    }

    /// The merge is told to write the files: after it ends, [`Self::on_merge_ended`] reads them.
    pub(crate) fn wait_for(&mut self, dir: PathBuf, mods: Vec<ScannedMod>) {
        self.waiting = Some((dir, mods));
        self.started = Some(std::time::Instant::now());
    }

    /// Reads the files on a worker thread, if this merge was for them. Errors of the merge do not
    /// matter: a mod that cannot be parsed just has nothing in them.
    pub(crate) fn on_merge_ended(&mut self) {
        let Some((dir, mods)) = self.waiting.take() else { return };
        self.is_reading = true;
        let result = Arc::clone(&self.read_result);
        std::thread::spawn(move || {
            let found = Conflicts::read(&dir, mods);
            *result.lock() = Some(found);
        });
    }

    /// Takes in what the worker read. Returns how many conflicts it found and how long the check took,
    /// when it is done.
    pub(crate) fn poll(&mut self) -> Option<(usize, std::time::Duration)> {
        let found = self.read_result.try_lock().and_then(|mut result| result.take())?;
        let count = found.points.len();
        self.current = Some(Arc::new(found));
        self.revision += 1;
        self.is_reading = false;
        self.marks_for = 0; // computed again
        Some((count, self.started.take().map_or_else(Default::default, |started| started.elapsed())))
    }

    /// Updates the flags if the load order or what is on changed. Runs every frame, and only compares
    /// a hash then: a list has a few hundred mods.
    ///
    /// `user_patch`: its priority, if it is merged: it is not in `list`.
    /// `hidden_applies`: `apply_user_patch` is on.
    pub(crate) fn refresh_marks(&mut self, list: &[ModItem], user_patch: Option<usize>, hidden_applies: bool) {
        self.hidden_applies = hidden_applies;
        let Some(found) = &self.current else { return };
        let hash = {
            let mut hasher = DefaultHasher::new();
            for item in list.iter().filter(|item| item.enabled) {
                (&item.id, item.priority).hash(&mut hasher);
            }
            (user_patch, hidden_applies, self.hidden_revision).hash(&mut hasher);
            hasher.finish() | 1 // never the 0 of "computed again"
        };
        if hash == self.marks_for {
            return;
        }
        self.marks_for = hash;
        self.revision += 1;
        self.priorities =
            list.iter().filter(|item| item.enabled).map(|item| (item.id.clone(), item.priority)).collect();
        if let Some(priority) = user_patch {
            self.priorities.insert(USER_PATCH_ID.to_owned(), priority);
        }
        let hidden = self.hidden_applies.then_some(&self.hidden);
        self.marks = found.marks(|id| self.priorities.get(id).copied(), hidden);
        self.dead = found.dead_mods(&self.marks, |id| self.priorities.get(id).copied(), hidden);
        self.stale = self.priorities.keys().any(|id| !found.mods.iter().any(|scanned| scanned.id == *id));
    }

    /// A click on a mod's row: picks it, or puts it down if it is the one picked.
    pub(crate) fn pick(&mut self, mod_id: &str) {
        self.picked = if self.picked.as_deref() == Some(mod_id) { None } else { Some(mod_id.to_owned()) };
    }

    pub(crate) fn picked(&self) -> Option<&str> {
        self.picked.as_deref()
    }

    /// What `mod_id` is to the picked mod, and the strongest of all: see [`Conflicts::relations`].
    pub(crate) fn relation_of(&self, mod_id: &str) -> (Relation, u32) {
        (self.relations.get(mod_id).copied().unwrap_or_default(), self.relations_max)
    }

    /// Finds [`Self::relation_of`] again if the mod picked, the load order or what is hidden changed.
    /// Runs every frame after [`Self::refresh_marks`], and only compares a hash then.
    pub(crate) fn refresh_relations(&mut self) {
        let hash = {
            let mut hasher = DefaultHasher::new();
            (self.marks_for, &self.picked).hash(&mut hasher);
            hasher.finish() | 1
        };
        if hash == self.relations_for {
            return;
        }
        self.relations_for = hash;

        let (Some(found), Some(picked)) = (&self.current, &self.picked) else {
            self.relations.clear();
            self.relations_max = 0;
            return;
        };

        let hidden = self.hidden_applies.then_some(&self.hidden);
        self.relations = found.relations(picked, |id| self.priorities.get(id).copied(), hidden);
        self.relations_max = self.relations.values().map(|relation| relation.count()).max().unwrap_or(0);
    }

    pub(crate) fn is_selected(&self, point: &Point) -> bool {
        self.selected.as_ref().is_some_and(|(target, path)| *target == point.target && *path == point.path)
    }

    pub(crate) fn select(&mut self, point: &Point) {
        self.selected = Some((point.target.clone(), point.path.clone()));
    }

    /// The user patch was written: see [`Conflicts::read_user_patch`]. The flags and the colors are
    /// found again at the next frame.
    pub(crate) fn read_user_patch(&mut self, user_patch: ScannedMod) {
        let Some(current) = &mut self.current else { return };
        Arc::make_mut(current).read_user_patch(user_patch);
        (self.marks_for, self.relations_for) = (0, 0);
        self.revision += 1;
    }

    /// See [`Self::bulk_resolutions`].
    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    /// The conflicts of the last scan that can be decided all at once (see [`BulkKind`]), with the
    /// mods that are on now and what is hidden: those still at odds, and those the user patch decides.
    pub(crate) fn bulk_resolutions(&self) -> Vec<BulkResolution> {
        let Some(found) = &self.current else { return Vec::new() };
        found.points.iter().filter_map(|point| self.bulk_resolution(found, point)).collect()
    }

    /// `point` as a [`BulkResolution`], if it is of a kind and two mods or more have their own value
    /// there. Their order is the load order: the first's events come first.
    fn bulk_resolution(&self, found: &Conflicts, point: &Point) -> Option<BulkResolution> {
        let kind = BulkKind::of(&point.path)?;
        let hidden = self.applied_hidden();
        let mut on: Vec<(usize, &ScannedMod)> = point
            .values
            .iter()
            .map(|value| &found.mods[value.owner as usize])
            .filter(|owner| !owner.hides(point, hidden))
            .filter_map(|owner| Some((*self.priorities.get(&owner.id)?, owner)))
            .collect();
        on.sort_by_key(|(priority, _)| *priority);

        let resolved = on.iter().any(|(_, owner)| owner.id == USER_PATCH_ID);
        let sides: Vec<ResolveSide> = on
            .iter()
            .filter(|(_, owner)| owner.id != USER_PATCH_ID)
            .filter_map(|(_, owner)| Some(ResolveSide { name: owner.name.clone(), mod_dir: owner.dir.clone()? }))
            .collect();
        let mod_dir = sides.first()?.mod_dir.clone();
        if sides.len() < 2 {
            return None;
        }
        let request = ResolveRequest {
            target: point.target.to_string(),
            path: point.path.to_string(),
            mod_dir,
            edit: false,
            sides,
            keep_all_events: true,
        };
        Some(BulkResolution { kind, request, resolved })
    }

    pub(crate) fn mark_of(&self, mod_id: &str) -> Mark {
        self.marks.get(mod_id).copied().unwrap_or_default()
    }

    /// Do the mods that are on now still disagree at `point`, with what is hidden gone?
    pub(crate) fn is_contested(&self, found: &Conflicts, point: &Point) -> bool {
        let hidden = self.applied_hidden();
        let mut left = point.values.iter().filter(|value| {
            let owner = &found.mods[value.owner as usize];
            self.priorities.contains_key(&owner.id) && !owner.hides(point, hidden)
        });
        let first = left.next().map(|value| value.hash);
        left.any(|value| Some(value.hash) != first)
    }

    /// The mod that is on now and comes last among those at `point`, with what is hidden gone.
    pub(crate) fn winner_of<'a>(&self, found: &'a Conflicts, point: &Point) -> Option<&'a ScannedMod> {
        let hidden = self.applied_hidden();
        point
            .values
            .iter()
            .map(|value| &found.mods[value.owner as usize])
            .filter(|owner| !owner.hides(point, hidden))
            .filter_map(|owner| self.priorities.get(&owner.id).map(|priority| (*priority, owner)))
            .max_by_key(|(priority, _)| *priority)
            .map(|(_, owner)| owner)
    }
}

#[cfg(test)]
mod tests {
    use behavior_merge_types::{
        adsf::{AdsfPatch, EditAnim, PatchKind},
        patches::{OnePatchMap, PatchJson, SeqPatchMap},
    };
    use json_patch::{JsonPatch, json_path};
    use skyrim_anim_data::adsf::{
        normal::ClipAnimDataBlock, patch::de::others::clip_anim::deserializer::parse_clip_anim_diff_patch,
    };

    use super::*;

    fn mods() -> Vec<ScannedMod> {
        ["low", "high"]
            .iter()
            .enumerate()
            .map(|(priority, id)| ScannedMod {
                id: (*id).to_owned(),
                name: (*id).to_owned(),
                priority: priority + 3,
                dir: Some(format!("D:/mods/{id}")),
            })
            .collect()
    }

    /// As MO2's colors: the later mod replaces the other where they put other values.
    #[test]
    fn a_picked_mod_overrides_the_earlier_and_is_overridden_by_the_later() {
        let found = read(&behavior("false", "true"), "meshes/m.bin");
        let in_order = |id: &str| match id {
            "low" => Some(1),
            "high" => Some(2),
            _ => None,
        };

        let of_low = found.relations("low", in_order, None);
        assert_eq!(of_low["high"], Relation { overridden_by_picked: 0, overrides_picked: 1, resolved: 0 });
        let of_high = found.relations("high", in_order, None);
        assert_eq!(of_high["low"], Relation { overridden_by_picked: 1, overrides_picked: 0, resolved: 0 });

        // A mod that is off is nothing to the others, and has nothing itself.
        let low_off = |id: &str| (id != "low").then(|| in_order(id)).flatten();
        assert!(found.relations("high", low_off, None).is_empty());
        assert!(found.relations("low", low_off, None).is_empty());
    }

    /// The user patch written: only its files are read again, and its value is at the conflict.
    #[test]
    fn the_user_patch_alone_is_read_again() {
        const PATH: [&str; 3] = ["#0100", "hkbModifierList", "enable"];
        let json = {
            let mut one = OnePatchMap::default();
            one.insert(json_path![PATH[0], PATH[1], PATH[2]], op("true", 4, REPLACE));
            one.insert(json_path![PATH[0], PATH[1], PATH[2]], op("false", 3, REPLACE));
            file(&one, &SeqPatchMap::default())
        };
        let mut found = read(&json, "meshes/m.bin");

        let dir = std::env::temp_dir().join(format!("d_merge_read_user_patch_{}", std::process::id()));
        let patch = "<hkobject name=\"#0100\" class=\"hkbModifierList\" signature=\"0xa4180ca1\">\n<!-- MOD_CODE ~upatch~ OPEN -->\n\t<hkparam name=\"enable\">false</hkparam>\n<!-- ORIGINAL -->\n\t<hkparam name=\"enable\">true</hkparam>\n<!-- CLOSE -->\n</hkobject>\n";
        let file = dir.join("meshes/m.bin/#0100.txt");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, patch).unwrap();
        let user_patch = ScannedMod {
            id: USER_PATCH_ID.to_owned(),
            name: "User Patch".to_owned(),
            priority: 5,
            dir: Some(dir.display().to_string()),
        };
        found.read_user_patch(user_patch.clone());
        found.read_user_patch(user_patch); // again: still one value of it
        std::fs::remove_dir_all(&dir).unwrap();

        let values = &found.points[0].values;
        let ours: Vec<&Value> =
            values.iter().filter(|value| found.mods[value.owner as usize].id == USER_PATCH_ID).collect();
        let low = values.iter().find(|value| found.mods[value.owner as usize].id == "low").unwrap();
        assert_eq!(ours.len(), 1, "{values:?}");
        assert_eq!(ours[0].hash, low.hash, "low's value, false");
    }

    /// The module docs' example: `a` is decided in the user patch, `b` is still to do.
    #[test]
    fn what_the_user_patch_decides_is_resolved() {
        let mut mods = mods();
        mods.push(ScannedMod {
            id: USER_PATCH_ID.to_owned(),
            name: "User Patch".to_owned(),
            priority: 5,
            dir: Some("D:/out/user_patch".to_owned()),
        });
        let json = {
            let mut one = OnePatchMap::default();
            one.insert(json_path!["#1", "C", "a"], op("true", 4, REPLACE));
            one.insert(json_path!["#1", "C", "a"], op("false", 3, REPLACE));
            one.insert(json_path!["#1", "C", "a"], op("false", 5, REPLACE));
            one.insert(json_path!["#1", "C", "b"], op("1", 4, REPLACE));
            one.insert(json_path!["#1", "C", "b"], op("2", 3, REPLACE));
            file(&one, &SeqPatchMap::default())
        };
        let found = {
            let mut points = Vec::new();
            let owners = Owners::new(&mods);
            read_file("meshes/m.bin", &json, &owners, &mut points);
            let useful = owners.useful.into_inner();
            Conflicts { mods, points, useful }
        };
        let in_order = |id: &str| match id {
            "low" => Some(3),
            "high" => Some(4),
            USER_PATCH_ID => Some(5),
            _ => None,
        };

        let marks = found.marks(in_order, None);
        assert_eq!(marks["low"], Mark { wins: 0, loses: 1, resolved: 1 });
        assert_eq!(marks["high"], Mark { wins: 1, loses: 0, resolved: 1 });
        assert!(!marks.contains_key(USER_PATCH_ID), "it is not in the mod list");

        let of_low = found.relations("low", in_order, None);
        assert_eq!(of_low["high"], Relation { overridden_by_picked: 0, overrides_picked: 1, resolved: 1 });

        // The user patch off: `a` is a conflict again.
        let without = |id: &str| (id != USER_PATCH_ID).then(|| in_order(id)).flatten();
        assert_eq!(found.marks(without, None)["low"], Mark { wins: 0, loses: 2, resolved: 0 });
    }

    /// `enterNotifyEvents` of two mods can be decided at once, in the load order; one the user patch
    /// has is decided already, and a field of another kind is not offered.
    #[test]
    fn event_arrays_can_be_decided_at_once() {
        let mut mods = mods();
        mods.push(ScannedMod {
            id: USER_PATCH_ID.to_owned(),
            name: "User Patch".to_owned(),
            priority: 5,
            dir: Some("D:/out/user_patch".to_owned()),
        });
        let value = |owner, hash| Value { owner, hash, preview: Box::from("") };
        let point = |path: &str, values| Point { target: Box::from("meshes/m.bin"), path: Box::from(path), values };
        let found = Conflicts {
            mods,
            points: vec![
                point("#1821/hkbStateMachineStateInfo/enterNotifyEvents", vec![value(1, 2), value(0, 1)]),
                point("#1/C/a", vec![value(0, 1), value(1, 2)]),
                point("#1830/hkbStateMachineStateInfo/exitNotifyEvents", vec![value(0, 1), value(1, 2), value(2, 3)]),
            ],
            ..Conflicts::default()
        };
        let scan = ConflictScan {
            current: Some(Arc::new(found)),
            priorities: [("low", 3), ("high", 4), (USER_PATCH_ID, 5)]
                .map(|(id, at)| (id.to_owned(), at))
                .into_iter()
                .collect(),
            ..ConflictScan::default()
        };

        let bulk = scan.bulk_resolutions();
        let found: Vec<(&str, bool, Vec<&str>)> = bulk
            .iter()
            .map(|item| {
                let sides = item.request.sides.iter().map(|side| side.name.as_str()).collect();
                (item.request.path.as_str(), item.resolved, sides)
            })
            .collect();
        assert_eq!(
            found,
            [
                ("#1821/hkbStateMachineStateInfo/enterNotifyEvents", false, vec!["low", "high"]),
                ("#1830/hkbStateMachineStateInfo/exitNotifyEvents", true, vec!["low", "high"]),
            ]
        );
        assert!(bulk.iter().all(|item| item.request.keep_all_events && item.kind == BulkKind::KeepAllEvents));
        assert_eq!(bulk[0].request.mod_dir, "D:/mods/low");
    }

    /// MO2's lightning: all it does loses, and nothing of it is kept.
    #[test]
    fn a_mod_that_only_loses_does_nothing() {
        let priority = |id: &str| match id {
            "low" => Some(1),
            "high" => Some(2),
            _ => None,
        };
        let dead = |found: &Conflicts| {
            let marks = found.marks(priority, None);
            let mut dead: Vec<String> = found.dead_mods(&marks, priority, None).into_iter().collect();
            dead.sort();
            dead
        };
        let found = read(&behavior("false", "true"), "meshes/m.bin");
        assert_eq!(dead(&found), ["low"]);

        // The load order changes who does nothing.
        let swapped = |id: &str| priority(if id == "low" { "high" } else { "low" });
        let marks = found.marks(swapped, None);
        assert_eq!(found.dead_mods(&marks, swapped, None).into_iter().collect::<Vec<_>>(), ["high"]);

        let lost = || {
            let mut one = OnePatchMap::default();
            one.insert(json_path!["#1", "C", "a"], op("false", 4, REPLACE));
            one.insert(json_path!["#1", "C", "a"], op("true", 3, REPLACE));
            one
        };

        // It also changes a field nobody else does: it is of use.
        let mut more = lost();
        more.insert(json_path!["#1", "C", "b"], op("7", 3, REPLACE));
        assert!(dead(&read(&file(&more, &SeqPatchMap::default()), "meshes/m.bin")).is_empty());

        // It pushes to an array: a merge keeps that.
        let mut pushes = SeqPatchMap::default();
        pushes.insert(json_path!["#1", "C", "items"], op("[]", 3, Action::SeqPush));
        assert!(dead(&read(&file(&lost(), &pushes), "meshes/m.bin")).is_empty());
    }

    /// Hiding the winner's operation: the conflict is gone from the flags, and the mod that lost keeps
    /// the place, with no new scan. Only that mod's only there.
    #[test]
    fn a_hidden_operation_is_no_conflict() {
        let found = read(&behavior("false", "true"), "meshes/m.bin");
        let priority = |id: &str| match id {
            "low" => Some(1),
            "high" => Some(2),
            _ => None,
        };
        let mut hidden = HiddenPatches::default();

        let before = found.marks(priority, Some(&hidden));
        assert_eq!(before["high"], Mark { wins: 1, loses: 0, resolved: 0 });
        assert_eq!(before["low"], Mark { wins: 0, loses: 1, resolved: 0 });

        hidden.toggle("high", "meshes/m.bin", "#1/C/a");
        assert!(found.marks(priority, Some(&hidden)).is_empty());
        assert!(found.marks(priority, None).contains_key("high"), "`apply_user_patch` is off");

        hidden.toggle("high", "meshes/m.bin", "#1/C/a");
        assert_eq!(found.marks(priority, Some(&hidden))["high"], Mark { wins: 1, loses: 0, resolved: 0 });
    }

    #[test]
    fn the_winner_and_the_flags_follow_what_is_hidden() {
        let found = Arc::new(read(&behavior("false", "true"), "meshes/m.bin"));
        let item = |id: &str, priority| ModItem { id: id.into(), enabled: true, priority, ..Default::default() };
        let list = [item("low", 1), item("high", 2)];
        let mut scan = ConflictScan::with_hidden(HiddenPatches::default());
        scan.current = Some(Arc::clone(&found));
        scan.refresh_marks(&list, None, true);
        let point = &found.points[0];
        assert_eq!(scan.winner_of(&found, point).map(|winner| winner.id.as_str()), Some("high"));
        assert!(scan.is_contested(&found, point));

        // Not `toggle_hidden`: a test must not write the app's file.
        scan.hidden.toggle("high", &point.target, &point.path);
        scan.hidden_revision += 1;
        scan.refresh_marks(&list, None, true);
        assert_eq!(scan.winner_of(&found, point).map(|winner| winner.id.as_str()), Some("low"));
        assert!(!scan.is_contested(&found, point));
        assert_eq!(scan.mark_of("high"), Mark::default());
        assert_eq!(scan.mark_of("low"), Mark::default());

        // The switch is off: it counts as if nothing were hidden.
        scan.refresh_marks(&list, None, false);
        assert_eq!(scan.winner_of(&found, point).map(|winner| winner.id.as_str()), Some("high"));
        assert_eq!(scan.mark_of("high"), Mark { wins: 1, loses: 0, resolved: 0 });
    }

    fn read(json: &str, target: &str) -> Conflicts {
        let mods = mods();
        let mut points = Vec::new();
        let owners = Owners::new(&mods);
        read_file(target, json, &owners, &mut points);
        let useful = owners.useful.into_inner();
        Conflicts { mods, points, useful }
    }

    /// An operation of the mod of `priority`, its value written as JSON.
    fn op(value: &str, priority: usize, action: Action) -> ValueWithPriority<'static> {
        let mut bytes = value.as_bytes().to_vec();
        let value = simd_json::BorrowedValue::from(simd_json::to_owned_value(&mut bytes).unwrap());
        ValueWithPriority { patch: JsonPatch { action, value }, priority }
    }

    const REPLACE: Action = Action::Pure { op: Op::Replace };

    /// A behavior's file, written as the merge writes it: see `PatchJson`.
    fn file<'a>(one: &OnePatchMap<'a>, seq: &SeqPatchMap<'a>) -> String {
        sonic_rs::to_string(&PatchJson { one, seq, overridden: one.overridden() }).unwrap()
    }

    /// Two mods replace `#1/C/a`: `winner` (priority 4) and `loser` (3).
    fn behavior(winner: &str, loser: &str) -> String {
        let mut one = OnePatchMap::default();
        one.insert(json_path!["#1", "C", "a"], op(winner, 4, REPLACE));
        one.insert(json_path!["#1", "C", "a"], op(loser, 3, REPLACE));
        file(&one, &SeqPatchMap::default())
    }

    #[test]
    fn overridden_values_are_conflicts() {
        let found = read(&behavior("true", "false"), "meshes/x.bin");
        assert_eq!(found.points.len(), 1);
        assert_eq!(&*found.points[0].path, "#1/C/a");
        assert_eq!(found.points[0].values.len(), 2);
        assert_eq!(&*found.points[0].values[0].preview, "false");
    }

    #[test]
    fn the_same_value_is_no_conflict() {
        assert!(read(&behavior("1", "1"), "x").points.is_empty());
        assert!(read(&behavior(r#""A|B""#, r#""B|A""#), "x").points.is_empty(), "flags in another order");
    }

    #[test]
    fn marks_follow_the_load_order() {
        let found = read(&behavior("true", "false"), "x");
        let order = |low: usize, high: usize| {
            move |id: &str| match id {
                "low" => Some(low),
                "high" => Some(high),
                _ => None,
            }
        };
        let marks = found.marks(order(1, 2), None);
        assert_eq!(marks["high"].glyph(), "+");
        assert_eq!(marks["low"].glyph(), "-");

        // Moved in the list: no new scan needed.
        let marks = found.marks(order(2, 1), None);
        assert_eq!(marks["low"].glyph(), "+");
        assert_eq!(marks["high"].glyph(), "-");

        // Turned off: nothing to disagree with.
        assert!(found.marks(|id| (id == "low").then_some(1), None).is_empty());
    }

    #[test]
    fn a_mod_with_both_flags() {
        assert_eq!(Mark { wins: 1, loses: 2, resolved: 0 }.glyph(), "±");
        assert_eq!(Mark { wins: 0, loses: 0, resolved: 3 }.glyph(), "✓");
        assert_eq!(Mark { wins: 0, loses: 1, resolved: 3 }.glyph(), "-", "something is still to do");
        assert_eq!(Mark::default().glyph(), "");
    }

    /// Mods writing to ranges of `#1/C/items`: `(priority, range, op, the values as JSON)`.
    fn seq(ranges: &[(usize, std::ops::Range<usize>, Op, &str)]) -> String {
        let mut seq = SeqPatchMap::default();
        for (priority, range, kind, values) in ranges {
            let action = Action::Seq { op: *kind, range: range.clone() };
            seq.insert(json_path!["#1", "C", "items"], op(values, *priority, action));
        }
        file(&OnePatchMap::default(), &seq)
    }

    fn paths(found: &Conflicts) -> Vec<&str> {
        found.points.iter().map(|point| &*point.path).collect()
    }

    #[test]
    fn arrays_conflict_per_element() {
        // Overlapping on element 1, and writing different things there.
        let found = read(&seq(&[(3, 0..2, Op::Replace, r#"["a","b"]"#), (4, 1..3, Op::Replace, r#"["x","c"]"#)]), "x");
        assert_eq!(paths(&found), ["#1/C/items/[1:2]"]);
        assert_eq!(&*found.points[0].values[0].preview, "b");
        assert_eq!(&*found.points[0].values[1].preview, "x");

        // Overlapping, and writing the same there: the rest of the ranges do not matter.
        let same = read(&seq(&[(3, 0..2, Op::Replace, r#"["a","b"]"#), (4, 1..3, Op::Replace, r#"["b","c"]"#)]), "x");
        assert!(same.points.is_empty());

        // Next to each other: no shared element.
        let apart = read(&seq(&[(3, 0..2, Op::Replace, r#"["a","b"]"#), (4, 2..3, Op::Replace, r#"["c"]"#)]), "x");
        assert!(apart.points.is_empty());
    }

    #[test]
    fn elements_side_by_side_are_one_conflict() {
        let found = read(&seq(&[(3, 0..2, Op::Replace, r#"["a","b"]"#), (4, 0..2, Op::Replace, r#"["x","y"]"#)]), "x");
        assert_eq!(paths(&found), ["#1/C/items/[0:2]"]);
        assert_eq!(&*found.points[0].values[0].preview, "a\nb");
    }

    #[test]
    fn removing_is_a_write_of_its_own() {
        let found = read(&seq(&[(3, 1..2, Op::Remove, "[]"), (4, 1..2, Op::Replace, r#"["x"]"#)]), "x");
        assert_eq!(paths(&found), ["#1/C/items/[1:2]"]);
        assert_eq!(&*found.points[0].values[0].preview, REMOVED_ELEMENT);

        // A replace with too few values removes the rest, as the merge does.
        let short = read(&seq(&[(3, 0..2, Op::Replace, r#"["a"]"#), (4, 1..2, Op::Replace, r#"["x"]"#)]), "x");
        assert_eq!(paths(&short), ["#1/C/items/[1:2]"]);

        // Both remove it: they agree.
        assert!(read(&seq(&[(3, 1..2, Op::Remove, "[]"), (4, 0..2, Op::Remove, "[]")]), "x").points.is_empty());
    }

    #[test]
    fn adding_does_not_conflict() {
        assert!(read(&seq(&[(3, 0..2, Op::Add, r#"["a"]"#), (4, 0..2, Op::Add, r#"["b"]"#)]), "x").points.is_empty());
    }

    #[test]
    fn pushes_do_not_conflict() {
        let mut seq = SeqPatchMap::default();
        seq.insert(json_path!["#1", "C", "items"], op("[1]", 3, Action::SeqPush));
        seq.insert(json_path!["#1", "C", "items"], op("[2]", 4, Action::SeqPush));
        assert!(read(&file(&OnePatchMap::default(), &seq), "x").points.is_empty());
    }

    /// `animationdatasinglefile.patch.json` of `patches`, as the merge writes it: real `AdsfPatch`es,
    /// so that a rename there fails here until `anim_json` follows.
    fn anim_file(patches: &[AdsfPatch<'_>]) -> String {
        sonic_rs::to_string(patches).unwrap()
    }

    /// Mod `mod_name` sets the play back speed of clip `Dodge~644` of `D~1` to `speed`.
    fn edit_speed(mod_name: &str, speed: &str) -> String {
        let text =
            format!("Dodge\n644\n<!-- MOD_CODE ~m~ OPEN -->\n{speed}\n<!-- ORIGINAL -->\n1\n<!-- CLOSE -->\n0\n0\n0\n");
        let patch = parse_clip_anim_diff_patch(&text, 1).unwrap_or_else(|e| panic!("{e}"));
        let mod_dir = format!("D:\\mods\\{mod_name}");
        let edit = EditAnim { patch, priority: 1, name_clip: "Dodge~644" };
        anim_file(&[AdsfPatch { target: "D~1", id: &mod_dir, patch: PatchKind::EditAnim(edit) }])
    }

    /// The patches of two anim data files, as one list.
    fn both(a: &str, b: &str) -> String {
        format!(
            "[{},{}]",
            a.trim_start_matches('[').trim_end_matches(']'),
            b.trim_start_matches('[').trim_end_matches(']')
        )
    }

    #[test]
    fn anim_data_fields_are_compared_by_their_place() {
        let found =
            read(&both(&edit_speed("low", "1.1"), &edit_speed("high", "2")), "meshes/animationdatasinglefile.bin");
        assert_eq!(found.points.len(), 1);
        assert_eq!(&*found.points[0].path, "D~1/Dodge~644/play_back_speed");

        assert!(read(&both(&edit_speed("low", "1"), &edit_speed("high", "1")), "x").points.is_empty());
        assert!(read(&both(&edit_speed("low", "1"), &edit_speed("low", "2")), "x").points.is_empty(), "one mod");
    }

    #[test]
    fn added_clips_do_not_conflict() {
        let add = |mod_name: &str| {
            let mod_dir = format!("D:/mods/{mod_name}");
            let patch = PatchKind::AddAnim(ClipAnimDataBlock::default());
            anim_file(&[AdsfPatch { target: "D~1", id: &mod_dir, patch }])
        };
        assert!(read(&both(&add("low"), &add("high")), "x").points.is_empty());
    }

    /// On real mods (`PATCH_MODS`: a folder of mods; `PATCH_OUT`: a scratch directory): the merge writes
    /// the files and stops, then they are read. Prints what it found, and how long each step took.
    #[test]
    #[ignore = "needs real mods and the template files of the repository"]
    fn finds_the_conflicts_of_real_mods() {
        use std::time::Instant;

        let (mods_root, output) = (std::env::var("PATCH_MODS").unwrap(), std::env::var("PATCH_OUT").unwrap());
        let mut scanned = Vec::new();
        for mod_dir in std::fs::read_dir(&mods_root).unwrap().flatten().map(|entry| entry.path()) {
            let Ok(codes) = std::fs::read_dir(mod_dir.join("Nemesis_Engine").join("mod")) else { continue };
            for code in codes.flatten().map(|entry| entry.path()).filter(|path| path.is_dir()) {
                let dir = code.to_string_lossy().into_owned();
                let name = mod_dir.file_name().unwrap().to_string_lossy().into_owned();
                let priority = scanned.len();
                scanned.push(ScannedMod { id: dir.clone(), name, priority, dir: Some(dir) });
            }
        }
        let nemesis_entries = scanned.iter().map(|item| (item.id.clone(), item.priority)).collect();
        let patches = behavior_merge::PatchMaps { nemesis_entries, fnis_entries: Default::default() };
        let config = behavior_merge::Config {
            resource_dir: concat!(env!("CARGO_MANIFEST_DIR"), "/../../../resource/assets/templates").into(),
            output_dir: output.clone().into(),
            output_target: behavior_merge::OutPutTarget::SkyrimSe,
            status_report: None,
            parser_mode: behavior_merge::ParserMode::Lenient,
            hack_options: Some(behavior_merge::HackOptions::enable_all()),
            debug: behavior_merge::DebugOptions {
                output_patch_json: true,
                output_merged_json: false,
                output_merged_xml: false,
                patch_json_only: true,
            },
            skyrim_data_dir_glob: None,
            generate_fnis_esp: false,
            skeleton_arm_fix: false,
            hidden_patches: Default::default(),
        };
        let dir = debug_patches_dir(&output);
        let _no_old_files = std::fs::remove_dir_all(&dir);
        let start = Instant::now();
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        let result = runtime.block_on(behavior_merge::behavior_gen(patches, config));
        println!("{} mods, merge up to the files: {:?}, ok: {}", scanned.len(), start.elapsed(), result.is_ok());

        let start = Instant::now();
        let found = Conflicts::read(&dir, scanned);
        println!("{} conflict points, read in {:?}", found.points.len(), start.elapsed());
        let start = Instant::now();
        let priorities: HashMap<&str, usize> =
            found.mods.iter().map(|item| (item.id.as_str(), item.priority)).collect();
        let marks = found.marks(|id| priorities.get(id).copied(), None);
        println!("{} mods have flags, computed in {:?}", marks.len(), start.elapsed());
        for point in found.points.iter().step_by((found.points.len() / 12).max(1)).take(14) {
            println!("{} {}", point.target, point.path);
            for value in &point.values {
                let preview: String = value.preview.chars().take(60).collect();
                println!("    {}: {preview}", found.mods[value.owner as usize].name);
            }
        }
    }

    #[test]
    fn a_file_names_its_target() {
        let dir = Path::new("D:/out/patches");
        assert_eq!(
            target_of_file(dir, &dir.join("meshes/a/0_master.patch.json")).as_deref(),
            Some("meshes/a/0_master.bin")
        );
        assert_eq!(target_of_file(dir, &dir.join("meshes/a/notes.txt")), None);
    }
}
