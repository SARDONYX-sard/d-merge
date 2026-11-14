//! Nemesis behavior patches from a template xml and an edited copy of it.
//!
//! A Nemesis patch mod has, per behavior (e.g. `draugrbehavior/`):
//! - `#0106.txt`: a template object with the changed lines marked
//!   ```text
//!   <!-- MOD_CODE ~mymod~ OPEN -->
//!   (new lines)
//!   <!-- ORIGINAL -->
//!   (template lines; absent for a pure insertion)
//!   <!-- CLOSE -->
//!   ```
//! - `#mymod$0.txt`: a new object, as is
//!
//! # Why the template xml (not any hkx)
//! Object ids (`#0106`) are the identity Nemesis matches on. Re-serializing an hkx numbers objects
//! differently, so the edited file must be a copy of the template xml (`resource/xml/templates`).
//!
//! # Normalization
//! d-merge's templates keep pointer arrays on one line (`#0105 #0109 #0107`), Nemesis' on one
//! pointer per line. Both sides are split into one pointer per line ([`normalize`]) before
//! diffing, so that each pointer is its own patch line (as Nemesis expects).

pub mod asdsf;
pub mod conflict;
mod format;
mod kept_array;
pub mod schema;
pub mod vars;

use std::{borrow::Cow, fmt::Write as _, ops::Range};

use fast_hash::{HashMap, HashSet};

pub use self::{
    format::format_object,
    kept_array::{is_in_kept_array, open_kept_arrays},
};

/// One top-level `<hkobject name="#0106" class="..">` of a behavior xml.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlObject<'a> {
    /// e.g. `#0106`
    pub id: &'a str,
    /// e.g. `hkbModifierList`
    pub class: &'a str,
    /// From the `<hkobject` line to its `</hkobject>` line, each line ending with `\n`.
    pub text: &'a str,
    /// Byte range of `text` in the whole xml.
    pub range: Range<usize>,
}

/// The top-level objects of `xml` (the ones with a `name`), in file order.
pub fn objects(xml: &str) -> Vec<XmlObject<'_>> {
    let mut found = Vec::new();
    let mut offset = 0;
    let mut open: Option<(usize, &str, &str, &str)> = None; // (start, indent, id, class)

    for line in xml.split_inclusive('\n') {
        let start = offset;
        offset += line.len();
        let trimmed = line.trim_start();
        let indent = &line[..line.len() - trimmed.len()];

        match open {
            None => {
                if trimmed.starts_with("<hkobject ")
                    && let Some(id) = attribute(trimmed, "name")
                {
                    let class = attribute(trimmed, "class").unwrap_or_default();
                    open = Some((start, indent, id, class));
                }
            }
            Some((begin, open_indent, id, class)) => {
                if indent == open_indent && trimmed.trim_end() == "</hkobject>" {
                    found.push(XmlObject { id, class, text: &xml[begin..offset], range: begin..offset });
                    open = None;
                }
            }
        }
    }
    found
}

/// `name="#0106"` -> `#0106`
pub(crate) fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let (at, _) = tag.match_indices(name).find(|(at, _)| tag[at + name.len()..].starts_with("=\""))?;
    let start = at + name.len() + 2;
    let len = tag[start..].find('"')?;
    Some(&tag[start..start + len])
}

/// Splits lines holding only pointers into one pointer per line (same indentation), and takes out
/// what the templates do not have. Most texts already are so, and stay borrowed.
///
/// e.g. `\t\t\t\t#0105 #0109 #0107\n` -> `\t\t\t\t#0105\n\t\t\t\t#0109\n\t\t\t\t#0107\n`
pub fn normalize(text: &str) -> Cow<'_, str> {
    if !needs_normalizing(text) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        let trimmed = content.trim_start();
        if is_ignored_comment(trimmed) {
            continue;
        }
        if has_several_pointers(trimmed) {
            let indent = &content[..content.len() - trimmed.len()];
            for token in trimmed.split_whitespace() {
                let _ = writeln!(out, "{indent}{token}");
            }
        } else {
            out.push_str(content);
            out.push('\n');
        }
    }
    Cow::Owned(out)
}

/// Does [`normalize`] change `text`?
fn needs_normalizing(text: &str) -> bool {
    if !text.is_empty() && !text.ends_with('\n') {
        return true;
    }
    text.split_inclusive('\n').any(|line| {
        let content = line.trim_end_matches('\n');
        let trimmed = content.trim_start();
        content.ends_with('\r') || is_ignored_comment(trimmed) || has_several_pointers(trimmed)
    })
}

/// serde-hkx's `<!-- referenceCount SERIALIZE_IGNORED -->`: not in Nemesis' templates.
fn is_ignored_comment(trimmed: &str) -> bool {
    trimmed.starts_with("<!--") && trimmed.ends_with("SERIALIZE_IGNORED -->")
}

/// e.g. `#0105 #0109`
fn has_several_pointers(trimmed: &str) -> bool {
    let mut tokens = trimmed.split_whitespace();
    let is_pointer = |token: &str| token.starts_with('#');
    tokens.next().is_some_and(is_pointer) && tokens.next().is_some_and(is_pointer) && tokens.all(is_pointer)
}

const fn open_marker(mod_code: &str) -> MarkerOpen<'_> {
    MarkerOpen(mod_code)
}

struct MarkerOpen<'a>(&'a str);

impl std::fmt::Display for MarkerOpen<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<!-- MOD_CODE ~{}~ OPEN -->", self.0)
    }
}

const ORIGINAL: &str = "<!-- ORIGINAL -->";
const CLOSE: &str = "<!-- CLOSE -->";

