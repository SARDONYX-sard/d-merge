//! Generates the animation data a new animation needs, from the mod's `hkbClipGenerator`s and hkx.
//!
//! A behavior only plays a clip if the game also has its data, in two more template files:
//!
//! | File                         | Patch                                                    | From             |
//! | ---------------------------- | -------------------------------------------------------- | ---------------- |
//! | `animationdatasinglefile`    | `DefaultMale~1/<clip name>~mymod$0.txt`: speed, triggers | hkx annotations  |
//! |                              | `DefaultMale~1/mymod$0.txt`: length, movement            | hkx duration     |
//! | `animationsetdatasinglefile` | `DefaultMaleData~DefaultMale/<set>.txt`: which hkx load  | hkx path (CRC32) |
//!
//! ```text
//! <hkparam name="name">Paraglide_Equip</hkparam>                         the clip's name
//! <hkparam name="animationName">Animations\NickNak\ParaGlideEquip.hkx</hkparam>
//!   -> <mod root>/meshes/actors/character/Animations/NickNak/ParaGlideEquip.hkx
//! ```
//! `mymod$0` numbers the clips. Generating again keeps the number a clip already has.
//!
//! Triggers are free text, so nothing is inferred about them: all the annotations of the hkx are
//! offered, and the user chooses. Editing them, [`trigger_names`] gives the known ones for completion.
//!
//! # Anim sets
//! An anim set file (`1HMSolo.txt`, a key of `txt_projects.<project>` in the template's json) lists
//! the hkx the game caches while that set is active. Most patches of it only push hkx paths, which
//! needs nothing but the paths. Which hkx go into which txt cannot be derived from the mod, so the
//! user picks both ([`AnimSetPush`]); this is independent of the clips above.

use std::{collections::BTreeSet, path::Path};

use skyrim_anim_data::adsf::alt::AltAdsf;

use super::{
    anim_patch::AdsfFile,
    workspace::{ADSF, ASDSF, Behavior, Workspace},
};

/// The `~1` of `DefaultMale~1`: which of the projects with that name. Only projects with one of
/// each are generated.
const PROJECT_INDEX: u32 = 1;

/// The CRC32 of `hkx` (without the dot), the last line of every anim info.
const HKX_EXTENSION_CRC: &str = "7891816";

/// A clip of the mod, from a `hkbClipGenerator`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ClipSource {
    /// e.g. `Paraglide_Equip`
    pub(super) name: String,
    /// As written in the behavior, e.g. `Animations\NickNak\ParaGlideEquip.hkx`
    pub(super) animation: String,
    /// What it is relative to, e.g. `meshes/actors/character`
    pub(super) base: String,
    /// e.g. `DefaultMale`, `DefaultFemale`
    pub(super) projects: Vec<String>,
}

/// What the data needs from a hkx.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct HkxInfo {
    /// Seconds.
    pub(super) duration: f32,
    /// Annotations as `(time, text)`, e.g. `(0.03, "AnimObjDraw")`.
    pub(super) triggers: Vec<(f32, String)>,
    /// The root motion, see [`serde_hkx_for_gui::hkanno::parse_root_motion`].
    pub(super) motion: Vec<[f32; 4]>,
}

/// The value of a field on one line. e.g., `name` in `<hkparam name="name">x</hkparam>` -> `x`
fn field<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<hkparam name=\"{name}\">");
    let (_, rest) = text.split_once(&open)?;
    rest.split_once("</hkparam>").map(|(value, _)| value)
}

