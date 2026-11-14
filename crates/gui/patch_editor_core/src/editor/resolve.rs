//! Takes the value a user picked in a conflict into the user patch.
//!
//! The patch page says which mod's change to keep, by the place the merge knows it by
//! ([`ResolveRequest`]). The user patch loads last, so what it says wins. Only that change goes in: the
//! object's other fields stay for the other mods to change.
//!
//! | Target                       | The change comes from                                      |
//! | ---------------------------- | ---------------------------------------------------------- |
//! | a behavior                   | `<mod>/0_master/#0106.txt`: the block that changes the path |
//! | `animationdatasinglefile`    | `<mod>/animationdatasinglefile/<project>/<file>.txt`, whole |
//! | `animationsetdatasinglefile` | `<mod>/animationsetdatasinglefile/<project>/<file>.txt`, whole |
//!
//! Clip data has one file per clip, and an anim set's lines are positions, so these are copied as they are.

use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
};

#[cfg(test)]
use gui_shared::patch::ResolveSide;
use gui_shared::patch::{ResolveRequest, USER_PATCH_CODE};
use nemesis_diff::{
    View,
    conflict::{apply_block_over, block_of_change, blocks_of_change, replacing_blocks},
    resolve,
};

use crate::workspace::{ADSF, ASDSF, Workspace, nemesis_folder, read_text};

/// Where the change ended up: to show it.
#[derive(Debug)]
pub struct Resolved {
    /// In `Workspace::behaviors`.
    pub behavior: usize,
    /// e.g. `#0106`
    pub id: String,
    /// The 0-based line in the object, if it is known.
    pub line: Option<usize>,
}

/// The name of the animation data behavior of this target, if it is one. e.g. [`ADSF`]
fn anim_stem(target: &str) -> Option<&'static str> {
    let file = target.rsplit('/').next().unwrap_or_default();
    let stem = file.strip_suffix(".bin").unwrap_or(file);
    [ADSF, ASDSF].into_iter().find(|known| stem.eq_ignore_ascii_case(known))
}

/// The object the request is about, opened if its behavior is not yet: where the user can do by hand
/// what [`apply`] could not. `None`: it is not in the template.
pub fn object_of(workspace: &mut Workspace, template_root: &Path, request: &ResolveRequest) -> Option<Resolved> {
    let (behavior, id) = if let Some(stem) = anim_stem(&request.target) {
        let id = anim_file_of(&request.path)?;
        workspace.anim_behavior(stem, template_root).ok()?;
        (workspace.behaviors.iter().position(|behavior| behavior.name == stem)?, id)
    } else {
        let at = workspace.open_behavior(&template_root.join(&request.target), &request.target).ok()?;
        (at, request.path.split('/').next()?.to_owned())
    };
    workspace.behaviors[behavior].text(&id)?;
    Some(Resolved { behavior, id, line: None })
}

/// See the module docs.
///
/// # Errors
/// The mod's file cannot be read, has no such change, or does not fit the object. The user can still
/// edit the object by hand: the message says why it was not done.
pub fn apply(workspace: &mut Workspace, template_root: &Path, request: &ResolveRequest) -> Result<Resolved, String> {
    match anim_stem(&request.target) {
        Some(stem) => apply_anim_data(workspace, template_root, request, stem),
        None if request.keep_all_events => apply_event_arrays(workspace, template_root, request),
        None => apply_behavior(workspace, template_root, request),
    }
}

/// The class `enterNotifyEvents` / `exitNotifyEvents` point to. See [`apply_event_arrays`].
const EVENT_ARRAY_CLASS: &str = "hkbStateMachineEventPropertyArray";

