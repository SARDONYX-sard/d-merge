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
pub mod vars;

use std::{fmt::Write as _, ops::Range};

use fast_hash::{HashMap, HashSet};

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
                    found.push(XmlObject {
                        id,
                        class,
                        text: &xml[begin..offset],
                        range: begin..offset,
                    });
                    open = None;
                }
            }
        }
    }
    found
}

/// `name="#0106"` -> `#0106`
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let start = tag.find(&key)? + key.len();
    let len = tag[start..].find('"')?;
    Some(&tag[start..start + len])
}

/// Splits lines holding only pointers into one pointer per line (same indentation).
///
/// e.g. `\t\t\t\t#0105 #0109 #0107\n` -> `\t\t\t\t#0105\n\t\t\t\t#0109\n\t\t\t\t#0107\n`
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        let trimmed = content.trim_start();
        // serde-hkx's `<!-- referenceCount SERIALIZE_IGNORED -->`: not in Nemesis' templates.
        if trimmed.starts_with("<!--") && trimmed.ends_with("SERIALIZE_IGNORED -->") {
            continue;
        }
        let indent = &content[..content.len() - trimmed.len()];
        let mut tokens = trimmed.split_whitespace().peekable();
        let only_pointers =
            tokens.peek().is_some() && trimmed.split_whitespace().all(|t| t.starts_with('#'));

        if only_pointers && trimmed.split_whitespace().nth(1).is_some() {
            for token in tokens {
                let _ = writeln!(out, "{indent}{token}");
            }
        } else {
            out.push_str(content);
            out.push('\n');
        }
    }
    out
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
    let after = normalize(edited);
    if before == after {
        return None;
    }
    let before_lines: Vec<&str> = before.split_inclusive('\n').collect();
    let after_lines: Vec<&str> = after.split_inclusive('\n').collect();

    let mut out = String::with_capacity(after.len() + 128);
    let mut at = 0; // next unwritten `before` line
    for Hunk { old, new } in line_hunks(&before, &after) {
        before_lines[at..old.start].iter().for_each(|line| out.push_str(line));

        let _ = writeln!(out, "{}", open_marker(mod_code));
        after_lines[new].iter().for_each(|line| out.push_str(line));
        if !old.is_empty() {
            let _ = writeln!(out, "{ORIGINAL}");
            before_lines[old.clone()].iter().for_each(|line| out.push_str(line));
        }
        let _ = writeln!(out, "{CLOSE}");
        at = old.end;
    }
    before_lines[at..].iter().for_each(|line| out.push_str(line));
    Some(out)
}

/// One changed block: lines of the template (`old`) replaced by lines of the edit (`new`).
/// Line indices are of the [`normalize`]d texts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub old: Range<usize>,
    pub new: Range<usize>,
}

/// Line diff of two already [`normalize`]d texts.
fn line_hunks(before: &str, after: &str) -> Vec<Hunk> {
    use imara_diff::{Algorithm, Diff, InternedInput};
    let input = InternedInput::new(before, after);
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    diff.postprocess_lines(&input);
    diff.hunks()
        .map(|h| Hunk {
            old: h.before.start as usize..h.before.end as usize,
            new: h.after.start as usize..h.after.end as usize,
        })
        .collect()
}

/// The changed blocks of an edited object (the `OPEN` blocks of its patch, in order).
pub fn hunks(template: &str, edited: &str) -> Vec<Hunk> {
    line_hunks(&normalize(template), &normalize(edited))
}

/// `edited` ([`normalize`]d) with hunk `index` put back to the template's lines.
pub fn revert_hunk(template: &str, edited: &str, index: usize) -> String {
    let (before, after) = (normalize(template), normalize(edited));
    let Some(Hunk { old, new }) = line_hunks(&before, &after).into_iter().nth(index) else {
        return after;
    };
    let before_lines: Vec<&str> = before.split_inclusive('\n').collect();
    let after_lines: Vec<&str> = after.split_inclusive('\n').collect();
    let mut out = String::with_capacity(after.len());
    after_lines[..new.start].iter().for_each(|line| out.push_str(line));
    before_lines[old].iter().for_each(|line| out.push_str(line));
    after_lines[new.end..].iter().for_each(|line| out.push_str(line));
    out
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
pub fn resolve(patch: &str, view: View) -> String {
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
                Section::New => view == View::Mod,
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
    pub const fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.added.is_empty()
    }
}