/// The projects (characters) a behavior belongs to, by its template path.
///
/// e.g., `meshes/actors/character/behaviors/0_master.bin` -> `DefaultMale`, `DefaultFemale`;
/// `meshes/actors/draugr/behaviors/x.bin` -> `DraugrProject` (the usual name; a few differ).
fn projects_of(meshes_path: &str) -> Vec<String> {
    let lower = meshes_path.to_ascii_lowercase();
    if lower.contains("/_1stperson/") {
        return vec!["FirstPerson".to_owned()];
    }
    if lower.contains("/actors/character/") {
        return vec!["DefaultMale".to_owned(), "DefaultFemale".to_owned()];
    }
    let actor = lower.split("/actors/").nth(1).and_then(|rest| rest.split('/').next());
    actor.map_or_else(Vec::new, |actor| {
        let mut chars = actor.chars();
        let first = chars.next().map(|c| c.to_ascii_uppercase()).into_iter();
        vec![first.chain(chars).chain("Project".chars()).collect()]
    })
}

/// `meshes/actors/character/behaviors/0_master.bin` -> `meshes/actors/character`
fn base_of(meshes_path: &str) -> &str {
    meshes_path.rsplit_once("/behaviors/").map_or(meshes_path, |(base, _)| base)
}

/// The clips the mod adds or changes, one per clip name.
pub(super) fn clip_sources(behaviors: &[Behavior]) -> Vec<ClipSource> {
    let mut clips: Vec<ClipSource> = Vec::new();
    for behavior in behaviors.iter().filter(|behavior| !behavior.is_anim_data()) {
        let generators = behavior.texts().filter(|o| (o.is_new || o.is_edited) && o.class == "hkbClipGenerator");
        for object in generators {
            let (Some(name), Some(animation)) = (field(object.text, "name"), field(object.text, "animationName"))
            else {
                continue;
            };
            if !clips.iter().any(|clip| clip.name == name) {
                clips.push(ClipSource {
                    name: name.to_owned(),
                    animation: animation.to_owned(),
                    base: base_of(&behavior.meshes_path).to_owned(),
                    projects: projects_of(&behavior.meshes_path),
                });
            }
        }
    }
    clips
}

/// The folder holding a mod's meshes, and the one animations are in. Windows paths: any case.
const MESHES_DIR: &str = "meshes";
const ANIMATIONS_DIR: &str = "animations";

/// `relative` (to the mod's root) in the form anim sets hash, if it is an animation of the mod: a
/// `.hkx` in an `animations` folder under `meshes`.
///
/// e.g., `meshes/actors/character/animations/A/B.hkx` -> `meshes\actors\character\animations\A\B.hkx`.
/// `meshes/actors/character/behaviors/0_master.hkx` is not an animation.
fn anim_hkx_path(relative: &Path) -> Option<String> {
    let is_hkx = relative.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("hkx"));
    let parts: Vec<&str> = relative.iter().map(std::ffi::OsStr::to_str).collect::<Option<_>>()?;
    let in_animations = parts.iter().any(|part| part.eq_ignore_ascii_case(ANIMATIONS_DIR));
    let in_meshes = parts.first().is_some_and(|part| part.eq_ignore_ascii_case(MESHES_DIR));
    (is_hkx && in_meshes && in_animations).then(|| parts.join("\\"))
}

/// The animations of the mod at `mod_root`, sorted, as [`anim_hkx_path`] gives them. Folders that
/// cannot be read are left out: the user sees what is listed, and picks from that.
pub(super) fn anim_hkx_files(mod_root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut folders = vec![mod_root.join(MESHES_DIR)];
    while let Some(folder) = folders.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else if let Some(hkx) = path.strip_prefix(mod_root).ok().and_then(anim_hkx_path) {
                found.push(hkx);
            }
        }
    }
    found.sort_unstable();
    found
}

/// The project to show first: the one `hkx` (a [`anim_hkx_files`] path) belongs to, else
/// `DefaultMale`, the most common. An index into `folders` (`DefaultMaleData~DefaultMale`, ...).
pub(super) fn default_project(folders: &[&str], hkx: Option<&str>) -> usize {
    let projects = hkx.map_or_else(Vec::new, |hkx| projects_of(&hkx.replace('\\', "/")));
    let folder_of = |project: &str| format!("{project}Data~{project}");
    let wanted = projects.iter().map(String::as_str).chain(["DefaultMale"]).map(folder_of);
    wanted.into_iter().find_map(|folder| folders.iter().position(|f| *f == folder)).unwrap_or(0)
}

