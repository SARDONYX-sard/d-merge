//! Which mods change the same thing, like MO2's `+` and `-`.
//!
//! The merge knows: it writes its patches as JSON when asked (`DebugOptions::output_patch_json`),
//! and `patch_json_only` stops it there. This reads those files and keeps what several mods disagree
//! on, so changing the load order later only needs [`Conflicts::marks`], no new reading.
//!
//! ```text
//! <output>/.d_merge/.debug/patches/meshes/actors/character/behaviors/0_master.patch.json
//! {
//!   "one": { "#0106/hkbModifierList/enable": { "patch": { "action": .., "value": false }, "priority": 7 } },
//!   "overridden": { "#0106/hkbModifierList/enable": [ { "patch": { .. "value": true }, "priority": 3 } ] },
//!   "seq": { "#0106/hkbModifierList/modifiers": [ { "patch": { "action": { "kind": "seq_push" }, .. } } ] }
//! }
//! ```
//! - `one`, a single value: the highest priority wins, the others are in `overridden`.
//! - `seq`, arrays: where two mods write different things to the same element, see [`read_array`].
//!   `seq_push` and adding conflict with nothing: a merge keeps them all.
//!
//! `animationdatasinglefile` and `animationsetdatasinglefile` are lists of every mod's patches
//! (see [`read_anim_data`]), where each field that is not `null` is a change.
//!
//! An operation can be hidden, see [`HiddenPatches`]. The check does not leave it out of the files,
//! so hiding and showing it needs no new scan: the flags just stop counting it.

use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
};

use behavior_merge::HiddenPatches;
use fast_hash::HashMap;
use gui_shared::{patch::USER_PATCH_ID, settings::pages::patch::mod_item::ModItem};
use parking_lot::Mutex;
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Mark {
    pub(crate) wins: u32,
    pub(crate) loses: u32,
}

impl Mark {
    /// What the mod list shows.
    pub(crate) const fn glyph(self) -> &'static str {
        match (self.wins > 0, self.loses > 0) {
            (true, true) => "±",
            (true, false) => "+",
            (false, true) => "-",
            (false, false) => "",
        }
    }
}

/// What a scan found. See the module docs.
#[derive(Debug, Default)]
pub(crate) struct Conflicts {
    pub(crate) mods: Vec<ScannedMod>,
    pub(crate) points: Vec<Point>,
    /// Per mod of [`Self::mods`]: how many of its operations conflict with nothing, so that they are
    /// kept whatever the load order. See [`Self::dead_mods`].
    pub(crate) useful: Vec<u32>,
}

/// The directory the merge writes the JSON to.
pub(crate) fn debug_patches_dir(output: &str) -> PathBuf {
    Path::new(output).join(".d_merge").join(".debug").join("patches")
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
    /// `hidden` is as if the mod did not do it.
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
                mark.wins == 0 && mark.loses > 0 && !keeps_a_value.contains(id.as_str()) && useful_of(id) == 0
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

    /// The owner of `{ "patch": .., "priority": 3 }`.
    fn of_entry(&self, entry: &sonic_rs::Value) -> Option<u32> {
        let priority = usize::try_from(entry.get("priority")?.as_u64()?).ok()?;
        self.by_priority.get(&priority).copied()
    }
}