/// Compares the edited copy with its template.
///
/// Objects the template does not have get Nemesis ids (`#mymod$0`, `#mymod$1`, ...), and every
/// reference to them is renamed too. Ids already in that form are kept.
pub fn diff_behavior(mod_code: &str, template: &str, edited: &str) -> BehaviorDiff {
    let template_objects: HashMap<&str, XmlObject<'_>> =
        objects(template).into_iter().map(|o| (o.id, o)).collect();
    let edited_objects = objects(edited);

    let renames = new_ids(mod_code, &template_objects, &edited_objects);
    let rename = |text: &str| {
        if renames.is_empty() { text.to_string() } else { rename_pointers(text, &renames) }
    };

    let mut diff = BehaviorDiff::default();
    let mut seen: HashSet<&str> = HashSet::default();
    for object in &edited_objects {
        seen.insert(object.id);
        if let Some(original) = template_objects.get(object.id) {
            if let Some(patch) = object_patch(mod_code, original.text, &rename(object.text)) {
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
                text: normalize(&rename(object.text)),
            });
        }
    }
    diff.removed = objects(template)
        .iter()
        .filter(|o| !seen.contains(o.id))
        .map(|o| o.id.to_string())
        .collect();
    diff
}

/// New object ids -> `#mymod$N`, numbered after the highest `$N` already used.
fn new_ids(
    mod_code: &str,
    template: &HashMap<&str, XmlObject<'_>>,
    edited: &[XmlObject<'_>],
) -> HashMap<String, String> {
    let prefix = format!("#{mod_code}$");
    let first = edited
        .iter()
        .filter_map(|o| o.id.strip_prefix(&prefix)?.parse::<usize>().ok())
        .max()
        .map_or(0, |n| n + 1);
    edited
        .iter()
        .filter(|o| !template.contains_key(o.id) && !o.id.starts_with(&prefix))
        .zip(first..)
        .map(|(object, n)| (object.id.to_string(), format!("{prefix}{n}")))
        .collect()
}

/// Replaces whole pointer tokens (`#0109`, not the `#01` in it).
fn rename_pointers(text: &str, renames: &HashMap<String, String>) -> String {
    let is_id_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('#') {
        out.push_str(&rest[..at]);
        let token_len =
            rest[at + 1..].find(|c: char| !is_id_char(c)).unwrap_or(rest.len() - at - 1);
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
pub fn apply_patches<'a>(
    template: &str,
    patches: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> String {
    let objects = objects(template);
    let mut replaced: HashMap<&str, String> = HashMap::default();
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
    let insert_at = rest
        .rfind("</hksection>")
        .and_then(|end| rest[..end].rfind('\n').map(|nl| nl + 1))
        .unwrap_or(rest.len());
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
    nemesis_xml::patch::parse_nemesis_patch(patch, None).map(|_| ()).map_err(|err| err.to_string())
}

/// A class or field name the Havok class table does not know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldProblem {
    /// 0-based line in the checked text.
    pub line: usize,
    pub message: String,
}

/// Checks the class and field names of an object (and of its nested classless `<hkobject>`s)
/// against d-merge's class table, e.g. a typo like `<hkparam name="modifers">`.
pub fn check_fields(object: &str) -> Vec<FieldProblem> {
    use nemesis_xml::patch::class_table::{FieldInfo, find_class_info};

    let mut problems = Vec::new();
    // Open objects: (class name, its fields if known).
    let mut objects: Vec<(&str, Option<&'static FieldInfo>)> = Vec::new();
    // Open fields: the class of their nested `<hkobject>`, if they hold objects.
    let mut fields: Vec<Option<&'static str>> = Vec::new();

    for (line_no, line) in object.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("<hkobject") {
            let class = attribute(rest, "class").or_else(|| fields.last().copied().flatten());
            let info = class.and_then(find_class_info);
            if let Some(class) = class.filter(|_| info.is_none()) {
                problems.push(FieldProblem {
                    line: line_no,
                    message: format!("Unknown class `{class}`."),
                });
            }
            objects.push((class.unwrap_or("?"), info));
            if trimmed.ends_with("/>") {
                objects.pop();
            }
        } else if trimmed.starts_with("</hkobject>") {
            objects.pop();
        } else if let Some(rest) = trimmed.strip_prefix("<hkparam") {
            let name = attribute(rest, "name").unwrap_or_default();
            let kind = match objects.last() {
                Some((class, Some(info))) => {
                    let kind = info.get(name).copied();
                    if kind.is_none() {
                        problems.push(FieldProblem {
                            line: line_no,
                            message: format!("`{class}` has no field `{name}`."),
                        });
                    }
                    kind
                }
                _ => None,
            };
            // `Object|hkbEventProperty`, `Array|Object|hkbStateMachineTransitionInfo`
            let nested = kind.and_then(|k| k.trim_start_matches("Array|").strip_prefix("Object|"));
            if !trimmed.contains("</hkparam>") && !trimmed.ends_with("/>") {
                fields.push(nested);
            }
        } else if trimmed.starts_with("</hkparam>") {
            fields.pop();
        }
    }
    problems
}

/// The class (and its class table fields) of the innermost `<hkobject>` open at `line`.
///
/// `line` is 0-based in an object text. A classless nested `<hkobject>` gets the class from
/// its field's type (`Object|hkbEventProperty`).
pub fn field_context(
    object: &str,
    line: usize,
) -> Option<(String, Vec<(&'static str, &'static str)>)> {
    use nemesis_xml::patch::class_table::find_class_info;
    let mut objects: Vec<Option<&str>> = Vec::new();
    let mut fields: Vec<Option<&str>> = Vec::new();
    let current_fields =
        |class: Option<&str>| -> Option<&'static nemesis_xml::patch::class_table::FieldInfo> {
            class.and_then(find_class_info)
        };
    for text in object.lines().take(line) {
        let trimmed = text.trim();
        if let Some(rest) = trimmed.strip_prefix("<hkobject") {
            let class = attribute(rest, "class").or_else(|| fields.last().copied().flatten());
            objects.push(class);
            if trimmed.ends_with("/>") {
                objects.pop();
            }
        } else if trimmed.starts_with("</hkobject>") {
            objects.pop();
        } else if let Some(rest) = trimmed.strip_prefix("<hkparam") {
            if !trimmed.contains("</hkparam>") && !trimmed.ends_with("/>") {
                let name = attribute(rest, "name").unwrap_or_default();
                let kind = current_fields(objects.last().copied().flatten())
                    .and_then(|info| info.get(name).copied());
                fields.push(
                    kind.and_then(|k| k.trim_start_matches("Array|").strip_prefix("Object|")),
                );
            }
        } else if trimmed.starts_with("</hkparam>") {
            fields.pop();
        }
    }
    let class = objects.last().copied().flatten()?;
    let info = find_class_info(class)?;
    Some((class.to_string(), info.entries().map(|(name, kind)| (*name, *kind)).collect()))
}

/// `<hkparam name="name">default</hkparam>` for a field of type `kind` (see [`skeleton`]);
/// nested lines are indented for a field at `depth` tabs. No trailing newline.
pub fn field_snippet(name: &str, kind: &str, depth: usize) -> String {
    let mut out = String::new();
    field_skeleton(&mut out, depth, name, kind, 0);
    out.trim_start_matches('\t').trim_end().to_string()
}

/// Every class name of d-merge's class table (alphabetical).
pub fn class_names() -> impl Iterator<Item = &'static str> {
    nemesis_xml::patch::class_table::class_names()
}