/// The `#XXXX.txt` contents for an edited template object, or `None` when nothing changed.
///
/// Both sides are [`normalize`]d first.
pub fn object_patch(mod_code: &str, template: &str, edited: &str) -> Option<String> {
    let before = normalize(template);
    let after = align_numelements(&before, &normalize(edited));
    if before == after {
        return None;
    }
    let lines = Lines::new(&before, &after);
    Some(write_patch(mod_code, &lines, &patch_hunks(&lines)))
}

/// The lines of the two [`normalize`]d sides of a patch.
struct Lines<'a> {
    before_text: &'a str,
    after_text: &'a str,
    before: Vec<&'a str>,
    after: Vec<&'a str>,
}

impl<'a> Lines<'a> {
    fn new(before: &'a str, after: &'a str) -> Self {
        Self {
            before_text: before,
            after_text: after,
            before: before.split_inclusive('\n').collect(),
            after: after.split_inclusive('\n').collect(),
        }
    }
}

/// The hunks a patch is written from.
///
/// The histogram diff gives the shortest blocks, but on the many alike lines of an array of objects
/// it may pair an added element with the lines of the next field, e.g. a push to `variableInfos`
/// becomes a block taking out `</hkparam>` and `<hkparam name="characterPropertyInfos" ...>`, which
/// the parser rejects (`TK Dodge RE`, `#0087`). Only then (a block whose tags don't nest, and a patch
/// the parser rejects) the other algorithms are tried, and the first that parses is taken.
fn patch_hunks(lines: &Lines<'_>) -> Vec<Hunk> {
    use imara_diff::Algorithm;
    let hunks = line_hunks(lines.before_text, lines.after_text);
    let crosses = |hunk: &Hunk| !nests(&lines.before[hunk.old.clone()]) || !nests(&lines.after[hunk.new.clone()]);
    if !hunks.iter().any(crosses) {
        return hunks;
    }
    let parses = |hunks: &[Hunk]| validate(&write_patch("m", lines, hunks)).is_ok();
    if parses(&hunks) {
        return hunks; // e.g. the push to an empty array, which keeps its line: see `kept_array`
    }
    [Algorithm::Myers, Algorithm::MyersMinimal]
        .into_iter()
        .map(|algorithm| line_hunks_by(algorithm, lines.before_text, lines.after_text))
        .find(|other| parses(other))
        .unwrap_or(hunks)
}

/// Do the tags of `lines` close only what they open, and close all of it?
///
/// e.g. `<hkobject>`, `<hkparam name="a">1</hkparam>`, `</hkobject>`: yes. `</hkparam>`, `<hkparam ...>`: no.
fn nests(lines: &[&str]) -> bool {
    let mut depth = 0_i32;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("</") {
            depth -= 1;
        } else if trimmed.starts_with('<')
            && !trimmed.starts_with("<!--")
            && !trimmed.ends_with("/>")
            && !trimmed.contains("</")
        {
            depth += 1;
        }
        if depth < 0 {
            return false;
        }
    }
    depth == 0
}

/// The patch of `hunks`. See [`object_patch`].
fn write_patch(mod_code: &str, lines: &Lines<'_>, hunks: &[Hunk]) -> String {
    let (before_lines, after_lines) = (&lines.before, &lines.after);
    let mut out = String::with_capacity(lines.after_text.len() + 128);
    let mut at = 0; // next unwritten `before` line
    for Hunk { old, new } in hunks.iter().cloned() {
        before_lines[at..old.start].iter().for_each(|line| out.push_str(line));

        let _ = writeln!(out, "{}", open_marker(mod_code));
        if new.is_empty() {
            // Nemesis' way of deleting: a blank line for each line taken out. See `is_deletion_mark`.
            old.clone().for_each(|_| out.push('\n'));
        }
        after_lines[new].iter().for_each(|line| out.push_str(line));
        if !old.is_empty() {
            let _ = writeln!(out, "{ORIGINAL}");
            before_lines[old.clone()].iter().for_each(|line| out.push_str(line));
        }
        let _ = writeln!(out, "{CLOSE}");
        at = old.end;
    }
    before_lines[at..].iter().for_each(|line| out.push_str(line));
    out
}

/// One changed block: lines of the template (`old`) replaced by lines of the edit (`new`).
/// Line indices are of the [`normalize`]d texts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub old: Range<usize>,
    pub new: Range<usize>,
}

/// Line diff of two already [`normalize`]d texts.
#[inline]
fn line_hunks(before: &str, after: &str) -> Vec<Hunk> {
    line_hunks_by(imara_diff::Algorithm::Histogram, before, after)
}

fn line_hunks_by(algorithm: imara_diff::Algorithm, before: &str, after: &str) -> Vec<Hunk> {
    use imara_diff::{Diff, InternedInput};
    let input = InternedInput::new(before, after);
    let mut diff = Diff::compute(algorithm, &input);
    diff.postprocess_lines(&input);
    diff.hunks()
        .map(|h| Hunk {
            old: h.before.start as usize..h.before.end as usize,
            new: h.after.start as usize..h.after.end as usize,
        })
        .collect()
}

/// The changed lines between two texts as they are written (not [`normalize`]d), for showing the
/// changes on the text being edited: the line numbers are those of `new`.
///
/// e.g., `a\nb\n` -> `a\nc\nd\n`: one hunk, `old` = `1..2` (`b`), `new` = `1..3` (`c`, `d`).
#[inline]
pub fn line_changes(old: &str, new: &str) -> Vec<Hunk> {
    line_hunks(old, new)
}

/// The changed blocks of an edited object (the `OPEN` blocks of its patch, in order).
#[inline]
pub fn hunks(template: &str, edited: &str) -> Vec<Hunk> {
    let before = normalize(template);
    let after = align_numelements(&before, &normalize(edited));
    patch_hunks(&Lines::new(&before, &after))
}