/// The same directory, however it was written. e.g. `D:\a/B` -> `d:/a/b`
fn dir_key(dir: &str) -> String {
    dir.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

fn read_file(target: &str, json: &str, owners: &Owners, points: &mut Vec<Point>) {
    let value: sonic_rs::Value = match sonic_rs::from_str(json) {
        Ok(value) => value,
        Err(err) => {
            tracing::error!("{target}: {err}");
            return;
        }
    };
    if value.is_array() {
        read_anim_data(target, &value, owners, points);
    } else {
        read_behavior(target, &value, owners, points);
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

fn value_of(owner: u32, value: &sonic_rs::Value) -> Value {
    let text = sonic_rs::to_string(value).unwrap_or_default();
    let preview: String = text.trim_matches('"').chars().take(PREVIEW_CHARS).collect();
    Value { owner, hash: value_hash(&text), preview: preview.into() }
}

/// The owner and value of `{ "patch": { "action": .., "value": .. }, "priority": 3 }`.
fn entry_value(entry: &sonic_rs::Value, owners: &Owners) -> Option<Value> {
    let priority = usize::try_from(entry.get("priority")?.as_u64()?).ok()?;
    let owner = *owners.by_priority.get(&priority)?;
    Some(value_of(owner, entry.get("patch")?.get("value")?))
}

/// See the module docs. Only what disagrees is kept.
fn read_behavior(target: &str, json: &sonic_rs::Value, owners: &Owners, points: &mut Vec<Point>) {
    if let (Some(one), Some(overridden)) =
        (json.get("one").and_then(|v| v.as_object()), json.get("overridden").and_then(|v| v.as_object()))
    {
        // `Object::get` walks the object: too slow with a `one` of many thousand entries.
        let one: HashMap<&str, &sonic_rs::Value> = one.iter().collect();
        let mut contested: fast_hash::HashSet<&str> = fast_hash::HashSet::default();
        for (path, losers) in overridden.iter() {
            contested.insert(path);
            let Some(losers) = losers.as_array() else { continue };
            let winner = one.get(path).and_then(|entry| entry_value(entry, owners));
            let values: Vec<Value> =
                winner.into_iter().chain(losers.iter().filter_map(|entry| entry_value(entry, owners))).collect();
            push_if_different(target, path, values, owners, points);
        }
        let only_one = one.iter().filter(|(path, _)| !contested.contains(*path));
        only_one.filter_map(|(_, entry)| owners.of_entry(entry)).for_each(|owner| owners.note_useful(owner));
    }
    let Some(seq) = json.get("seq").and_then(|v| v.as_object()) else { return };
    for (path, entries) in seq.iter() {
        let Some(entries) = entries.as_array() else { continue };
        let entries: Vec<&sonic_rs::Value> = entries.iter().collect();
        read_array(target, path, &entries, owners, points);
    }
}

/// What an element becomes when a mod removes it.
const REMOVED_ELEMENT: &str = "(removed)";

/// What one mod writes to one element of an array, by the index in the template.
type ElementWrites = BTreeMap<u64, Vec<(u32, String)>>;

/// Adds what `entry` writes, if it replaces or removes a range. See [`read_array`].
fn add_element_writes(entry: &sonic_rs::Value, owners: &Owners, writes: &mut ElementWrites) -> Option<()> {
    let patch = entry.get("patch")?;
    let action = patch.get("action")?;
    let op = action.get("op").and_then(|op| op.as_str());
    if action.get("kind")?.as_str()? != "seq" || !matches!(op, Some("replace" | "remove")) {
        return None;
    }
    let range = action.get("range")?;
    let (start, end) = (range.get("start")?.as_u64()?, range.get("end")?.as_u64()?);
    let priority = usize::try_from(entry.get("priority")?.as_u64()?).ok()?;
    let owner = *owners.by_priority.get(&priority)?;

    let values = patch.get("value").and_then(|value| value.as_array());
    for index in start..end {
        let offset = usize::try_from(index - start).ok()?;
        // A replace with fewer values than its range removes the rest.
        let value = if op == Some("replace") { values.and_then(|values| values.get(offset)) } else { None };
        let text =
            value.map_or_else(|| REMOVED_ELEMENT.to_owned(), |value| sonic_rs::to_string(value).unwrap_or_default());
        writes.entry(index).or_default().push((owner, text));
    }
    Some(())
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
fn read_array(target: &str, path: &str, entries: &[&sonic_rs::Value], owners: &Owners, points: &mut Vec<Point>) {
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
    start: u64,
    end: u64,
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

/// `animationdatasinglefile.patch.json`, `animationsetdatasinglefile.patch.json`: a list.
///
/// ```json
/// [ { "target": "DefaultMale~1", "id": "D:\\mods\\a\\Nemesis_Engine\\mod\\a",
///     "patch": { "EditAnim": { "name_clip": "Dodge~644", "priority": 39,
///       "patch": { "name": null, "play_back_speed": "1.1", "trigger_names_patches": { "one": {}, "seq": [] } } } } } ]
/// ```
/// Each field that is not `null` is a change at `<target>/<name>/<field>`. What adds is left out:
/// `AddAnim`, `AddMotion` and the `seq` lists, which are pushes; headers are lists merged by the merge.
fn read_anim_data(target: &str, json: &sonic_rs::Value, owners: &Owners, points: &mut Vec<Point>) {
    let mut places: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for entry in json.as_array().into_iter().flat_map(|list| list.iter()) {
        let (Some(project), Some(id), Some(patch)) = (
            entry.get("target").and_then(|v| v.as_str()),
            entry.get("id").and_then(|v| v.as_str()),
            entry.get("patch").and_then(|v| v.as_object()),
        ) else {
            continue;
        };
        let Some(&owner) = owners.by_dir.get(&dir_key(id)) else { continue };
        for (kind, body) in patch.iter() {
            let name = ["name_clip", "clip_id", "index", "file_name"]
                .iter()
                .find_map(|key| body.get(key).and_then(|v| v.as_str()));
            let Some(name) = name else {
                owners.note_useful(owner); // a header: a list merged by the merge
                continue;
            };
            let place = format!("{project}/{name}");
            match kind {
                "SpecifiedAnim" | "SpecifiedMotion" | "AddAnimSet" => {
                    if let Some(value) = body.get("patch") {
                        places.entry(place).or_default().push(value_of(owner, value));
                    }
                }
                "EditAnim" | "EditMotion" | "EditAnimSet" => {
                    if let Some(fields) = body.get("patch") {
                        flatten(&place, fields, owner, &mut places);
                    }
                }
                _ => owners.note_useful(owner), // what adds
            }
        }
    }
    for (path, values) in places {
        push_if_different(target, &path, values, owners, points);
    }
}

/// The leaves of a patch, each at `<place>/<field>/..`. See [`read_anim_data`].
fn flatten(place: &str, value: &sonic_rs::Value, owner: u32, places: &mut BTreeMap<String, Vec<Value>>) {
    if value.is_null() {
        return;
    }
    // `{ "patch": { "action": .., "value": .. }, "priority": 3 }`: one change.
    if let Some(inner) = value.get("patch").and_then(|patch| patch.get("value")) {
        places.entry(place.to_owned()).or_default().push(value_of(owner, inner));
        return;
    }
    if let Some(object) = value.as_object() {
        for (key, field) in object.iter() {
            if key != "seq" {
                flatten(&format!("{place}/{key}"), field, owner, places);
            }
        }
        return;
    }
    places.entry(place.to_owned()).or_default().push(value_of(owner, value));
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
        let mut hasher = DefaultHasher::new();
        for item in list.iter().filter(|item| item.enabled) {
            (&item.id, item.priority).hash(&mut hasher);
        }
        (user_patch, hidden_applies, self.hidden_revision).hash(&mut hasher);
        let hash = hasher.finish() | 1; // never the 0 of "computed again"
        if hash == self.marks_for {
            return;
        }
        self.marks_for = hash;
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

    pub(crate) fn is_selected(&self, point: &Point) -> bool {
        self.selected.as_ref().is_some_and(|(target, path)| *target == point.target && *path == point.path)
    }

    pub(crate) fn select(&mut self, point: &Point) {
        self.selected = Some((point.target.clone(), point.path.clone()));
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

        // It also changes a field nobody else does: it is of use.
        let more = format!(
            r##"{{"one":{{"#1/C/a":{},"#1/C/b":{}}},"overridden":{{"#1/C/a":[{}]}},"seq":{{}}}}"##,
            entry("false", 4, PURE),
            entry("7", 3, PURE),
            entry("true", 3, PURE)
        );
        assert!(dead(&read(&more, "meshes/m.bin")).is_empty());

        // It pushes to an array: a merge keeps that.
        let push = r#"{"kind":"seq_push"}"#;
        let pushes = format!(
            r##"{{"one":{{"#1/C/a":{}}},"overridden":{{"#1/C/a":[{}]}},"seq":{{"#1/C/items":[{}]}}}}"##,
            entry("false", 4, PURE),
            entry("true", 3, PURE),
            entry("[]", 3, push)
        );
        assert!(dead(&read(&pushes, "meshes/m.bin")).is_empty());
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
        assert_eq!(before["high"], Mark { wins: 1, loses: 0 });
        assert_eq!(before["low"], Mark { wins: 0, loses: 1 });

        hidden.toggle("high", "meshes/m.bin", "#1/C/a");
        assert!(found.marks(priority, Some(&hidden)).is_empty());
        assert!(found.marks(priority, None).contains_key("high"), "`apply_user_patch` is off");

        hidden.toggle("high", "meshes/m.bin", "#1/C/a");
        assert_eq!(found.marks(priority, Some(&hidden))["high"], Mark { wins: 1, loses: 0 });
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
        assert_eq!(scan.mark_of("high"), Mark { wins: 1, loses: 0 });
    }

    fn read(json: &str, target: &str) -> Conflicts {
        let mods = mods();
        let mut points = Vec::new();
        let owners = Owners::new(&mods);
        read_file(target, json, &owners, &mut points);
        let useful = owners.useful.into_inner();
        Conflicts { mods, points, useful }
    }

    fn entry(value: &str, priority: usize, action: &str) -> String {
        format!(r#"{{"patch":{{"action":{action},"value":{value}}},"priority":{priority}}}"#)
    }

    const PURE: &str = r#"{"kind":"pure","op":"replace"}"#;

    fn behavior(winner: &str, loser: &str) -> String {
        format!(
            r##"{{"one":{{"#1/C/a":{}}},"overridden":{{"#1/C/a":[{}]}},"seq":{{}}}}"##,
            entry(winner, 4, PURE),
            entry(loser, 3, PURE)
        )
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
        assert_eq!(Mark { wins: 1, loses: 2 }.glyph(), "±");
        assert_eq!(Mark::default().glyph(), "");
    }

    fn seq(ranges: &[(usize, u32, u32, &str, &str)]) -> String {
        let entries: Vec<String> = ranges
            .iter()
            .map(|(priority, start, end, op, value)| {
                let action = format!(r#"{{"kind":"seq","op":"{op}","range":{{"start":{start},"end":{end}}}}}"#);
                entry(value, *priority, &action)
            })
            .collect();
        format!(r##"{{"one":{{}},"seq":{{"#1/C/items":[{}]}}}}"##, entries.join(","))
    }

    fn paths(found: &Conflicts) -> Vec<&str> {
        found.points.iter().map(|point| &*point.path).collect()
    }

    #[test]
    fn arrays_conflict_per_element() {
        // Overlapping on element 1, and writing different things there.
        let found = read(&seq(&[(3, 0, 2, "replace", r#"["a","b"]"#), (4, 1, 3, "replace", r#"["x","c"]"#)]), "x");
        assert_eq!(paths(&found), ["#1/C/items/[1:2]"]);
        assert_eq!(&*found.points[0].values[0].preview, "b");
        assert_eq!(&*found.points[0].values[1].preview, "x");

        // Overlapping, and writing the same there: the rest of the ranges do not matter.
        let same = read(&seq(&[(3, 0, 2, "replace", r#"["a","b"]"#), (4, 1, 3, "replace", r#"["b","c"]"#)]), "x");
        assert!(same.points.is_empty());

        // Next to each other: no shared element.
        let apart = read(&seq(&[(3, 0, 2, "replace", r#"["a","b"]"#), (4, 2, 3, "replace", r#"["c"]"#)]), "x");
        assert!(apart.points.is_empty());
    }

    #[test]
    fn elements_side_by_side_are_one_conflict() {
        let found = read(&seq(&[(3, 0, 2, "replace", r#"["a","b"]"#), (4, 0, 2, "replace", r#"["x","y"]"#)]), "x");
        assert_eq!(paths(&found), ["#1/C/items/[0:2]"]);
        assert_eq!(&*found.points[0].values[0].preview, "a\nb");
    }

    #[test]
    fn removing_is_a_write_of_its_own() {
        let found = read(&seq(&[(3, 1, 2, "remove", "[]"), (4, 1, 2, "replace", r#"["x"]"#)]), "x");
        assert_eq!(paths(&found), ["#1/C/items/[1:2]"]);
        assert_eq!(&*found.points[0].values[0].preview, REMOVED_ELEMENT);

        // A replace with too few values removes the rest, as the merge does.
        let short = read(&seq(&[(3, 0, 2, "replace", r#"["a"]"#), (4, 1, 2, "replace", r#"["x"]"#)]), "x");
        assert_eq!(paths(&short), ["#1/C/items/[1:2]"]);

        // Both remove it: they agree.
        assert!(read(&seq(&[(3, 1, 2, "remove", "[]"), (4, 0, 2, "remove", "[]")]), "x").points.is_empty());
    }

    #[test]
    fn adding_does_not_conflict() {
        assert!(read(&seq(&[(3, 0, 2, "add", r#"["a"]"#), (4, 0, 2, "add", r#"["b"]"#)]), "x").points.is_empty());
    }

    #[test]
    fn pushes_do_not_conflict() {
        let push = entry("[1]", 3, r#"{"kind":"seq_push"}"#);
        let other = entry("[2]", 4, r#"{"kind":"seq_push"}"#);
        let json = format!(r##"{{"one":{{}},"seq":{{"#1/C/items":[{push},{other}]}}}}"##);
        assert!(read(&json, "x").points.is_empty());
    }

    #[test]
    fn anim_data_fields_are_compared_by_their_place() {
        let edit = |id: &str, speed: &str| {
            format!(
                r#"{{"target":"D~1","id":"D:\\mods\\{id}","patch":{{"EditAnim":{{"name_clip":"Dodge~644","priority":1,"patch":{{"name":null,"play_back_speed":"{speed}","trigger_names_patches":{{"one":{{}},"seq":[]}}}}}}}}}}"#
            )
        };
        let found =
            read(&format!("[{},{}]", edit("low", "1.1"), edit("high", "2")), "meshes/animationdatasinglefile.bin");
        assert_eq!(found.points.len(), 1);
        assert_eq!(&*found.points[0].path, "D~1/Dodge~644/play_back_speed");

        assert!(read(&format!("[{},{}]", edit("low", "1"), edit("high", "1")), "x").points.is_empty());
        assert!(read(&format!("[{},{}]", edit("low", "1"), edit("low", "2")), "x").points.is_empty(), "one mod");
    }

    #[test]
    fn added_clips_do_not_conflict() {
        let add = |id: &str| format!(r#"{{"target":"D~1","id":"D:/mods/{id}","patch":{{"AddAnim":{{"name":"x"}}}}}}"#);
        assert!(read(&format!("[{},{}]", add("low"), add("high")), "x").points.is_empty());
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