/// Keeps the events of every mod of `request.sides` at an `enterNotifyEvents` / `exitNotifyEvents`
/// conflict ([`gui_shared::patch::is_event_array_conflict`]). Each mod points the field to an array of
/// its own; the user patch gets one array with all their events, and points the field to it. The mods'
/// objects are not changed.
///
/// ```text
/// #1821 enterNotifyEvents: #modA$1 (events: 10)   |   #modB$1 (events: 20)
/// -> #upatch$0 (events: 10, 20), #1821 enterNotifyEvents: #upatch$0
/// ```
/// Doing it again rewrites the user patch's array, instead of adding another.
fn apply_event_arrays(
    workspace: &mut Workspace,
    template_root: &Path,
    request: &ResolveRequest,
) -> Result<Resolved, String> {
    let mut parts = request.path.split('/');
    let (id, field) = (parts.next().unwrap_or_default(), parts.next_back().unwrap_or_default());
    // Each mod's array, by what its patch points the field to.
    let mut arrays = Vec::new();
    for side in &request.sides {
        let Ok(patch) = read_text(&patch_file(&side.mod_dir, &request.target, id)) else { continue };
        let object = resolve(&patch, View::Mod);
        let Some(pointer) = field_value(&object, field).filter(|value| *value != "null") else { continue };
        let array = read_text(&patch_file(&side.mod_dir, &request.target, pointer))?;
        arrays.push(resolve(&array, View::Mod).into_owned());
    }
    if arrays.len() < 2 {
        return Err(format!("{}: fewer than two mods have an array of events here", request.path));
    }

    let at = workspace.open_behavior(&template_root.join(&request.target), &request.target)?;
    let behavior = &mut workspace.behaviors[at];
    let current = behavior.text(id).ok_or_else(|| format!("{id}: not in the template"))?.to_owned();
    // The array made the last time, if the field still points to it.
    let made_before = field_value(&current, field)
        .filter(|value| value.starts_with(&format!("#{USER_PATCH_CODE}$")) && behavior.added.contains_key(*value))
        .map(str::to_owned);
    let new_id = made_before.unwrap_or_else(|| behavior.add_object(USER_PATCH_CODE, None));
    let texts: Vec<&str> = arrays.iter().map(String::as_str).collect();
    let merged = merged_event_array(&new_id, &texts)?;
    behavior.added.insert(new_id.clone(), merged);
    let edited = with_field_value(&current, field, &new_id).ok_or_else(|| format!("{id}: no field `{field}`"))?;
    behavior.set_text(id, &edited);
    Ok(Resolved { behavior: at, id: id.to_owned(), line: None })
}

/// The value of the one-line field `field` of an object. e.g. `<hkparam name="a">#m$1</hkparam>` -> `#m$1`
fn field_value<'a>(object: &'a str, field: &str) -> Option<&'a str> {
    let open = format!("<hkparam name=\"{field}\">");
    object.lines().find_map(|line| line.trim().strip_prefix(open.as_str())?.strip_suffix("</hkparam>"))
}

/// `object` with the value of its one-line field `field` set to `value`. `None`: it has no such line.
fn with_field_value(object: &str, field: &str, value: &str) -> Option<String> {
    let open = format!("<hkparam name=\"{field}\">");
    let mut found = false;
    let mut out = String::with_capacity(object.len() + value.len());
    for line in object.split_inclusive('\n') {
        match line.find(open.as_str()) {
            Some(at) if !found && line.trim_end().ends_with("</hkparam>") => {
                found = true;
                let ending = &line[line.trim_end().len()..];
                out.push_str(&line[..at + open.len()]);
                out.push_str(value);
                out.push_str("</hkparam>");
                out.push_str(ending);
            }
            _ => out.push_str(line),
        }
    }
    found.then_some(out)
}

/// The elements (`<hkobject>` blocks, with their lines) of the array field `field` of `object`.
fn array_elements<'a>(object: &'a str, field: &str) -> Vec<&'a str> {
    let open = format!("<hkparam name=\"{field}\"");
    let mut lines = object.split_inclusive('\n').scan(0, |offset, line| {
        let at = *offset;
        *offset += line.len();
        Some((at, line))
    });
    if !lines.by_ref().any(|(_, line)| line.trim_start().starts_with(open.as_str())) {
        return Vec::new();
    }
    let (mut elements, mut depth, mut start) = (Vec::new(), 0_usize, 0);
    for (at, line) in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("</hkobject>") {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                elements.push(&object[start..at + line.len()]);
            }
        } else if trimmed.starts_with("<hkobject") && !trimmed.ends_with("/>") {
            if depth == 0 {
                start = at;
            }
            depth += 1;
        } else if depth == 0 && trimmed.starts_with("</hkparam>") {
            break;
        }
    }
    elements
}