/// `edited` ([`normalize`]d) with hunk `index` put back to the template's lines.
pub fn revert_hunk(template: &str, edited: &str, index: usize) -> String {
    let before = normalize(template);
    let after = align_numelements(&before, &normalize(edited));
    let lines = Lines::new(&before, &after);
    let Some(Hunk { old, new }) = patch_hunks(&lines).into_iter().nth(index) else {
        return after;
    };
    let mut out = String::with_capacity(after.len());
    lines.after[..new.start].iter().for_each(|line| out.push_str(line));
    lines.before[old].iter().for_each(|line| out.push_str(line));
    lines.after[new.end..].iter().for_each(|line| out.push_str(line));
    out
}

/// `edited` with the `<hkparam ... numelements=` lines as a patch has to write them. The k-th such line
/// of a name matches the template's k-th.
///
/// - The count goes back to the template's value: Nemesis / d-merge count the elements themselves, and
///   a changed `<hkparam ...>` line would become a patch block where the parsers do not accept one.
/// - An array that is empty in the template keeps its line, however its elements were written, so
///   that the patch only adds them instead of replacing the array: see [`kept_array`].
pub(crate) fn align_numelements(template: &str, edited: &str) -> String {
    // field name -> its `numelements` values in template order, and the line if it is an empty array
    let mut template_counts: HashMap<&str, Vec<(&str, Option<&str>)>> = HashMap::default();
    for line in template.lines() {
        if let (Some(name), Some(count)) = (counted_field(line), count_of(line)) {
            let empty = line.trim_end().ends_with("\"0\"></hkparam>").then_some(line);
            template_counts.entry(name).or_default().push((count, empty));
        }
    }

    let lines: Vec<&str> = edited.split_inclusive('\n').collect();
    let mut seen: HashMap<&str, usize> = HashMap::default();
    let mut out = String::with_capacity(edited.len());
    let mut at = 0;
    while at < lines.len() {
        let line = lines[at];
        at += 1;
        let template_field = counted_field(line).and_then(|name| {
            let k = seen.entry(name).or_default();
            let found = template_counts.get(name)?.get(*k)?;
            *k += 1;
            Some(found)
        });
        let Some(&(original, empty)) = template_field else {
            out.push_str(line);
            continue;
        };
        if let Some(kept) = empty
            && let Some(skip) = keep_empty_array_line(kept, line, &lines[at..], &mut out)
        {
            at += skip;
            continue;
        }
        match count_of(line).filter(|current| *current != original) {
            Some(current) => {
                let (before, after) = line.split_at(line.find(NUMELEMENTS).unwrap_or_default() + NUMELEMENTS.len());
                out.push_str(before);
                out.push_str(original);
                out.push_str(&after[current.len()..]);
            }
            None => out.push_str(line),
        }
    }
    out
}

const NUMELEMENTS: &str = " numelements=\"";

/// `<hkparam name="x" numelements="2">` -> `x`
fn counted_field(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix("<hkparam name=\"")?;
    line.contains(NUMELEMENTS).then(|| rest.split('"').next().unwrap_or_default())
}

/// `<hkparam name="x" numelements="2">` -> `2`
fn count_of(line: &str) -> Option<&str> {
    let at = line.find(NUMELEMENTS)? + NUMELEMENTS.len();
    line[at..].split('"').next()
}

/// Writes an array that is empty in the template the way that keeps its line (`kept`). Returns how
/// many lines of `rest` it took care of. `None`: it did not write anything: this is that line, or the
/// array does not close yet.
///
/// e.g., for `numelements="0"></hkparam>`
/// - `numelements="0">`, `<hkcstring>a</hkcstring>`, `</hkparam>`: the line, and the rest as it is
/// - `numelements="0">`, `</hkparam>`: the line alone, since nothing is added
/// - `numelements="0"><hkcstring>a</hkcstring></hkparam>`: the line, the element, `</hkparam>`
fn keep_empty_array_line(kept: &str, line: &str, rest: &[&str], out: &mut String) -> Option<usize> {
    let indent = &kept[..kept.len() - kept.trim_start().len()];
    if line.contains("</hkparam>") {
        let (_, value) = line.split_once('>')?;
        let (value, _) = value.rsplit_once("</hkparam>")?;
        let value = value.trim();
        if value.is_empty() {
            return None;
        }
        let _ = writeln!(out, "{kept}\n{indent}\t{value}\n{indent}</hkparam>");
        return Some(0);
    }
    let end = kept_array::array_end(rest)?;
    let _ = writeln!(out, "{kept}");
    Some(if end.has_elements { 0 } else { end.close + 1 })
}

/// What a patch file means: the edited object (`Mod`) or the template object (`Original`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// `OPEN` sections kept, `ORIGINAL` sections dropped.
    Mod,
    /// `ORIGINAL` sections kept, `OPEN` sections dropped.
    Original,
}

/// Removes the markers of a patch file, keeping the `view` side.
///
/// e.g. [`View::Mod`] of a `#0106.txt` is the object as the mod changes it.
pub fn resolve(patch: &str, view: View) -> Cow<'_, str> {
    // Without a comment, there is no marker to take out.
    if !patch.contains("<!--") && (patch.is_empty() || patch.ends_with('\n')) {
        return Cow::Borrowed(patch);
    }
    #[derive(PartialEq)]
    enum Section {
        Common,
        New,
        Old,
    }
    let mut section = Section::Common;
    let mut out = String::with_capacity(patch.len());
    for line in patch.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with("<!-- MOD_CODE ~") && trimmed.ends_with("OPEN -->") {
            section = Section::New;
        } else if trimmed == ORIGINAL {
            section = Section::Old;
        } else if trimmed == CLOSE {
            section = Section::Common;
        } else {
            let keep = match section {
                Section::Common => true,
                Section::New => view == View::Mod && !is_deletion_mark(line),
                Section::Old => view == View::Original,
            };
            if keep {
                out.push_str(line);
                if !line.ends_with('\n') {
                    out.push('\n');
                }
            }
        }
    }
    Cow::Owned(out)
}

