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

use std::path::{Path, PathBuf};

use gui_shared::patch::ResolveRequest;
#[cfg(test)]
use gui_shared::patch::ResolveSide;
use nemesis_diff::{
    View,
    conflict::{apply_block_over, block_of_change, blocks_of_change, replacing_blocks},
    resolve,
};

use crate::patch::workspace::{ADSF, ASDSF, Workspace, nemesis_folder, read_text};

/// Where the change ended up: to show it.
#[derive(Debug)]
pub(in crate::patch) struct Resolved {
    /// In `Workspace::behaviors`.
    pub(in crate::patch) behavior: usize,
    /// e.g. `#0106`
    pub(in crate::patch) id: String,
    /// The 0-based line in the object, if it is known.
    pub(in crate::patch) line: Option<usize>,
}

/// The name of the animation data behavior of this target, if it is one. e.g. [`ADSF`]
fn anim_stem(target: &str) -> Option<&'static str> {
    let file = target.rsplit('/').next().unwrap_or_default();
    let stem = file.strip_suffix(".bin").unwrap_or(file);
    [ADSF, ASDSF].into_iter().find(|known| stem.eq_ignore_ascii_case(known))
}

/// The object the request is about, opened if its behavior is not yet: where the user can do by hand
/// what [`apply`] could not. `None`: it is not in the template.
pub(in crate::patch) fn object_of(
    workspace: &mut Workspace,
    template_root: &Path,
    request: &ResolveRequest,
) -> Option<Resolved> {
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
pub(in crate::patch) fn apply(
    workspace: &mut Workspace,
    template_root: &Path,
    request: &ResolveRequest,
) -> Result<Resolved, String> {
    match anim_stem(&request.target) {
        Some(stem) => apply_anim_data(workspace, template_root, request, stem),
        None => apply_behavior(workspace, template_root, request),
    }
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
pub(in crate::patch) struct MergeSide {
    /// e.g. `Paraglider`
    pub(in crate::patch) name: String,
    /// Its lines there. e.g. `\t\t\t\t#para$3`. Empty: it removes them.
    pub(in crate::patch) lines: Vec<String>,
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
pub(in crate::patch) struct MergeValues {
    /// The object: it is shown with this one.
    pub(in crate::patch) id: String,
    /// e.g. `#2489/hkbManualSelectorGenerator/generators/[0:1]`
    pub(in crate::patch) path: String,
    /// What the template has there, as the mods say.
    pub(in crate::patch) original: Vec<String>,
    pub(in crate::patch) sides: Vec<MergeSide>,
}

/// What the mods of `request` have at its conflict. `None`: there is nothing to choose from, as with
/// less than two mods, or animation data, whose files are whole.
pub(in crate::patch) fn merge_values(request: &ResolveRequest, id: &str) -> Option<MergeValues> {
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
pub(in crate::patch) fn insert_lines(text: &str, after: usize, lines: &[String]) -> (String, usize) {
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

    #[test]
    fn finds_the_file_of_an_anim_data_change() {
        assert_eq!(anim_file_of("D~1/Idle~348/play_back_speed").as_deref(), Some("D~1/Idle~348.txt"));
        assert_eq!(anim_file_of("D~1/50/duration").as_deref(), Some("D~1/50.txt"));
        assert_eq!(anim_file_of("D~D/_MTSolo.txt/x").as_deref(), Some("D~D/_MTSolo.txt"));
        assert_eq!(anim_file_of("nothing"), None);
    }
}