/// Reads what the data needs from the clip's hkx under the mod's root.
pub(super) fn read_hkx(mod_root: &Path, clip: &ClipSource) -> Result<HkxInfo, String> {
    let path = mod_root.join(&clip.base).join(clip.animation.replace('\\', "/"));
    let error = |e: &dyn std::fmt::Display| format!("{}: {e}", path.display());
    let bytes = std::fs::read(&path).map_err(|e| error(&e))?;
    let hkanno = serde_hkx_for_gui::hkanno::parse_as_hkanno(&bytes, &path).map_err(|e| error(&e))?;
    let triggers = hkanno
        .annotation_tracks
        .iter()
        .flat_map(|track| &track.annotations)
        .filter_map(|annotation| Some((annotation.time, annotation.text.as_ref()?.to_string())))
        .collect();
    let motion = serde_hkx_for_gui::hkanno::parse_root_motion(&bytes, &path).map_err(|e| error(&e))?;
    Ok(HkxInfo { duration: hkanno.duration, triggers, motion })
}

/// The clip's block: name, id, speed, crop start, crop end, then the triggers.
///
/// e.g. `Paraglide_Equip`, `para$2`, `1`, `0`, `0`, `1`, `AnimObjDraw:0.03`
fn clip_anim_text(name: &str, clip_id: &str, info: &HkxInfo) -> String {
    use std::fmt::Write as _;
    let mut text = format!("{name}\n{clip_id}\n1\n0\n0\n{}\n", info.triggers.len());
    for (time, trigger) in &info.triggers {
        let _ = writeln!(text, "{trigger}:{time}");
    }
    text
}

/// The clip's movement: id, length, translations (time x y z), rotations (time x y z w).
///
/// e.g. `para$0`, `3.3`, `1`, `3.3 0 0 0`, `1`, `3.3 0 0 0 1`: no movement.
fn clip_motion_text(clip_id: &str, duration: f32, motion: &[[f32; 4]]) -> String {
    use std::fmt::Write as _;
    let (translations, rotations) = motion_keys(duration, motion);
    let mut text = format!("{clip_id}\n{duration}\n{}\n", translations.len());
    for [time, x, y, z] in translations {
        let _ = writeln!(text, "{time} {x} {y} {z}");
    }
    let _ = writeln!(text, "{}", rotations.len());
    for [time, x, y, z, w] in rotations {
        let _ = writeln!(text, "{time} {x} {y} {z} {w}");
    }
    text
}

/// The root motion as the keys the data has: `[time, x, y, z]` and `[time, x, y, z, w]`.
///
/// The samples are evenly spaced. Only the changes are kept, and the end is always there, so an
/// animation that does not move has one key at its end: `3.3 0 0 0`, `3.3 0 0 0 1`.
/// `w` of a sample is the turn around the up axis (z) in radians; a rotation is a quaternion.
fn motion_keys(duration: f32, motion: &[[f32; 4]]) -> (Vec<[f32; 4]>, Vec<[f32; 5]>) {
    #[expect(clippy::cast_precision_loss, reason = "a few thousand samples")]
    let step = duration / (motion.len().max(2) - 1) as f32;
    #[expect(clippy::cast_precision_loss, reason = "a few thousand samples")]
    let time = |i: usize| i as f32 * step;

    let mut translations = Vec::new();
    let mut rotations = Vec::new();
    let (mut last_position, mut last_turn) = ([0.0_f32; 3], 0.0_f32);
    for (i, [x, y, z, turn]) in motion.iter().copied().enumerate() {
        if [x, y, z].iter().zip(&last_position).any(|(a, b)| (a - b).abs() > f32::EPSILON) {
            translations.push([time(i), x, y, z]);
            last_position = [x, y, z];
        }
        if (turn - last_turn).abs() > f32::EPSILON {
            let (sin, cos) = (turn / 2.0).sin_cos();
            rotations.push([time(i), 0.0, 0.0, sin, cos]);
            last_turn = turn;
        }
    }
    if translations.last().is_none_or(|key| key[0] < duration) {
        let [x, y, z] = last_position;
        translations.push([duration, x, y, z]);
    }
    if rotations.last().is_none_or(|key| key[0] < duration) {
        let (sin, cos) = (last_turn / 2.0).sin_cos();
        rotations.push([duration, 0.0, 0.0, sin, cos]);
    }
    (translations, rotations)
}