/// A blank line in the `OPEN` section of a block: Nemesis' mark of a deleted line, not a line of the
/// object. It must not be read as one, or writing the patch again makes a block that adds blank lines.
///
/// e.g., this block deletes the two lines of its `ORIGINAL`:
/// ```text
/// <!-- MOD_CODE ~mymod~ OPEN -->
/// (blank)
/// (blank)
/// <!-- ORIGINAL -->
///     <hkparam name="a">1</hkparam>
///     <hkparam name="b">2</hkparam>
/// <!-- CLOSE -->
/// ```
fn is_deletion_mark(line: &str) -> bool {
    line.trim().is_empty()
}

/// A patch file's block: `old` lines from line `at` of what the patch says the template is, replaced
/// by `new`.
struct PatchBlock<'a> {
    at: usize,
    old: usize,
    new: Vec<&'a str>,
}

fn patch_blocks(patch: &str) -> Vec<PatchBlock<'_>> {
    let (mut blocks, mut at) = (Vec::new(), 0);
    let mut current: Option<(PatchBlock<'_>, bool)> = None; // (the block, in its `ORIGINAL`)
    for line in patch.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<!-- MOD_CODE ~") && trimmed.ends_with("OPEN -->") {
            current = Some((PatchBlock { at, old: 0, new: Vec::new() }, false));
        } else if trimmed == ORIGINAL {
            if let Some((_, in_original)) = &mut current {
                *in_original = true;
            }
        } else if trimmed == CLOSE {
            blocks.extend(current.take().map(|(block, _)| block));
        } else {
            match &mut current {
                Some((_, false)) if is_deletion_mark(line) => {}
                Some((block, false)) => block.new.push(line),
                Some((block, true)) => {
                    block.old += 1;
                    at += 1;
                }
                None => at += 1,
            }
        }
    }
    blocks
}

/// Where lines `at..at + len` of `old` are in `new`, whose differences are `hunks`. `None`: some of them
/// are not there as they are.
fn place_of_lines(hunks: &[Hunk], at: usize, len: usize) -> Option<Range<usize>> {
    let mut shift = 0_isize;
    for hunk in hunks {
        if hunk.old.end <= at {
            shift = hunk.new.end.cast_signed() - hunk.old.end.cast_signed();
        } else if hunk.old.start >= at + len {
            break;
        } else {
            return None;
        }
    }
    let start = at.saturating_add_signed(shift);
    Some(start..start + len)
}

/// What a patch file makes of `template`: its blocks, and nothing else.
///
/// A mod's file is a copy of the object as its author had it, which may not be the template: d-merge
/// and Nemesis only read the blocks, so the lines around them do not matter, and must not become
/// changes when the patch is written again.
///
/// e.g., a file of `a`, `b`, a block changing `c` to `C`, `stale` on the template `a`, `b`, `c`, `d`
/// -> `a`, `b`, `C`, `d`
///
/// A block is only placed where the template has what it replaces. If one is not, the lines do not
/// compare, e.g. the file has the values of a vector one per line, and the template all on one line:
/// the result is the file as it is, whose form the block fits.
pub fn apply_to_template(patch: &str, template: &str) -> String {
    let patch = normalize(patch);
    let template = normalize(template);
    let base = resolve(&patch, View::Original);
    if base == template {
        return resolve(&patch, View::Mod).into_owned();
    }
    let hunks = line_hunks(&base, &template);
    let blocks = patch_blocks(&patch);
    let places: Option<Vec<Range<usize>>> =
        blocks.iter().map(|block| place_of_lines(&hunks, block.at, block.old)).collect();
    let Some(places) = places else { return resolve(&patch, View::Mod).into_owned() };

    let lines: Vec<&str> = template.split_inclusive('\n').collect();
    let mut out = String::with_capacity(template.len() + patch.len());
    let mut done = 0; // The lines of the template that were written or replaced.
    for (block, place) in blocks.iter().zip(places) {
        lines[done..place.start].iter().for_each(|line| out.push_str(line));
        for line in &block.new {
            out.push_str(line);
            out.push('\n');
        }
        done = place.end;
    }
    lines[done..].iter().for_each(|line| out.push_str(line));
    out
}

/// A template object the edited copy changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedObject {
    pub id: String,
    pub class: String,
    /// `#XXXX.txt` contents.
    pub patch: String,
    /// Number of `OPEN` blocks.
    pub hunks: usize,
}

/// An object the template does not have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedObject {
    /// e.g. `#mymod$0` (the file is `#mymod$0.txt`)
    pub id: String,
    /// Its id in the edited copy, if it was renamed (e.g. `#9001`).
    pub renamed_from: Option<String>,
    pub class: String,
    pub text: String,
}

/// All changes of an edited copy against its template.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BehaviorDiff {
    pub changed: Vec<ChangedObject>,
    pub added: Vec<AddedObject>,
    /// Template objects missing from the edited copy. Nemesis cannot delete objects: the mod
    /// should unlink them instead.
    pub removed: Vec<String>,
}

impl BehaviorDiff {
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.added.is_empty()
    }
}

