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

use std::path::Path;

use gui_shared::patch::ResolveRequest;
use nemesis_diff::{
    View,
    conflict::{apply_block, block_of_change, replacing_blocks},
    resolve,
};

use super::workspace::{ADSF, ASDSF, Workspace, nemesis_folder, read_text};

/// Where the change ended up: to show it. `(behavior, object)`
pub(super) type Resolved = (usize, String);

/// See the module docs.
///
/// # Errors
/// The mod's file cannot be read, has no such change, or does not fit the object. The user can still
/// edit the object by hand: the message says why it was not done.
pub(super) fn apply(
    workspace: &mut Workspace,
    template_root: &Path,
    request: &ResolveRequest,
) -> Result<Resolved, String> {
    let file = request.target.rsplit('/').next().unwrap_or_default();
    let stem = file.strip_suffix(".bin").unwrap_or(file);
    if stem.eq_ignore_ascii_case(ADSF) || stem.eq_ignore_ascii_case(ASDSF) {
        return apply_anim_data(workspace, template_root, request, stem);
    }
    apply_behavior(workspace, template_root, request)
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
    let name = if stem.eq_ignore_ascii_case(ADSF) { ADSF } else { ASDSF };
    let behavior = workspace.anim_behavior(name, template_root)?;
    if behavior.template_object(&id).is_some() {
        behavior.set_text(&id, resolve(&text, View::Mod));
    } else {
        behavior.added.insert(id.clone(), text);
    }
    let at = workspace.behaviors.iter().position(|behavior| behavior.name == name).unwrap_or(0);
    Ok((at, id))
}

fn apply_behavior(
    workspace: &mut Workspace,
    template_root: &Path,
    request: &ResolveRequest,
) -> Result<Resolved, String> {
    let at = workspace.open_behavior(&template_root.join(&request.target), &request.target)?;
    let behavior = &mut workspace.behaviors[at];
    let id = request.path.split('/').next().unwrap_or_default();

    let file = format!("{id}.txt");
    let folder = match nemesis_folder(&request.target) {
        Some((name, true)) => Path::new("_1stperson").join(name),
        Some((name, false)) => Path::new(&name).to_path_buf(),
        None => Path::new(&request.target).to_path_buf(),
    };
    let patch = read_text(&Path::new(&request.mod_dir).join(folder).join(file))?;

    if replacing_blocks(&patch).is_none() {
        // A new object: the file is the object.
        behavior.added.insert(id.to_owned(), patch);
        return Ok((at, id.to_owned()));
    }
    let block = block_of_change(&patch, &request.path)
        .ok_or_else(|| format!("{}: this mod's file has no such change", request.path))?;
    let current = behavior.text(id).ok_or_else(|| format!("{id}: not in the template"))?;
    let edited = apply_block(current, &block)
        .ok_or_else(|| format!("{}: the lines this change replaces are not in the object", request.path))?;
    behavior.set_text(id, edited);
    Ok((at, id.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_file_of_an_anim_data_change() {
        assert_eq!(anim_file_of("D~1/Idle~348/play_back_speed").as_deref(), Some("D~1/Idle~348.txt"));
        assert_eq!(anim_file_of("D~1/50/duration").as_deref(), Some("D~1/50.txt"));
        assert_eq!(anim_file_of("D~D/_MTSolo.txt/x").as_deref(), Some("D~D/_MTSolo.txt"));
        assert_eq!(anim_file_of("nothing"), None);
    }
}