/// A new object `id` of `class` with every field of the class table at a default value
/// (`null`, `0`, `false`, empty array, nested objects filled the same way).
///
/// `like`: a template object of the same class, if any. Its `signature` and its set of fields
/// are used (Havok XML leaves out the serialize-ignored ones such as `referenceCount`).
pub fn skeleton(class: &str, id: &str, like: Option<&str>) -> Option<String> {
    use nemesis_xml::patch::class_table::find_class_info;
    let fields = find_class_info(class)?;
    let signature = like.and_then(|text| attribute(text, "signature")).unwrap_or("");
    // Fields `like` has at its top level (3 tabs), or all but the usual ignored ones.
    let keep = |name: &str| match like {
        Some(text) => text.contains(&format!("\n\t\t\t<hkparam name=\"{name}\"")),
        None => !IGNORED_FIELDS.contains(&name),
    };

    let mut out =
        format!("\t\t<hkobject name=\"{id}\" class=\"{class}\" signature=\"{signature}\">\n");
    for (name, kind) in fields.entries().filter(|(name, _)| keep(name)) {
        field_skeleton(&mut out, 3, name, kind, 0);
    }
    out.push_str("\t\t</hkobject>\n");
    Some(out)
}

/// Fields Havok XML does not write (`SERIALIZE_IGNORED`), used when there is no template object.
const IGNORED_FIELDS: [&str; 9] = [
    "memSizeAndFlags",
    "referenceCount",
    "cachedBindables",
    "areBindablesCached",
    "id",
    "cloneState",
    "padNode",
    "padModifier",
    "padStateInfo",
];