/// The trigger names in the template's clips, for completion.
///
/// e.g., `SoundPlay.Foo:0.5` -> `SoundPlay.Foo`. Only `.bin` templates; else none.
pub(super) fn template_trigger_names(template_root: &Path) -> BTreeSet<String> {
    let path = template_root.join("meshes").join(format!("{ADSF}.bin"));
    let Ok(bytes) = std::fs::read(path) else { return BTreeSet::new() };
    let Ok(adsf) = rmp_serde::from_slice::<AltAdsf<'_>>(&bytes) else { return BTreeSet::new() };
    let blocks = adsf.0.values().flat_map(|project| project.clip_anim_blocks.values());
    blocks.flat_map(|block| &block.trigger_names).map(|trigger| trigger_name(trigger).to_owned()).collect()
}

/// The names of [`template_trigger_names`] and of the clip files of the work, sorted.
pub(super) fn trigger_names(template: &BTreeSet<String>, adsf: &Behavior) -> Vec<String> {
    // After name, id, speed, crop start, crop end and the count.
    const TRIGGERS_START: usize = 6;
    let clips = adsf.added.iter().filter(|(id, text)| AdsfFile::of(id).is_whole_clip(text));
    let lines = clips.flat_map(|(_, text)| text.lines().skip(TRIGGERS_START));
    let work: BTreeSet<&str> = lines.map(trigger_name).filter(|name| !name.is_empty()).collect();
    let all: BTreeSet<&str> = template.iter().map(String::as_str).chain(work).collect();
    all.into_iter().map(str::to_owned).collect()
}

/// `SoundPlay.Foo:0.5` -> `SoundPlay.Foo`
fn trigger_name(trigger: &str) -> &str {
    trigger.rsplit_once(':').map_or(trigger, |(name, _)| name).trim()
}

/// The number of clip `name` in `folder` if it has one, else the next free one.
///
/// e.g., `DefaultMale~1/Idle~para$2.txt` exists: `Idle` -> 2, a new clip -> 3.
fn clip_number(ids: &[&str], folder: &str, name: &str, code: &str) -> usize {
    let marker = format!("{code}$");
    let number_of = |id: &str| -> Option<usize> {
        let stem = id.strip_suffix(".txt")?;
        stem[stem.rfind(&marker)? + marker.len()..].parse().ok()
    };
    let existing = format!("{folder}/{name}~{marker}");
    if let Some(number) = ids.iter().find(|id| id.starts_with(&existing)).and_then(|id| number_of(id)) {
        return number;
    }
    ids.iter().filter_map(|id| number_of(id)).max().map_or(0, |max| max + 1)
}

/// `[directory crc, file name crc, extension crc]` of an animation for an anim set. Both are of the
/// lowercase path with backslashes.
///
/// e.g., `meshes\actors\character\Animations\A\B.hkx`: the crcs of
/// `meshes\actors\character\animations\a` and `b`.
fn anim_info(hkx: &str) -> Option<[String; 3]> {
    let path = hkx.to_lowercase();
    let (dir, file) = path.rsplit_once('\\')?;
    let (stem, _) = file.rsplit_once('.')?;
    Some([
        skyrim_crc::calc_crc32(dir).to_string(),
        skyrim_crc::calc_crc32(stem).to_string(),
        HKX_EXTENSION_CRC.to_owned(),
    ])
}