/// Compares the edited copy with its template.
///
/// Objects the template does not have get Nemesis ids (`#mymod$0`, `#mymod$1`, ...), and every
/// reference to them is renamed too. Ids already in that form are kept.
pub fn diff_behavior(mod_code: &str, template: &str, edited: &str) -> BehaviorDiff {
    let template_objects: HashMap<&str, XmlObject<'_>> = objects(template).into_iter().map(|o| (o.id, o)).collect();
    let edited_objects = objects(edited);

    let renames = new_ids(mod_code, &template_objects, &edited_objects);

    let mut diff = BehaviorDiff::default();
    let mut seen: HashSet<&str> = HashSet::default();
    for object in &edited_objects {
        seen.insert(object.id);
        if let Some(original) = template_objects.get(object.id) {
            if let Some(patch) = object_patch(mod_code, original.text, &renamed(object.text, &renames)) {
                let hunks = patch.matches(CLOSE).count();
                diff.changed.push(ChangedObject {
                    id: object.id.to_string(),
                    class: object.class.to_string(),
                    patch,
                    hunks,
                });
            }
        } else {
            let id = renames.get(object.id).cloned().unwrap_or_else(|| object.id.to_string());
            diff.added.push(AddedObject {
                renamed_from: (id != object.id).then(|| object.id.to_string()),
                id,
                class: object.class.to_string(),
                text: normalize(&renamed(object.text, &renames)).into_owned(),
            });
        }
    }
    diff.removed = objects(template).iter().filter(|o| !seen.contains(o.id)).map(|o| o.id.to_string()).collect();
    diff
}

/// New object ids -> `#mymod$N`, numbered after the highest `$N` already used.
fn new_ids(
    mod_code: &str,
    template: &HashMap<&str, XmlObject<'_>>,
    edited: &[XmlObject<'_>],
) -> HashMap<String, String> {
    let prefix = format!("#{mod_code}$");
    let first =
        edited.iter().filter_map(|o| o.id.strip_prefix(&prefix)?.parse::<usize>().ok()).max().map_or(0, |n| n + 1);
    edited
        .iter()
        .filter(|o| !template.contains_key(o.id) && !o.id.starts_with(&prefix))
        .zip(first..)
        .map(|(object, n)| (object.id.to_string(), format!("{prefix}{n}")))
        .collect()
}

/// Replaces whole pointer tokens (`#0109`, not the `#01` in it).
/// `text` with its pointers to new objects renamed. Without new objects, it stays borrowed.
fn renamed<'a>(text: &'a str, renames: &HashMap<String, String>) -> Cow<'a, str> {
    if renames.is_empty() { Cow::Borrowed(text) } else { Cow::Owned(rename_pointers(text, renames)) }
}

fn rename_pointers(text: &str, renames: &HashMap<String, String>) -> String {
    let is_id_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('#') {
        out.push_str(&rest[..at]);
        let token_len = rest[at + 1..].find(|c: char| !is_id_char(c)).unwrap_or(rest.len() - at - 1);
        let token = &rest[at..at + 1 + token_len];
        out.push_str(renames.get(token).map_or(token, String::as_str));
        rest = &rest[at + 1 + token_len..];
    }
    out.push_str(rest);
    out
}

/// Applies a mod's patch files to the template: the edited copy to keep working on.
///
/// `patches`: `(file stem, contents)`, e.g. `("#0106", "..")`, `("#mymod$0", "..")`.
/// Changed objects are replaced by their [`View::Mod`]; new objects are appended to the section.
pub fn apply_patches<'a>(template: &str, patches: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    let objects = objects(template);
    let mut replaced: HashMap<&str, Cow<'a, str>> = HashMap::default();
    let mut appended = String::new();
    for (stem, contents) in patches {
        let resolved = resolve(contents, View::Mod);
        if objects.iter().any(|o| o.id == stem) {
            replaced.insert(stem, resolved);
        } else {
            appended.push_str(&resolved);
        }
    }

    let mut out = String::with_capacity(template.len() + appended.len());
    let mut at = 0;
    for object in &objects {
        if let Some(text) = replaced.get(object.id) {
            out.push_str(&template[at..object.range.start]);
            out.push_str(text);
            at = object.range.end;
        }
    }
    let rest = &template[at..];
    // New objects go on the line before `</hksection>` (or at the end without one).
    let insert_at =
        rest.rfind("</hksection>").and_then(|end| rest[..end].rfind('\n').map(|nl| nl + 1)).unwrap_or(rest.len());
    out.push_str(&rest[..insert_at]);
    out.push_str(&appended);
    out.push_str(&rest[insert_at..]);
    out
}

/// Checks that d-merge's Nemesis parser accepts a patch file (as Nemesis would).
///
/// # Errors
/// The parse error, readable.
pub fn validate(patch: &str) -> Result<(), String> {
    // The push to an empty array that keeps its line: see `kept_array`.
    let hacks = nemesis_xml::hack::HackOptions { empty_array_outside_hkparam: true, ..Default::default() };
    nemesis_xml::patch::parse_nemesis_patch(patch, Some(hacks)).map(|_| ()).map_err(|err| err.to_string())
}

/// A class or field name the Havok class table does not know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldProblem {
    /// 0-based line in the checked text.
    pub line: usize,
    pub message: String,
}

