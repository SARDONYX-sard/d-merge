use std::path::{Path, PathBuf};

use rayon::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Category {
    /// A regular Nemesis patch.
    Nemesis,

    /// An `animationsetdatasinglefile` patch.
    Adsf,

    /// An `animationdatasinglefile` patch.
    Asdsf,
}

/// Returns all patch files in one mod dir(e.g. `<data>/Nemesis_Engine/mod/<mod_code>`) with their category.
///
/// The category is decided by the dir layout, so there is no need to parse the full path:
/// - `#*.txt`(anywhere) => [`Category::Nemesis`]
///   - `<template>/`, `_1stperson/<template>/`, EngineExt `meshes/**/`
/// - `animationdatasinglefile/<project>/*.txt` => [`Category::Adsf`]
/// - `animationsetdatasinglefile/<project>/*.txt` => [`Category::Asdsf`]
///
/// # Note
/// Sub dirs are walked in parallel, and this can also be called inside `rayon::par_iter` per mod directory.
pub(crate) fn collect_nemesis_paths(
    mod_dir: &Path,
) -> impl ParallelIterator<Item = (Category, PathBuf)> + '_ {
    jwalk_glob::walk_files(mod_dir).filter_map(move |path| {
        let category = classify(mod_dir, &path)?;
        Some((category, path))
    })
}

/// Classifies a file in `mod_dir`.
fn classify(mod_dir: &Path, path: &Path) -> Option<Category> {
    if !is_txt_file(path) {
        return None;
    }
    if is_nemesis_file(path) {
        return Some(Category::Nemesis);
    }

    let top_dir = path.strip_prefix(mod_dir).ok()?.components().next()?.as_os_str().to_str()?;
    if top_dir.eq_ignore_ascii_case("animationdatasinglefile") {
        Some(Category::Adsf)
    } else if top_dir.eq_ignore_ascii_case("animationsetdatasinglefile") {
        (!is_manifest_patch(path)).then_some(Category::Asdsf)
    } else {
        None
    }
}

#[inline]
fn is_txt_file(file_name: impl AsRef<Path>) -> bool {
    file_name.as_ref().extension().is_some_and(|ext| ext.eq_ignore_ascii_case("txt"))
}

/// Check if the file name starts with a `#` and is a file.
///
/// # Assumption.
/// - The file is a file with a txt extension.
fn is_nemesis_file(path: impl AsRef<Path>) -> bool {
    path.as_ref()
        .file_stem()
        .is_some_and(|name| name.to_str().is_some_and(|name| name.starts_with('#')))
}

/// It is a valid path, but the patch itself targeting it is invalid.(HorsePower - Modernized Horse Riding (Total Riding Overhaul v1.2.0)
///
/// e.g. `animationsetdatasinglefile/HorseProjectData~HorseProject/horseproject.txt`
fn is_manifest_patch(path: &Path) -> bool {
    let name_eq = |path: Option<&Path>, expected: &str| {
        path.and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case(expected))
    };

    name_eq(Some(path), "horseproject.txt")
        && name_eq(path.parent(), "HorseProjectData~HorseProject")
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOD_DIR: &str = "C:/MO2/mods/slide/Nemesis_Engine/mod/slide";

    fn classify_in_mod(relative: &str) -> Option<Category> {
        let mod_dir = Path::new(MOD_DIR);
        classify(mod_dir, &mod_dir.join(relative))
    }

    #[test]
    fn test_classify_nemesis_patch_file() {
        assert_eq!(classify_in_mod("shout_behavior/#id$0.txt"), Some(Category::Nemesis));
        assert_eq!(classify_in_mod("_1stperson/0_master/#0106.txt"), Some(Category::Nemesis));
        assert_eq!(
            classify_in_mod("meshes/actors/character/behaviors/foo/#0001.txt"),
            Some(Category::Nemesis)
        );
    }

    #[test]
    fn test_classify_anim_data_patch_files() {
        assert_eq!(
            classify_in_mod("animationdatasinglefile/DefaultFemale~1/SprintSlide~slide$0.txt"),
            Some(Category::Adsf)
        );
        assert_eq!(
            classify_in_mod("AnimationDataSingleFile/FirstPerson~1/TKDodgeForward~791.txt"),
            Some(Category::Adsf)
        );
    }

    #[test]
    fn test_classify_anim_set_data_patch_files() {
        assert_eq!(
            classify_in_mod("animationsetdatasinglefile/DefaultMaleData~DefaultMale/_MTSolo.txt"),
            Some(Category::Asdsf)
        );
    }

    #[test]
    fn test_classify_invalid_patch_files() {
        assert_eq!(classify_in_mod("some_other_folder/DefaultFemale~1/slide$0.txt"), None);
        assert_eq!(classify_in_mod("animation_data_single_file/DefaultFemale~1/slide$0.txt"), None);
        assert_eq!(classify_in_mod("shout_behavior/readme.txt"), None);
        assert_eq!(classify_in_mod("info.ini"), None);

        // It is a valid path, but the patch itself targeting it is invalid.(HorsePower - Modernized Horse Riding (Total Riding Overhaul v1.2.0)
        assert_eq!(
            classify_in_mod(
                "animationsetdatasinglefile/HorseProjectData~HorseProject/horseproject.txt"
            ),
            None
        );
    }
}