/// Adds the clip's two files per project to the `animationdatasinglefile` behavior.
fn add_clip(adsf: &mut Behavior, code: &str, clip: &ClipSource, info: &HkxInfo) {
    for project in &clip.projects {
        add_clip_files(adsf, code, &format!("{project}~{PROJECT_INDEX}"), &clip.name, info);
    }
}

/// Adds the clip's two files to `folder`, with the next free `<code>$<number>` (or the one it has).
/// Returns the id of the clip's file.
fn add_clip_files(adsf: &mut Behavior, code: &str, folder: &str, name: &str, info: &HkxInfo) -> String {
    let ids: Vec<&str> = adsf.added.keys().map(String::as_str).collect();
    let clip_id = format!("{code}${}", clip_number(&ids, folder, name, code));
    let file = format!("{folder}/{name}~{clip_id}.txt");
    adsf.added.insert(file.clone(), clip_anim_text(name, &clip_id, info));
    adsf.added.insert(format!("{folder}/{clip_id}.txt"), clip_motion_text(&clip_id, info.duration, &info.motion));
    file
}

/// New empty clips in `folder` (e.g. `DefaultMale~1`), numbered in order, to be filled by hand:
/// `count` of them named `<name>_0`, `<name>_1`, ..., or just `<name>` for one.
///
/// Returns the ids of the files added, of the clips and their motions.
pub(super) fn add_numbered_clips(
    adsf: &mut Behavior,
    code: &str,
    folder: &str,
    name: &str,
    count: usize,
) -> Vec<String> {
    // A second: the data needs a length, and the hkx decides the real one.
    let info = HkxInfo { duration: 1.0, triggers: Vec::new(), motion: Vec::new() };
    let before: std::collections::BTreeSet<String> = adsf.added.keys().cloned().collect();
    for i in 0..count {
        let name = if count == 1 { name.to_owned() } else { format!("{name}_{i}") };
        add_clip_files(adsf, code, folder, &name, &info);
    }
    adsf.added.keys().filter(|id| !before.contains(*id)).cloned().collect()
}

/// Which animations go into which anim set files. All of `hkx` go into all of `sets`.
#[derive(Debug, Default)]
pub(super) struct AnimSetPush<'a> {
    /// As [`anim_hkx_files`] gives them, e.g. `meshes\actors\character\animations\A\B.hkx`
    pub(super) hkx: Vec<&'a str>,
    /// Ids of anim sets, e.g. `DefaultMaleData~DefaultMale/1HMSolo.txt`
    pub(super) sets: Vec<&'a str>,
}

/// Adds the animations to the anim sets. Those already in a set stay once.
fn push_to_anim_sets(asdsf: &mut Behavior, push: &AnimSetPush<'_>) -> Result<(), String> {
    let infos: Vec<[String; 3]> = push.hkx.iter().filter_map(|hkx| anim_info(hkx)).collect();
    for id in &push.sets {
        let text = asdsf.text(id).ok_or_else(|| format!("{id}: not in the template"))?;
        let edited = nemesis_diff::asdsf::add_anim_infos(text, &infos).map_err(|e| format!("{id}: {e}"))?;
        asdsf.set_text(id, edited);
    }
    Ok(())
}

/// What is written. Each part is independent, and can be empty.
#[derive(Debug)]
pub(super) struct Generate<'a> {
    pub(super) code: &'a str,
    pub(super) clips: Vec<(&'a ClipSource, &'a HkxInfo)>,
    pub(super) push: AnimSetPush<'a>,
}