/// Checks the class and field names of an object (and of its nested classless `<hkobject>`s)
/// against d-merge's class schema, e.g. a typo like `<hkparam name="modifers">`.
pub fn check_fields(object: &str) -> Vec<FieldProblem> {
    use nemesis_xml::patch::class_schema::{ClassSchema, TypeKind, find_class_schema};

    let object = open_kept_arrays(object);
    let mut problems = Vec::new();
    // Open objects: (class name, its schema if known).
    let mut objects: Vec<(&str, Option<&'static ClassSchema>)> = Vec::new();
    // Open fields: the class of their nested `<hkobject>`, if they hold objects.
    let mut fields: Vec<Option<&'static str>> = Vec::new();

    for (line_no, line) in object.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("<hkobject") {
            let class = attribute(rest, "class").or_else(|| fields.last().copied().flatten());
            let info = class.and_then(find_class_schema);
            if let Some(class) = class.filter(|_| info.is_none()) {
                problems.push(FieldProblem { line: line_no, message: format!("Unknown class `{class}`.") });
            }
            if let Some(info) = info.filter(|_| schema::line_with_signature(line).is_some()) {
                let message = format!("The signature of `{}` is `{}`.", info.name, info.signature);
                problems.push(FieldProblem { line: line_no, message });
            }
            objects.push((class.unwrap_or("?"), info));
            if trimmed.ends_with("/>") {
                objects.pop();
            }
        } else if trimmed.starts_with("</hkobject>") {
            objects.pop();
        } else if let Some(rest) = trimmed.strip_prefix("<hkparam") {
            let name = attribute(rest, "name").unwrap_or_default();
            let field = match objects.last() {
                Some((class, Some(info))) => {
                    let field = info.field(name);
                    if field.is_none() {
                        problems
                            .push(FieldProblem { line: line_no, message: format!("`{class}` has no field `{name}`.") });
                    }
                    field
                }
                _ => None,
            };
            // A struct, or an array of structs: their `<hkobject>`s are of `field.class`.
            let nested =
                field.filter(|f| f.kind == TypeKind::Struct || f.sub_kind == TypeKind::Struct).and_then(|f| f.class);
            if !trimmed.contains("</hkparam>") && !trimmed.ends_with("/>") {
                fields.push(nested);
            }
        } else if trimmed.starts_with("</hkparam>") {
            fields.pop();
        }
    }
    problems
}

/// Can a code be used of a mod that already exists? Any ASCII letters/digits, not `vanilla`.
///
/// Mods don't keep to what [`is_valid_mod_code`] asks of a new one, and Nemesis itself ships with
/// `nemesis`. e.g. `nemesis`, `throwws`: yes. `ab-cd`, an empty one: no.
#[inline]
pub fn is_existing_mod_code(code: &str) -> bool {
    !code.is_empty() && code.chars().all(|c| c.is_ascii_alphanumeric()) && !code.eq_ignore_ascii_case("vanilla")
}

