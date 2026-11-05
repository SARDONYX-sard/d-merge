//! What a patch file of the two animation data templates is, told by its path, and its parser check.
//!
//! The path decides how d-merge reads the file, so the rules must stay the same as
//! `behavior_merge`'s `tasks/adsf/path_parser.rs` and `tasks/asdsf/path_parser.rs`, and the parsers as
//! `parse_anim_data_patch` next to them. An id here is the path below the template's folder:
//!
//! `animationdatasinglefile/`
//! | Id                               | Kind                 | Contents                              |
//! | -------------------------------- | -------------------- | ------------------------------------- |
//! | `$header$/$header$.txt`          | `ProjectNamesHeader` | the project list, with `MOD_CODE`     |
//! | `DefaultMale~1/$header$.txt`     | `AnimHeader`         | a project's header, with `MOD_CODE`   |
//! | `DefaultMale~1/Slide~slide$0.txt`| `AddAnim`            | a new clip; the id is assigned later  |
//! | `FirstPerson~1/Dodge~348.txt`    | `IndexedAnim`        | `MOD_CODE`: an edit; else that clip   |
//! | `DefaultMale~1/slide$0.txt`      | `AddMotion`          | a new motion; the id is assigned later|
//! | `DefaultMale~1/50.txt`           | `IndexedMotion`      | `MOD_CODE`: an edit; else that motion |
//!
//! `animationsetdatasinglefile/`
//! | Id                                               | Kind             | Contents                  |
//! | ------------------------------------------------ | ---------------- | ------------------------- |
//! | `$header$/$header$.txt`                          | `ProjectsHeader` | the project list          |
//! | `DefaultMaleData~DefaultMale/$header$.txt`       | `SetsHeader`     | a project's set file list |
//! | `DefaultMaleData~DefaultMale/tkuc$Start.txt`     | `AddSet`         | a whole new set `Start.txt`|
//! | `DefaultMaleData~DefaultMale/_MTSolo.txt`        | `EditSet`        | a set, with `MOD_CODE`    |

use gui_shared::settings::pages::patch::behavior::ParserMode;
use skyrim_anim_data::{
    adsf::patch::de::{
        add::{parse_clip_anim_block_patch, parse_clip_motion_block_patch},
        anim_header::deserializer::parse_anim_header_diff_patch,
        others::{
            clip_anim::deserializer::parse_clip_anim_diff_patch,
            clip_motion::deserializer::parse_clip_motion_diff_patch,
        },
    },
    asdsf::{normal::de::anim_set_data, patch::de::deserializer::parse_anim_set_diff_patch},
    diff_line::deserializer::parse_lines_diff_patch,
};
use winnow::Parser as _;

/// The file name of the header patches, without `.txt`.
const HEADER: &str = "$header$";

/// `DefaultMale~1/Slide~slide$0.txt` -> `DefaultMale~1`, `Slide~slide$0.txt`
fn split_id(id: &str) -> (&str, &str) {
    id.rsplit_once('/').unwrap_or(("", id))
}

/// A file of `animationdatasinglefile`, see the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AdsfFile {
    ProjectNamesHeader,
    AnimHeader,
    AddAnim,
    IndexedAnim,
    AddMotion,
    IndexedMotion,
}

impl AdsfFile {
    /// Only the file's own name counts: the folder of nearly all of them has a `~` too.
    pub(super) fn of(id: &str) -> Self {
        let (folder, file) = split_id(id);
        let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
        if stem.eq_ignore_ascii_case(HEADER) {
            return if folder.eq_ignore_ascii_case(HEADER) { Self::ProjectNamesHeader } else { Self::AnimHeader };
        }
        match stem.rsplit_once('~') {
            Some((_, clip_id)) if clip_id.contains('$') => Self::AddAnim,
            Some(_) => Self::IndexedAnim,
            None if stem.contains('$') => Self::AddMotion,
            None => Self::IndexedMotion,
        }
    }

    /// Does it hold a whole clip, as opposed to its motion, a header or an edit?
    pub(super) fn is_whole_clip(self, patch: &str) -> bool {
        match self {
            Self::AddAnim => true,
            Self::IndexedAnim => !is_edit(patch),
            Self::ProjectNamesHeader | Self::AnimHeader | Self::AddMotion | Self::IndexedMotion => false,
        }
    }
}

/// An `Indexed*` file is an edit if it has `MOD_CODE`, else it is whole.
fn is_edit(patch: &str) -> bool {
    patch.contains("<!-- MOD_CODE")
}

/// A file of `animationsetdatasinglefile`, see the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AsdsfFile {
    ProjectsHeader,
    SetsHeader,
    AddSet,
    EditSet,
}