/// Writes the data into the workspace, adding a behavior only if there is something for it.
pub(super) fn generate(workspace: &mut Workspace, template_root: &Path, request: &Generate<'_>) -> Result<(), String> {
    if !request.clips.is_empty() {
        let adsf = workspace.anim_behavior(ADSF, template_root)?;
        for (clip, info) in &request.clips {
            add_clip(adsf, request.code, clip, info);
        }
    }
    if !request.push.hkx.is_empty() && !request.push.sets.is_empty() {
        push_to_anim_sets(workspace.anim_behavior(ASDSF, template_root)?, &request.push)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use gui_shared::settings::pages::patch::behavior::ParserMode;

    use super::{super::anim_patch::validate_asdsf, *};

    fn validate_adsf(id: &str, patch: &str) -> Result<(), String> {
        super::super::anim_patch::validate_adsf(id, patch, ParserMode::Strict)
    }

    fn info() -> HkxInfo {
        HkxInfo { duration: 3.5, triggers: vec![(0.03, "AnimObjDraw".to_owned())], motion: Vec::new() }
    }

    /// On a real mod (`PATCH_EDITOR_MOD`, its root): the hkx of its clips can be read, and the
    /// generated files pass the parsers.
    #[test]
    #[ignore = "needs a real mod"]
    fn generates_from_a_real_mod() {
        let selected = std::env::var("PATCH_EDITOR_MOD").unwrap();
        let templates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resource/assets/templates");
        let (workspace, code, _) = Workspace::import(Path::new(&selected), &templates).unwrap();
        let clips = clip_sources(&workspace.behaviors);
        assert!(!clips.is_empty());
        for clip in clips {
            let info = read_hkx(Path::new(&selected), &clip).unwrap();
            println!("{} {} {:.2}s {:?}", clip.name, clip.animation, info.duration, info.triggers);
            let id = format!("{code}$0");
            validate_adsf(&format!("x~1/{}~{id}.txt", clip.name), &clip_anim_text(&clip.name, &id, &info)).unwrap();
            validate_adsf(&format!("x~1/{id}.txt"), &clip_motion_text(&id, info.duration, &info.motion)).unwrap();
        }
    }

    /// On a real mod (`PATCH_EDITOR_MOD`, its root) and the real template: the hkx are found, and
    /// the push passes d-merge's parser.
    #[test]
    #[ignore = "needs a real mod"]
    fn pushes_the_hkx_of_a_real_mod() {
        let selected = std::env::var("PATCH_EDITOR_MOD").unwrap();
        let templates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resource/assets/templates");
        let hkx = anim_hkx_files(Path::new(&selected));
        assert!(!hkx.is_empty());
        println!("{} hkx, first: {}", hkx.len(), hkx[0]);

        let mut workspace = Workspace::default();
        let id = "DefaultMaleData~DefaultMale/1HMSolo.txt";
        let push = AnimSetPush { hkx: hkx.iter().map(String::as_str).collect(), sets: vec![id] };
        let request = Generate { code: "test", clips: Vec::new(), push };
        generate(&mut workspace, &templates, &request).unwrap();

        let asdsf = workspace.anim_behavior(ASDSF, &templates).unwrap();
        let patch = asdsf.patch("test", id).unwrap();
        asdsf.validate(id, &patch, ParserMode::Strict).unwrap();
        assert_eq!(patch.matches("<!-- MOD_CODE").count(), 1);
        println!("{}", &patch[patch.find("<!-- MOD_CODE").unwrap()..]);
    }

    /// What real mods write: one `MOD_CODE` block at the end, and the parser reads it as a push.
    #[test]
    fn pushing_to_an_anim_set_is_one_block_at_the_end() {
        let set = "V3\n0\n0\n0\n1\n1\n2\n7891816\n";
        let info = anim_info(r"meshes\actors\dragon\animations\ground_bite.hkx").unwrap();
        let edited = nemesis_diff::asdsf::add_anim_infos(set, &[info]).unwrap();
        let patch = nemesis_diff::object_patch("test", set, &edited).unwrap();
        assert_eq!(
            patch,
            "V3\n0\n0\n0\n1\n1\n2\n7891816\n<!-- MOD_CODE ~test~ OPEN -->\n3692944883\n3191128947\n7891816\n<!-- CLOSE -->\n"
        );
        validate_asdsf("DragonProjectData~DragonProject/FullBody.txt", &patch).unwrap();
    }

    #[test]
    fn only_hkx_in_animations_under_meshes_are_listed() {
        let path = |p: &str| anim_hkx_path(Path::new(p));
        assert_eq!(
            path("Meshes/actors/character/Animations/A/B.HKX").as_deref(),
            Some(r"Meshes\actors\character\Animations\A\B.HKX")
        );
        assert_eq!(path("meshes/actors/character/behaviors/0_master.hkx"), None);
        assert_eq!(path("meshes/actors/character/animations/readme.txt"), None);
        assert_eq!(path("animations/a.hkx"), None);
    }

    #[test]
    fn the_first_project_shown_follows_the_hkx() {
        let folders = ["AData~A", "DefaultMaleData~DefaultMale", "DraugrProjectData~DraugrProject"];
        assert_eq!(default_project(&folders, Some(r"meshes\actors\draugr\animations\a.hkx")), 2);
        assert_eq!(default_project(&folders, Some(r"meshes\actors\character\animations\a.hkx")), 1);
        assert_eq!(default_project(&folders, Some(r"meshes\actors\unknown\animations\a.hkx")), 1);
        assert_eq!(default_project(&folders, None), 1);
        assert_eq!(default_project(&["AData~A"], None), 0);
    }

    #[test]
    fn finds_the_projects_of_a_behavior() {
        assert_eq!(projects_of("meshes/actors/character/behaviors/0_master.bin"), ["DefaultMale", "DefaultFemale"]);
        assert_eq!(projects_of("meshes/actors/character/_1stperson/behaviors/0_master.bin"), ["FirstPerson"]);
        assert_eq!(projects_of("meshes/actors/draugr/behaviors/x.bin"), ["DraugrProject"]);
        assert!(projects_of("meshes/other.bin").is_empty());
    }

    #[test]
    fn reads_fields() {
        let text = "<hkparam name=\"name\">Idle</hkparam>\n<hkparam name=\"animationName\">a.hkx</hkparam>\n";
        assert_eq!(field(text, "name"), Some("Idle"));
        assert_eq!(field(text, "animationName"), Some("a.hkx"));
        assert_eq!(field(text, "other"), None);
    }

    /// The generated files must be what d-merge's patch parsers accept.
    #[test]
    fn generated_files_pass_the_parsers() {
        let anim = clip_anim_text("Idle", "para$2", &info());
        assert_eq!(anim, "Idle\npara$2\n1\n0\n0\n1\nAnimObjDraw:0.03\n");
        validate_adsf("DefaultMale~1/Idle~para$2.txt", &anim).unwrap();

        let motion = clip_motion_text("para$2", 3.5, &[]);
        assert_eq!(motion, "para$2\n3.5\n1\n3.5 0 0 0\n1\n3.5 0 0 0 1\n");
        validate_adsf("DefaultMale~1/para$2.txt", &motion).unwrap();

        let no_triggers = HkxInfo { duration: 1.0, triggers: Vec::new(), motion: Vec::new() };
        validate_adsf("DefaultMale~1/Idle~para$2.txt", &clip_anim_text("Idle", "para$2", &no_triggers)).unwrap();

        assert!(validate_adsf("DefaultMale~1/para$2.txt", "not a motion").is_err());
    }

    #[test]
    fn root_motion_becomes_keys() {
        // No samples, or none moving: one key at the end.
        assert_eq!(motion_keys(2.0, &[]), (vec![[2.0, 0.0, 0.0, 0.0]], vec![[2.0, 0.0, 0.0, 0.0, 1.0]]));
        let still = [[0.0; 4], [0.0; 4]];
        assert_eq!(motion_keys(2.0, &still).0, [[2.0, 0.0, 0.0, 0.0]]);

        // Three samples over 2 seconds: moves at 1 s, and then stays.
        let walk = [[0.0; 4], [0.0, 5.0, 0.0, 0.0], [0.0, 5.0, 0.0, 0.0]];
        assert_eq!(motion_keys(2.0, &walk).0, [[1.0, 0.0, 5.0, 0.0], [2.0, 0.0, 5.0, 0.0]]);

        // A half turn at the end: a quaternion around z.
        let turn = [[0.0; 4], [0.0, 0.0, 0.0, std::f32::consts::PI]];
        let (_, rotations) = motion_keys(1.0, &turn);
        assert_eq!(rotations.len(), 1);
        assert!((rotations[0][3] - 1.0).abs() < 1e-5 && rotations[0][4].abs() < 1e-5);

        validate_adsf("D~1/para$0.txt", &clip_motion_text("para$0", 2.0, &walk)).unwrap();
    }

    #[test]
    fn trigger_names_come_from_the_template_and_the_work() {
        assert_eq!(trigger_name("SoundPlay.Foo:0.5"), "SoundPlay.Foo");
        assert_eq!(trigger_name("NoTime"), "NoTime");

        let path = Path::new("meshes").join(format!("{ADSF}.bin"));
        let mut adsf = Behavior::open(&path).unwrap();
        adsf.added.insert("D~1/Idle~m$0.txt".to_owned(), "Idle\nm$0\n1\n0\n0\n1\nMine:0.5\n".to_owned());
        // A motion is no clip, though its folder has a `~`: its keys are not trigger names.
        let motion = "m$0\n1\n2\n0.5 0 0 0\n1 0 0 0\n2\n0.5 0 0 0 1\n1 0 0 0 1\n";
        adsf.added.insert("D~1/m$0.txt".to_owned(), motion.to_owned());
        let template = BTreeSet::from(["Theirs".to_owned()]);
        assert_eq!(trigger_names(&template, &adsf), ["Mine", "Theirs"]);
    }

    #[test]
    fn numbered_clips_pair_up_and_pass_the_parsers() {
        let path = Path::new("meshes").join(format!("{ADSF}.bin"));
        let mut adsf = Behavior::open(&path).unwrap();
        let added = add_numbered_clips(&mut adsf, "m", "D~1", "Idle", 2);
        assert_eq!(added, ["D~1/Idle_0~m$0.txt", "D~1/Idle_1~m$1.txt", "D~1/m$0.txt", "D~1/m$1.txt"]);
        for id in &added {
            validate_adsf(id, &adsf.added[id]).unwrap();
        }
        // The numbers go on, and one clip has no suffix.
        assert_eq!(add_numbered_clips(&mut adsf, "m", "D~1", "Run", 1), ["D~1/Run~m$2.txt", "D~1/m$2.txt"]);
        // A clip that exists is not added twice.
        assert!(add_numbered_clips(&mut adsf, "m", "D~1", "Run", 1).is_empty());
    }

    #[test]
    fn a_clip_keeps_its_number() {
        let ids = ["D~1/Idle~para$2.txt", "D~1/para$2.txt", "D~1/Run~para$0.txt", "D~1/para$0.txt"];
        assert_eq!(clip_number(&ids, "D~1", "Idle", "para"), 2);
        assert_eq!(clip_number(&ids, "D~1", "New", "para"), 3);
        assert_eq!(clip_number(&[], "D~1", "New", "para"), 0);
    }

    /// CRC32 of the lowercase path, as FNIS generation does it.
    #[test]
    fn anim_info_hashes_the_lowercase_path() {
        let [dir, stem, extension] = anim_info(r"meshes\actors\character\Animations\NickNak\Idle.hkx").unwrap();
        assert_eq!(dir, skyrim_crc::calc_crc32(r"meshes\actors\character\animations\nicknak").to_string());
        assert_eq!(stem, skyrim_crc::calc_crc32("idle").to_string());
        assert_eq!(extension, HKX_EXTENSION_CRC);
    }
}