/// One `hkbStateMachineEventPropertyArray` `new_id` with the events of all `arrays` (in their order,
/// each the same event once), from the first one's head line.
///
/// # Errors
/// One of them is not of that class.
fn merged_event_array(new_id: &str, arrays: &[&str]) -> Result<String, String> {
    let mut events: Vec<&str> = Vec::new();
    for array in arrays {
        let head = array.lines().next().unwrap_or_default();
        if !head.contains(&format!("class=\"{EVENT_ARRAY_CLASS}\"")) {
            return Err(format!("not an array of events: {}", head.trim()));
        }
        for event in array_elements(array, "events") {
            if !events.iter().any(|known| known.trim() == event.trim()) {
                events.push(event);
            }
        }
    }
    let head = arrays.first().and_then(|array| array.lines().next()).unwrap_or_default();
    let indent = &head[..head.len() - head.trim_start().len()];
    let name_start = head.find("name=\"").map_or(0, |at| at + "name=\"".len());
    let name_end = head[name_start..].find('"').map_or(name_start, |len| name_start + len);
    let mut out = format!("{}{new_id}{}\n", &head[..name_start], &head[name_end..]);
    let _ = writeln!(out, "{indent}\t<hkparam name=\"events\" numelements=\"{}\">", events.len());
    for event in &events {
        out.push_str(event);
        if !event.ends_with('\n') {
            out.push('\n');
        }
    }
    let _ = writeln!(out, "{indent}\t</hkparam>\n{indent}</hkobject>");
    Ok(out)
}

/// `DefaultMale~1/Idle~348/play_back_speed` -> `DefaultMale~1/Idle~348.txt`
/// `DefaultMaleData~DefaultMale/_MTSolo.txt/anim_infos` -> `DefaultMaleData~DefaultMale/_MTSolo.txt`
fn anim_file_of(path: &str) -> Option<String> {
    let mut parts = path.split('/');
    let (project, name) = (parts.next()?, parts.next()?);
    let has_extension = Path::new(name).extension().is_some_and(|extension| extension.eq_ignore_ascii_case("txt"));
    Some(if has_extension { format!("{project}/{name}") } else { format!("{project}/{name}.txt") })
}

fn apply_anim_data(
    workspace: &mut Workspace,
    template_root: &Path,
    request: &ResolveRequest,
    stem: &str,
) -> Result<Resolved, String> {
    let id = anim_file_of(&request.path).ok_or_else(|| format!("{}: no file in this path", request.path))?;
    let text = read_text(&Path::new(&request.mod_dir).join(stem).join(&id))?;
    let behavior = workspace.anim_behavior(stem, template_root)?;
    if behavior.template_object(&id).is_some() {
        behavior.set_text(&id, &resolve(&text, View::Mod));
    } else {
        behavior.added.insert(id.clone(), text);
    }
    let behavior = workspace.behaviors.iter().position(|behavior| behavior.name == stem).unwrap_or(0);
    Ok(Resolved { behavior, id, line: None })
}

/// The patch file of object `id` of a behavior in the mod at `mod_dir`.
///
/// e.g. `D:/mods/a/Nemesis_Engine/mod/a/0_master/#2489.txt`
fn patch_file(mod_dir: &str, target: &str, id: &str) -> PathBuf {
    let folder = match nemesis_folder(target) {
        Some((name, true)) => Path::new("_1stperson").join(name),
        Some((name, false)) => Path::new(&name).to_path_buf(),
        None => Path::new(target).to_path_buf(),
    };
    Path::new(mod_dir).join(folder).join(format!("{id}.txt"))
}

/// What a mod has at a conflict. See [`MergeValues`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeSide {
    /// e.g. `Paraglider`
    pub name: String,
    /// Its lines there. e.g. `\t\t\t\t#para$3`. Empty: it removes them.
    pub lines: Vec<String>,
}

/// What the mods have at a conflict, to make the result from in the editor: with an array, the elements
/// of several mods are often wanted, which no one of them has.
///
/// ```text
/// #2489/hkbManualSelectorGenerator/generators/[0:1]
/// Original   #0338
/// Flinching  #flinch$2     [Insert]
/// Impact     #neymar$1019  [Insert]
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeValues {
    /// The object: it is shown with this one.
    pub id: String,
    /// e.g. `#2489/hkbManualSelectorGenerator/generators/[0:1]`
    pub path: String,
    /// What the template has there, as the mods say.
    pub original: Vec<String>,
    pub sides: Vec<MergeSide>,
}