impl AsdsfFile {
    pub(super) fn of(id: &str) -> Self {
        let (folder, file) = split_id(id);
        if file.rsplit_once('.').is_some_and(|(stem, _)| stem.eq_ignore_ascii_case(HEADER)) {
            if folder.eq_ignore_ascii_case(HEADER) { Self::ProjectsHeader } else { Self::SetsHeader }
        } else if file.contains('$') {
            Self::AddSet
        } else {
            Self::EditSet
        }
    }
}

/// d-merge's parser check of one `animationdatasinglefile` patch, in the mode of the patch page.
///
/// The mode only matters for whole clips and motions. `Lenient` takes a wrong count (`7` triggers,
/// none listed): d-merge counts them itself, so that is neither an error nor a warning here, as a
/// message about what will work anyway would only hide the real ones.
pub(super) fn validate_adsf(id: &str, patch: &str, mode: ParserMode) -> Result<(), String> {
    let text = |e: &dyn std::fmt::Display| e.to_string();
    let strict = matches!(mode, ParserMode::Strict);
    match AdsfFile::of(id) {
        AdsfFile::ProjectNamesHeader => parse_lines_diff_patch(patch, 0).map(|_| ()).map_err(|e| text(&e)),
        AdsfFile::AnimHeader => parse_anim_header_diff_patch(patch).map(|_| ()).map_err(|e| text(&e)),
        AdsfFile::IndexedAnim if is_edit(patch) => {
            parse_clip_anim_diff_patch(patch, 0).map(|_| ()).map_err(|e| text(&e))
        }
        AdsfFile::AddAnim | AdsfFile::IndexedAnim if strict => {
            parse_clip_anim_block_patch::<true>(patch).map(|_| ()).map_err(|e| text(&e))
        }
        AdsfFile::AddAnim | AdsfFile::IndexedAnim => {
            parse_clip_anim_block_patch::<false>(patch).map(|_| ()).map_err(|e| text(&e))
        }
        AdsfFile::IndexedMotion if is_edit(patch) => {
            parse_clip_motion_diff_patch(patch, 0).map(|_| ()).map_err(|e| text(&e))
        }
        AdsfFile::AddMotion | AdsfFile::IndexedMotion if strict => {
            parse_clip_motion_block_patch::<true>(patch).map(|_| ()).map_err(|e| text(&e))
        }
        AdsfFile::AddMotion | AdsfFile::IndexedMotion => {
            parse_clip_motion_block_patch::<false>(patch).map(|_| ()).map_err(|e| text(&e))
        }
    }
}

