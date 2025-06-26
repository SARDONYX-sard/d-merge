use std::path::{Path, PathBuf};

use rayon::prelude::*;

pub enum Category {
    Nemesis,
    Adsf,
    Asdsf,
}

/// Collects all relevant file paths within the given ID directory.
///
/// # Errors
/// Returns an error if path traversal fails.
pub fn collect_nemesis_paths(path: impl AsRef<Path>) -> Vec<(Category, PathBuf)> {
    jwalk::WalkDir::new(path)
        .into_iter()
        .par_bridge()
        .filter_map(|result| {
            let txt_path = {
                let path = result.ok()?.path();
                is_txt_file(&path).then_some(path)?
            };

            if is_nemesis_file(&txt_path) {
                return Some((Category::Nemesis, txt_path));
            }
            if is_adsf_patch_file(&txt_path) {
                return Some((Category::Adsf, txt_path));
            }
            if is_asdsf_patch_file(&txt_path) {
                return Some((Category::Asdsf, txt_path));
            }

            None
        })
        .collect()
}

#[inline]
fn is_txt_file(path: &Path) -> bool {
    let is_txt = path
        .extension()
        .is_some_and(|path| path.eq_ignore_ascii_case("txt"));
    let is_file = path.is_file();

    is_txt && is_file
}

/// Check if the file name starts with a `#` and is a file.
///
/// # Assumption.
/// - The file is a file with a txt extension.
fn is_nemesis_file(path: &Path) -> bool {
    let is_sharp_prefix = path
        .file_stem()
        .is_some_and(|name| name.to_str().is_some_and(|name| name.starts_with('#')));

    is_sharp_prefix
}

/// Check `<name>~<anim_data_clip_id>.txt` or `<anim_data_clip_id>.txt` format.
///
/// # Assumption.
/// - The file is a file with a txt extension.
fn is_adsf_patch_file(txt_path: &Path) -> bool {
    // Check if any parent directory in the last 3 components is "animationdatasinglefile"
    let has_adsf_parent = txt_path
        .ancestors()
        .take(3) // includes self, parent, grandparent, great-grandparent
        .any(|ancestor| {
            ancestor
                .file_name() // Intend: Get the final component dir name.
                .is_some_and(|name| name.eq_ignore_ascii_case("animationdatasinglefile"))
        });
    if !has_adsf_parent {
        return false;
    }

    // File stem should be non-empty and optionally contain a ~
    txt_path.file_stem().is_some_and(|s| !s.is_empty()) // Allow either `<clip>~<anim_data_id>` or just `<anim_data_id>`
}

// Check if any parent directory in the last 3 components is "animationdatasinglefile"
///
/// # Assumption.
/// - The file is a file with a txt extension.
fn is_asdsf_patch_file(txt_path: &Path) -> bool {
    let has_adsf_parent = txt_path
        .ancestors()
        .take(3) // includes self, parent, grandparent, great-grandparent
        .any(|ancestor| {
            ancestor
                .file_name() // Intend: Get the final component dir name.
                .is_some_and(|name| name.eq_ignore_ascii_case("animationsetdatasinglefile"))
        });
    if !has_adsf_parent {
        return false;
    }

    // File stem should be non-empty and optionally contain a ~
    txt_path.file_stem().is_some_and(|s| !s.is_empty()) // Allow either `<clip>~<anim_data_id>` or just `<anim_data_id>`
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_is_nemesis_patch_file() {
        let path = r"mod/Nemesis_engine/mod/id/shout_behavior/#id$0.txt";
        assert!(is_nemesis_file(Path::new(path)));
    }

    #[test]
    fn test_is_adsf_patch_file_valid_cases() {
        // Pattern: <clip_id>~<anim_data_id>.txt
        assert!(is_adsf_patch_file(Path::new(
            r"/mod/slide/animationdatasinglefile/DefaultFemale~1/SprintSlide~slide$0.txt"
        )));

        // Pattern: <anim_data_id>.txt
        assert!(is_adsf_patch_file(Path::new(
            r"/mod/slide/animationdatasinglefile/DefaultFemale~1/slide$0.txt"
        )));
    }

    #[test]
    fn test_is_asdsf_patch_file_valid_cases() {
        // Pattern: <clip_id>~<anim_data_id>.txt
        assert!(is_adsf_patch_file(Path::new(
            r"/mod/slide/animationsetdatasinglefile/DefaultFemale~1/SprintSlide~slide$0.txt"
        )));

        // Pattern: <anim_data_id>.txt
        assert!(is_adsf_patch_file(Path::new(
            r"/mod/slide/animationdatasinglefile/DefaultFemale~1/slide$0.txt"
        )));
    }

    #[test]
    fn test_is_adsf_patch_file_wrong_directory() {
        assert!(!is_adsf_patch_file(Path::new(
            r"/mod/slide/some_other_folder/DefaultFemale~1/slide$0.txt"
        )));
    }

    #[test]
    fn test_is_adsf_patch_file_partial_match() {
        // Not contains "animationdatasinglefile" in the directory path
        assert!(!is_adsf_patch_file(Path::new(
            r"/mod/slide/animation_data_single_file/DefaultFemale~1/slide$0.txt"
        )));
    }
}