/// What the mods of `request` have at its conflict. `None`: there is nothing to choose from, as with
/// less than two mods, or animation data, whose files are whole.
pub fn merge_values(request: &ResolveRequest, id: &str) -> Option<MergeValues> {
    if anim_stem(&request.target).is_some() || request.sides.len() < 2 {
        return None;
    }
    let mut original = Vec::new();
    let lines_of = |text: &str| text.lines().map(str::to_owned).collect::<Vec<String>>();
    let sides: Vec<MergeSide> = request
        .sides
        .iter()
        .filter_map(|side| {
            // A mod whose file cannot be read has nothing to offer: the others still do.
            let patch = read_text(&patch_file(&side.mod_dir, &request.target, id)).ok()?;
            let blocks = blocks_of_change(&patch, &request.path);
            if blocks.is_empty() {
                return None;
            }
            if original.is_empty() {
                original = blocks.iter().flat_map(|block| lines_of(&block.original)).collect();
            }
            let lines = blocks.iter().flat_map(|block| lines_of(&block.text)).collect();
            Some(MergeSide { name: side.name.clone(), lines })
        })
        .collect();
    (!sides.is_empty()).then(|| MergeValues { id: id.to_owned(), path: request.path.clone(), original, sides })
}

/// `text` with `lines` after line `after`. Returns it, and the last line put in.
///
/// e.g. `a\nb\n`, after line 0, `x`: `a\nx\nb\n`, 1
pub fn insert_lines(text: &str, after: usize, lines: &[String]) -> (String, usize) {
    let mut all: Vec<&str> = text.lines().collect();
    let at = (after + 1).min(all.len());
    all.splice(at..at, lines.iter().map(String::as_str));
    let mut out = all.join("\n");
    out.push('\n');
    (out, at + lines.len().saturating_sub(1))
}