/// Is `code` valid for a **new** Nemesis mod? (4-6 ASCII letters/digits, not `vanilla`)
///
/// The rule of the Nemesis Sub-tool, which makes patches (`T2/main.cpp`).
#[inline]
pub fn is_valid_mod_code(code: &str) -> bool {
    (4..=6).contains(&code.len())
        && code.chars().all(|c| c.is_ascii_alphanumeric())
        && !code.eq_ignore_ascii_case("vanilla")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: &str = "<?xml version=\"1.0\" encoding=\"ascii\"?>
<hkpackfile classversion=\"8\" contentsversion=\"hk_2010.2.0-r1\" toplevelobject=\"#0091\">
\t<hksection name=\"__data__\">
\t\t<hkobject name=\"#0106\" class=\"hkbModifierList\" signature=\"0xa4180ca1\">
\t\t\t<hkparam name=\"variableBindingSet\">null</hkparam>
\t\t\t<hkparam name=\"userData\">1</hkparam>
\t\t\t<hkparam name=\"name\">GetUpModList</hkparam>
\t\t\t<hkparam name=\"enable\">true</hkparam>
\t\t\t<hkparam name=\"modifiers\" numelements=\"3\">
\t\t\t\t#0105 #0107 #0104
\t\t\t</hkparam>
\t\t</hkobject>
\t\t<hkobject name=\"#0115\" class=\"hkbBoneWeightArray\" signature=\"0xcd902b77\">
\t\t\t<hkparam name=\"variableBindingSet\">null</hkparam>
\t\t\t<hkparam name=\"boneWeights\" numelements=\"0\"></hkparam>
\t\t</hkobject>
\t</hksection>
</hkpackfile>
";

    #[test]
    fn finds_objects() {
        let found = objects(TEMPLATE);
        assert_eq!(
            found.iter().map(|o| (o.id, o.class)).collect::<Vec<_>>(),
            [("#0106", "hkbModifierList"), ("#0115", "hkbBoneWeightArray")]
        );
        assert!(found[1].text.ends_with("\t\t</hkobject>\n"));
    }

    #[test]
    fn patch_marks_changed_pointer_only() {
        let edited = TEMPLATE.replace("#0105 #0107 #0104", "#0105 #9000 #0104").replace(
            "\t</hksection>",
            "\t\t<hkobject name=\"#9000\" class=\"hkbBoneWeightArray\" signature=\"0xcd902b77\">\n\t\t\t<hkparam name=\"variableBindingSet\">null</hkparam>\n\t\t</hkobject>\n\t</hksection>",
        );
        let diff = diff_behavior("mymod", TEMPLATE, &edited);

        assert_eq!(diff.changed.len(), 1);
        let patch = &diff.changed[0].patch;
        assert!(patch.contains(
            "\t\t\t\t#0105\n<!-- MOD_CODE ~mymod~ OPEN -->\n\t\t\t\t#mymod$0\n<!-- ORIGINAL -->\n\t\t\t\t#0107\n<!-- CLOSE -->\n\t\t\t\t#0104\n"
        ), "{patch}");
        assert_eq!(resolve(patch, View::Original), normalize(objects(TEMPLATE)[0].text));

        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].id, "#mymod$0");
        assert_eq!(diff.added[0].renamed_from.as_deref(), Some("#9000"));
        assert!(diff.added[0].text.starts_with("\t\t<hkobject name=\"#mymod$0\""));
        assert!(diff.removed.is_empty());

        validate(patch).unwrap();
    }

    #[test]
    fn apply_then_diff_round_trips() {
        let edited =
            TEMPLATE.replace("<hkparam name=\"userData\">1</hkparam>", "<hkparam name=\"userData\">2</hkparam>");
        let diff = diff_behavior("mymod", TEMPLATE, &edited);
        let patch = &diff.changed[0].patch;

        let applied = apply_patches(TEMPLATE, [("#0106", patch.as_str())]);
        assert_eq!(diff_behavior("mymod", TEMPLATE, &applied), diff);
    }

    #[test]
    fn revert_one_hunk() {
        let template = objects(TEMPLATE)[0].text;
        let edited = template
            .replace("<hkparam name=\"userData\">1</hkparam>", "<hkparam name=\"userData\">2</hkparam>")
            .replace("<hkparam name=\"enable\">true</hkparam>", "<hkparam name=\"enable\">false</hkparam>");
        assert_eq!(hunks(template, &edited).len(), 2);

        let reverted = revert_hunk(template, &edited, 0);
        assert_eq!(hunks(template, &reverted).len(), 1);
        assert!(reverted.contains("userData\">1<") && reverted.contains("enable\">false<"));
    }

    #[test]
    fn field_names() {
        assert!(check_fields(objects(TEMPLATE)[0].text).is_empty());
        let typo = objects(TEMPLATE)[0].text.replace("\"modifiers\"", "\"modifers\"");
        let problems = check_fields(&typo);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].line, 5);
        assert!(problems[0].message.contains("modifers"));
    }

    #[test]
    fn numelements_is_not_a_change() {
        let template = objects(TEMPLATE)[0].text;
        // The modder bumped the count while adding a pointer.
        let edited = template
            .replace("numelements=\"3\"", "numelements=\"4\"")
            .replace("#0105 #0107 #0104", "#0105 #0107 #0104 #0200");
        let patch = object_patch("mymod", template, &edited).unwrap();
        assert!(patch.contains("<hkparam name=\"modifiers\" numelements=\"3\">"), "{patch}");
        assert_eq!(patch.matches("<!-- CLOSE -->").count(), 1, "{patch}");
        validate(&patch).unwrap();
    }

    const EMPTY_ARRAY: &str = "\t\t<hkobject name=\"#0106\" class=\"hkbCharacterStringData\" signature=\"0x655b42bc\">\n\
                               \t\t\t<hkparam name=\"characterPropertyNames\" numelements=\"0\"></hkparam>\n\
                               \t\t</hkobject>\n";
    const EMPTY_LINE: &str = "numelements=\"0\"></hkparam>\n";

    /// What every way of writing the elements of `EMPTY_ARRAY` has to come to.
    const PUSH: &str = "\t\t<hkobject name=\"#0106\" class=\"hkbCharacterStringData\" signature=\"0x655b42bc\">\n\
                        \t\t\t<hkparam name=\"characterPropertyNames\" numelements=\"0\"></hkparam>\n\
                        <!-- MOD_CODE ~mymod~ OPEN -->\n\
                        \t\t\t\t<hkcstring>Hi</hkcstring>\n\
                        \t\t\t</hkparam>\n\
                        <!-- CLOSE -->\n\
                        \t\t</hkobject>\n";

    #[test]
    fn a_push_to_an_empty_array_keeps_its_line() {
        for written in [
            // Nemesis' way.
            "numelements=\"0\"></hkparam>\n\t\t\t\t<hkcstring>Hi</hkcstring>\n\t\t\t</hkparam>\n",
            // The XML way, whatever it counts.
            "numelements=\"0\">\n\t\t\t\t<hkcstring>Hi</hkcstring>\n\t\t\t</hkparam>\n",
            "numelements=\"1\">\n\t\t\t\t<hkcstring>Hi</hkcstring>\n\t\t\t</hkparam>\n",
            // On one line.
            "numelements=\"1\"><hkcstring>Hi</hkcstring></hkparam>\n",
        ] {
            let edited = EMPTY_ARRAY.replace(EMPTY_LINE, written);
            assert_eq!(object_patch("mymod", EMPTY_ARRAY, &edited).as_deref(), Some(PUSH), "{written}");
        }
    }

    #[test]
    fn a_push_to_an_empty_array_passes_the_check_and_is_a_push() {
        validate(PUSH).unwrap();
        let hacks = nemesis_xml::hack::HackOptions { empty_array_outside_hkparam: true, ..Default::default() };
        let (patches, _) = nemesis_xml::patch::parse_nemesis_patch(PUSH, Some(hacks)).unwrap();
        assert!(format!("{patches:?}").contains("SeqPush"), "several mods can add to it: {patches:?}");
    }

    #[test]
    fn a_push_to_an_empty_array_survives_reading_the_patch_back() {
        let edited = resolve(PUSH, View::Mod);
        assert_eq!(object_patch("mymod", EMPTY_ARRAY, &edited).as_deref(), Some(PUSH));
    }

    #[test]
    fn an_empty_array_with_nothing_in_it_is_not_a_change() {
        for written in
            [EMPTY_LINE, "numelements=\"0\">\n\t\t\t</hkparam>\n", "numelements=\"0\">\n\t\t\t\t\n\t\t\t</hkparam>\n"]
        {
            let edited = EMPTY_ARRAY.replace(EMPTY_LINE, written);
            assert_eq!(object_patch("mymod", EMPTY_ARRAY, &edited), None, "{written}");
        }
    }

    /// Its elements must not be told apart from a typo in the field names.
    #[test]
    fn a_kept_line_has_no_problems_in_its_fields() {
        let edited = resolve(PUSH, View::Mod);
        assert_eq!(check_fields(&edited), []);
    }

    fn patch_of(common: &str, block: &str, rest: &str) -> String {
        format!("{common}<!-- MOD_CODE ~m~ OPEN -->\n{block}<!-- CLOSE -->\n{rest}")
    }

    /// `Draugr Dagger Animation Fix`: a copy of other data than the template around its one change.
    #[test]
    fn only_the_blocks_of_a_patch_change_the_template() {
        let template = "a\nb\nc\nd\ne\nf\n";
        let patch = patch_of("a\nb\n", "C\n<!-- ORIGINAL -->\nc\n", "d\nstale\nf\n");
        assert_eq!(apply_to_template(&patch, template), "a\nb\nC\nd\ne\nf\n");
    }

    #[test]
    fn a_block_goes_to_its_place_in_a_template_of_another_length() {
        // A line is missing before the block, and two more are after it.
        let template = "a\nnew\nb\nc\nd\ne\nf\n";
        let patch = patch_of("a\nb\n", "C\n<!-- ORIGINAL -->\nc\n", "d\n");
        assert_eq!(apply_to_template(&patch, template), "a\nnew\nb\nC\nd\ne\nf\n");
    }

    /// `Dodge MCO-DXP`: the values of a vector one per line, and all on one line in the template.
    #[test]
    fn a_block_that_the_template_has_no_place_for_keeps_the_file() {
        let template = "a\n(0 1)\nb\n";
        let patch = patch_of("a\n", "2\n<!-- ORIGINAL -->\n0\n", "1\nb\n");
        assert_eq!(apply_to_template(&patch, template), "a\n2\n1\nb\n");
    }

    #[test]
    fn a_block_that_only_adds_goes_to_its_place() {
        let template = "a\nb\nc\nd\n";
        let patch = patch_of("a\nb\n", "new\n", "c\nstale\n");
        assert_eq!(apply_to_template(&patch, template), "a\nb\nnew\nc\nd\n");
    }

    /// Writing the patch again gives what the file had.
    #[test]
    fn a_patch_on_its_own_template_stays_the_same() {
        let template = "a\nb\nc\nd\n";
        let patch = patch_of("a\nb\n", "C\n<!-- ORIGINAL -->\nc\n", "d\n");
        let edited = apply_to_template(&patch, template);
        assert_eq!(edited, "a\nb\nC\nd\n");
        assert_eq!(object_patch("m", template, &edited).as_deref(), Some(patch.as_str()));
    }

    /// `Combat Gameplay Overhaul` (`#0521`): blank lines in a block delete the lines of its `ORIGINAL`.
    ///
    /// e.g., `a`, [`b`, `c` -> 2 blank lines], `d` -> `a`, `d`
    #[test]
    fn blank_lines_of_a_block_delete() {
        let template = "a\nb\nc\nd\n";
        let patch = patch_of("a\n", "\n\n<!-- ORIGINAL -->\nb\nc\n", "d\n");
        assert_eq!(resolve(&patch, View::Mod), "a\nd\n");
        assert_eq!(apply_to_template(&patch, template), "a\nd\n");
        // Before its fix the blank lines were read as lines, and written again as a block adding them.
        assert_eq!(object_patch("m", template, "a\nd\n").as_deref(), Some(patch.as_str()));
    }

    #[test]
    fn a_block_of_whole_elements_nests() {
        assert!(nests(&["<hkobject>", "<hkparam name=\"a\">1</hkparam>", "<hkparam name=\"b\"/>", "</hkobject>"]));
        assert!(nests(&["<!-- comment -->", "#0105"]));
        assert!(!nests(&["</hkparam>", "<hkparam name=\"b\" numelements=\"1\">"]));
        assert!(!nests(&["<hkobject>"]));
    }

    /// A deletion written again passes d-merge's parser.
    #[test]
    fn a_deleted_array_element_passes_the_check() {
        let template = "\t\t<hkobject name=\"#0521\" class=\"hkbClipTriggerArray\" signature=\"0x59c23a0f\">\n\
                        \t\t\t<hkparam name=\"triggers\" numelements=\"2\">\n\
                        \t\t\t\t<hkobject>\n\t\t\t\t\t<hkparam name=\"localTime\">0.000000</hkparam>\n\t\t\t\t</hkobject>\n\
                        \t\t\t\t<hkobject>\n\t\t\t\t\t<hkparam name=\"localTime\">1.000000</hkparam>\n\t\t\t\t</hkobject>\n\
                        \t\t\t</hkparam>\n\
                        \t\t</hkobject>\n";
        let edited = template.replace(
            "\t\t\t\t<hkobject>\n\t\t\t\t\t<hkparam name=\"localTime\">1.000000</hkparam>\n\t\t\t\t</hkobject>\n",
            "",
        );
        let patch = object_patch("mymod", template, &edited).unwrap();
        assert!(patch.contains("OPEN -->\n\n\n\n<!-- ORIGINAL -->"), "{patch}");
        validate(&patch).unwrap();
    }

    #[test]
    fn mod_codes() {
        assert!(is_valid_mod_code("colisc"));
        assert!(!is_valid_mod_code("abc"));
        assert!(!is_valid_mod_code("vanilla"));
        assert!(!is_valid_mod_code("ab-cd"));
    }

    #[test]
    fn existing_mods_have_longer_codes() {
        assert!(!is_valid_mod_code("nemesis"));
        assert!(is_existing_mod_code("nemesis"));
        assert!(is_existing_mod_code("throwws"));
        assert!(is_existing_mod_code("abc"));
        assert!(!is_existing_mod_code(""));
        assert!(!is_existing_mod_code("VANILLA"));
        assert!(!is_existing_mod_code("ab-cd"));
    }
}