/// d-merge's parser check of one `animationsetdatasinglefile` patch. These parsers have no modes.
pub(super) fn validate_asdsf(id: &str, patch: &str) -> Result<(), String> {
    let text = |e: &dyn std::fmt::Display| e.to_string();
    match AsdsfFile::of(id) {
        AsdsfFile::ProjectsHeader | AsdsfFile::SetsHeader => {
            parse_lines_diff_patch(patch, 0).map(|_| ()).map_err(|e| text(&e))
        }
        AsdsfFile::AddSet => anim_set_data.parse(patch).map(|_| ()).map_err(|e| text(&e)),
        AsdsfFile::EditSet => parse_anim_set_diff_patch(patch, 0).map(|_| ()).map_err(|e| text(&e)),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    fn validate_adsf(id: &str, patch: &str) -> Result<(), String> {
        super::validate_adsf(id, patch, ParserMode::Strict)
    }

    /// Every file under `dir`, as `<project>/<file>` ids with their paths.
    fn patch_files(dir: &Path) -> Vec<(String, PathBuf)> {
        let mut files = Vec::new();
        for project in std::fs::read_dir(dir).unwrap().flatten().map(|entry| entry.path()) {
            let Ok(entries) = std::fs::read_dir(&project) else { continue };
            for file in entries.flatten().map(|entry| entry.path()).filter(|path| path.is_file()) {
                let name = |path: &Path| path.file_name().unwrap().to_string_lossy().into_owned();
                files.push((format!("{}/{}", name(&project), name(&file)), file));
            }
        }
        files
    }

    /// On all the mods in a folder (`PATCH_EDITOR_MODS`): how many patch files each check rejects.
    /// Those files fail in d-merge too, so this finds wrong kinds only as a sudden high count.
    /// Clip data is checked in `Lenient` here; [`each_adsf_kind_uses_its_parser`] covers `Strict`.
    #[test]
    #[ignore = "needs real mods"]
    fn checks_the_patches_of_real_mods() {
        type Check = fn(&str, &str) -> Result<(), String>;
        let lenient: Check = |id, patch| super::validate_adsf(id, patch, ParserMode::Lenient);
        let strict: Check = validate_adsf;
        let checks: [(&str, &str, Check); 3] = [
            ("animationdatasinglefile", "strict", strict),
            ("animationdatasinglefile", "lenient", lenient),
            ("animationsetdatasinglefile", "", validate_asdsf),
        ];
        let mods = std::env::var("PATCH_EDITOR_MODS").unwrap();
        for (folder, mode, check) in checks {
            let (mut total, mut rejected) = (0, 0);
            for mod_dir in std::fs::read_dir(&mods).unwrap().flatten().map(|entry| entry.path()) {
                let Ok(codes) = std::fs::read_dir(mod_dir.join("Nemesis_Engine").join("mod")) else { continue };
                for code in codes.flatten().map(|entry| entry.path().join(folder)).filter(|dir| dir.is_dir()) {
                    for (id, path) in patch_files(&code) {
                        let bytes = std::fs::read(&path).unwrap();
                        let text = String::from_utf8_lossy(&bytes).replace("\r\n", "\n");
                        total += 1;
                        if let Err(err) = check(&id, &text) {
                            rejected += 1;
                            let first = err.lines().next().unwrap_or_default();
                            println!("{}: {first}", path.display());
                        }
                    }
                }
            }
            println!("{folder} {mode}: {rejected} rejected of {total}");
        }
    }

    /// The table in the module docs, which are `behavior_merge`'s `path_parser` tests.
    #[test]
    fn adsf_files_are_told_by_their_own_name() {
        assert_eq!(AdsfFile::of("$header$/$header$.txt"), AdsfFile::ProjectNamesHeader);
        assert_eq!(AdsfFile::of("DefaultMale~3/$header$.txt"), AdsfFile::AnimHeader);
        assert_eq!(AdsfFile::of("Default~1/RunForward~slide$2.txt"), AdsfFile::AddAnim);
        assert_eq!(AdsfFile::of("FirstPerson~1/TKDodgeRight~348.txt"), AdsfFile::IndexedAnim);
        assert_eq!(AdsfFile::of("Default~1/slide$10.txt"), AdsfFile::AddMotion);
        assert_eq!(AdsfFile::of("Default~1/10.txt"), AdsfFile::IndexedMotion);
    }

    #[test]
    fn asdsf_files_are_told_by_their_own_name() {
        assert_eq!(AsdsfFile::of("$header$/$header$.txt"), AsdsfFile::ProjectsHeader);
        assert_eq!(AsdsfFile::of("NemesisData~NemesisProject/$header$.txt"), AsdsfFile::SetsHeader);
        assert_eq!(AsdsfFile::of("DefaultFemaleData~DefaultFemale/tkuc$PlayerFNIS1Start.txt"), AsdsfFile::AddSet);
        assert_eq!(AsdsfFile::of("DefaultMaleData~DefaultMale/_MTSolo.txt"), AsdsfFile::EditSet);
    }

    #[test]
    fn an_indexed_clip_is_whole_without_mod_code() {
        assert!(AdsfFile::IndexedAnim.is_whole_clip("TKDodgeRight\n348\n1\n0\n0\n0\n"));
        assert!(!AdsfFile::IndexedAnim.is_whole_clip("<!-- MOD_CODE ~m~ OPEN -->\n"));
        assert!(!AdsfFile::AddMotion.is_whole_clip("m$0\n"));
    }

    #[test]
    fn each_adsf_kind_uses_its_parser() {
        let clip = "Idle\npara$2\n1\n0\n0\n1\nAnimObjDraw:0.03\n";
        validate_adsf("D~1/Idle~para$2.txt", clip).unwrap();
        let indexed = "Idle\n348\n1\n0\n0\n0\n";
        validate_adsf("D~1/Idle~348.txt", indexed).unwrap();
        let motion = "para$2\n3.5\n1\n3.5 0 0 0\n1\n3.5 0 0 0 1\n";
        validate_adsf("D~1/para$2.txt", motion).unwrap();
        validate_adsf("D~1/50.txt", "50\n3.5\n1\n3.5 0 0 0\n1\n3.5 0 0 0 1\n").unwrap();

        assert!(validate_adsf("D~1/para$2.txt", "not a motion").is_err());
        assert!(validate_adsf("D~1/50.txt", "not a motion").is_err());
        assert!(validate_adsf("D~1/Idle~348.txt", "not a clip").is_err());
    }

    /// A real one (BFCO): 7 triggers are announced and none follow.
    #[test]
    fn only_strict_rejects_a_wrong_count() {
        let (id, clip) = ("D~1/Attack~bfco$100.txt", "Attack\nbfco$100\n1\n0\n0\n7\n");
        assert!(super::validate_adsf(id, clip, ParserMode::Strict).is_err());
        super::validate_adsf(id, clip, ParserMode::Lenient).unwrap();
        // What is not a clip at all is wrong in both.
        assert!(super::validate_adsf(id, "not a clip", ParserMode::Lenient).is_err());
    }

    #[test]
    fn each_asdsf_kind_uses_its_parser() {
        validate_asdsf("D~D/x$New.txt", "V3\n0\n0\n0\n1\n1\n2\n7891816\n").unwrap();
        assert!(validate_asdsf("D~D/x$New.txt", "not a set").is_err());
        assert!(validate_asdsf("D~D/x.txt", "not a set").is_err());
    }
}