fn field_skeleton(out: &mut String, depth: usize, name: &str, kind: &str, nesting: usize) {
    use nemesis_xml::patch::class_table::find_class_info;
    let indent = "\t".repeat(depth);
    let value = match kind {
        "Pointer" => "null",
        "Bool" => "false",
        "F64" => "0.000000",
        "Object|Vector4" | "Object|Quaternion" => "(0.000000 0.000000 0.000000 0.000000)",
        kind if kind.starts_with('I') || kind.starts_with('U') => "0",
        kind if kind.starts_with("Array|") => {
            let _ = writeln!(out, "{indent}<hkparam name=\"{name}\" numelements=\"0\"></hkparam>");
            return;
        }
        kind => match kind.strip_prefix("Object|").and_then(find_class_info) {
            // A nested struct (e.g. `hkbEventProperty`): its fields, a few levels deep at most.
            Some(fields) if nesting < 4 => {
                let _ = writeln!(out, "{indent}<hkparam name=\"{name}\">");
                let _ = writeln!(out, "{indent}\t<hkobject>");
                for (field, kind) in fields.entries() {
                    field_skeleton(out, depth + 2, field, kind, nesting + 1);
                }
                let _ = writeln!(out, "{indent}\t</hkobject>");
                let _ = writeln!(out, "{indent}</hkparam>");
                return;
            }
            _ => "",
        },
    };
    let _ = writeln!(out, "{indent}<hkparam name=\"{name}\">{value}</hkparam>");
}

/// `info.ini` of a Nemesis mod.
pub fn info_ini(name: &str, author: &str, site: &str, auto: bool) -> String {
    let auto = if auto { "true" } else { "null" };
    format!("name={name}\nauthor={author}\nsite={site}\nauto={auto}\n")
}

/// Is `code` a valid Nemesis mod code? (4-6 ASCII letters/digits, not `vanilla`)
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
        let edited = TEMPLATE.replace(
            "<hkparam name=\"userData\">1</hkparam>",
            "<hkparam name=\"userData\">2</hkparam>",
        );
        let diff = diff_behavior("mymod", TEMPLATE, &edited);
        let patch = &diff.changed[0].patch;

        let applied = apply_patches(TEMPLATE, [("#0106", patch.as_str())]);
        assert_eq!(diff_behavior("mymod", TEMPLATE, &applied), diff);
    }

    #[test]
    fn revert_one_hunk() {
        let template = objects(TEMPLATE)[0].text;
        let edited = template
            .replace(
                "<hkparam name=\"userData\">1</hkparam>",
                "<hkparam name=\"userData\">2</hkparam>",
            )
            .replace(
                "<hkparam name=\"enable\">true</hkparam>",
                "<hkparam name=\"enable\">false</hkparam>",
            );
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
    fn class_skeleton() {
        let template = objects(TEMPLATE)[0].text;
        let new = skeleton("hkbModifierList", "#mymod$0", Some(template)).unwrap();
        assert!(new.starts_with(
            "\t\t<hkobject name=\"#mymod$0\" class=\"hkbModifierList\" signature=\"0xa4180ca1\">\n"
        ));
        assert!(new.contains("\t\t\t<hkparam name=\"modifiers\" numelements=\"0\"></hkparam>\n"));
        assert!(!new.contains("referenceCount"));
        assert!(check_fields(&new).is_empty(), "{new}");
        validate(&new).unwrap();
    }

    #[test]
    fn context_of_a_line() {
        let template = objects(TEMPLATE)[0].text;
        let (class, fields) = field_context(template, 2).unwrap();
        assert_eq!(class, "hkbModifierList");
        assert!(
            fields.iter().any(|(name, kind)| *name == "modifiers" && kind.starts_with("Array"))
        );
        assert_eq!(field_snippet("enable", "Bool", 3), "<hkparam name=\"enable\">false</hkparam>");
    }

    #[test]
    fn mod_codes() {
        assert!(is_valid_mod_code("colisc"));
        assert!(!is_valid_mod_code("abc"));
        assert!(!is_valid_mod_code("vanilla"));
        assert!(!is_valid_mod_code("ab-cd"));
    }
}