fn apply_behavior(
    workspace: &mut Workspace,
    template_root: &Path,
    request: &ResolveRequest,
) -> Result<Resolved, String> {
    let at = workspace.open_behavior(&template_root.join(&request.target), &request.target)?;
    let behavior = &mut workspace.behaviors[at];
    let id = request.path.split('/').next().unwrap_or_default();

    let patch = read_text(&patch_file(&request.mod_dir, &request.target, id))?;

    if replacing_blocks(&patch).is_none() {
        // A new object: the file is the object.
        behavior.added.insert(id.to_owned(), patch);
        return Ok(Resolved { behavior: at, id: id.to_owned(), line: None });
    }
    let block = block_of_change(&patch, &request.path)
        .ok_or_else(|| format!("{}: this mod's file has no such change", request.path))?;
    let current = behavior.text(id).ok_or_else(|| format!("{id}: not in the template"))?;
    let template = behavior.template_object(id).ok_or_else(|| format!("{id}: not in the template"))?;
    let (edited, line) = apply_block_over(&template.text, current, &block)
        .ok_or_else(|| format!("{}: the lines this change replaces are not in the object", request.path))?;
    behavior.set_text(id, &edited);
    Ok(Resolved { behavior: at, id: id.to_owned(), line: Some(line) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_go_below_the_cursor() {
        let lines = ["x".to_owned(), "y".to_owned()];
        assert_eq!(insert_lines("a\nb\n", 0, &lines), ("a\nx\ny\nb\n".to_owned(), 2));
        assert_eq!(insert_lines("a\nb\n", 1, &lines), ("a\nb\nx\ny\n".to_owned(), 3));
        // Beyond the end, and a last line without its end.
        assert_eq!(insert_lines("a\nb", 9, &lines), ("a\nb\nx\ny\n".to_owned(), 3));
        assert_eq!(insert_lines("a\nb", 1, &lines[..1]), ("a\nb\nx\n".to_owned(), 2));
    }

    /// As in `0_master`'s `#2489`, which three mods change.
    #[test]
    fn each_mod_has_its_lines_at_the_conflict() {
        const PATH: &str = "#2489/hkbManualSelectorGenerator/generators/[0:1]";
        let dir = std::env::temp_dir().join(format!("d_merge_merge_values_{}", std::process::id()));
        let patch = |code: &str, value: &str| {
            format!(
                "\t\t<hkobject name=\"#2489\" class=\"hkbManualSelectorGenerator\" signature=\"0xd932fab8\">\n\t\t\t<hkparam name=\"generators\" numelements=\"2\">\n<!-- MOD_CODE ~{code}~ OPEN -->\n\t\t\t\t{value}\n<!-- ORIGINAL -->\n\t\t\t\t#0338\n<!-- CLOSE -->\n\t\t\t\t#0340\n\t\t\t</hkparam>\n\t\t</hkobject>\n"
            )
        };
        let side = |code: &str, value: &str| {
            let mod_dir = dir.join(code);
            let file = mod_dir.join("0_master").join("#2489.txt");
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, patch(code, value)).unwrap();
            ResolveSide { name: code.to_owned(), mod_dir: mod_dir.display().to_string() }
        };
        let flinch = side("flinch", "#flinch$2");
        let impact = side("neymar", "#neymar$1019");
        let gone = ResolveSide { name: "gone".to_owned(), mod_dir: dir.join("gone").display().to_string() };
        let request = |sides| ResolveRequest {
            target: "meshes/actors/character/behaviors/0_master.bin".to_owned(),
            path: PATH.to_owned(),
            mod_dir: String::new(),
            edit: true,
            sides,
            keep_all_events: false,
        };

        let values = merge_values(&request(vec![flinch.clone(), gone, impact]), "#2489").unwrap();
        let alone = merge_values(&request(vec![flinch]), "#2489");
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(values.original, ["\t\t\t\t#0338"]);
        assert_eq!(values.sides.len(), 2, "the one that cannot be read is left out");
        assert_eq!(values.sides[0].name, "flinch");
        assert_eq!(values.sides[0].lines, ["\t\t\t\t#flinch$2"]);
        assert_eq!(values.sides[1].lines, ["\t\t\t\t#neymar$1019"]);
        assert_eq!(alone, None, "one mod: nothing to choose from");
    }

    /// The example of the request: two mods each with an array of one event.
    fn event_array(id: &str, event: u32) -> String {
        format!(
            "\t\t<hkobject name=\"{id}\" class=\"hkbStateMachineEventPropertyArray\" signature=\"0xb07b4388\">\n\t\t\t<hkparam name=\"events\" numelements=\"1\">\n\t\t\t\t<hkobject>\n\t\t\t\t\t<hkparam name=\"id\">{event}</hkparam>\n\t\t\t\t\t<hkparam name=\"payload\">null</hkparam>\n\t\t\t\t</hkobject>\n\t\t\t</hkparam>\n\t\t</hkobject>\n"
        )
    }

    #[test]
    fn the_events_of_every_mod_are_kept() {
        let (a, b) = (event_array("#modA$1", 10), event_array("#modB$1", 20));
        let merged = merged_event_array("#upatch$0", &[&a, &b, &a]).unwrap();
        assert!(merged.starts_with("\t\t<hkobject name=\"#upatch$0\" class=\"hkbStateMachineEventPropertyArray\""));
        assert!(merged.contains("numelements=\"2\""), "the same event once: {merged}");
        let ten = merged.find(">10<").unwrap();
        assert!(ten < merged.find(">20<").unwrap(), "in their order");
        assert!(nemesis_diff::validate(&merged).is_ok(), "{merged}");
        assert!(merged_event_array("#u$0", &["\t\t<hkobject name=\"#x\" class=\"hkbClipGenerator\">\n"]).is_err());
    }

    #[test]
    fn the_field_points_to_the_merged_array() {
        let object = "\t\t<hkobject name=\"#1821\" class=\"hkbStateMachineStateInfo\">\n\t\t\t<hkparam name=\"enterNotifyEvents\">null</hkparam>\n\t\t</hkobject>\n";
        assert_eq!(field_value(object, "enterNotifyEvents"), Some("null"));
        let edited = with_field_value(object, "enterNotifyEvents", "#upatch$0").unwrap();
        assert_eq!(edited, object.replace(">null<", ">#upatch$0<"));
        assert_eq!(with_field_value(object, "exitNotifyEvents", "#x"), None);
    }

    #[test]
    fn finds_the_file_of_an_anim_data_change() {
        assert_eq!(anim_file_of("D~1/Idle~348/play_back_speed").as_deref(), Some("D~1/Idle~348.txt"));
        assert_eq!(anim_file_of("D~1/50/duration").as_deref(), Some("D~1/50.txt"));
        assert_eq!(anim_file_of("D~D/_MTSolo.txt/x").as_deref(), Some("D~D/_MTSolo.txt"));
        assert_eq!(anim_file_of("nothing"), None);
    }
}
