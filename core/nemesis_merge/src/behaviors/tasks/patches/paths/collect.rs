use std::path::{Path, PathBuf};

use rayon::prelude::*;
use winnow::{
    ModalResult, Parser,
    ascii::Caseless,
    combinator::{alt, fail, repeat, seq},
    error::{StrContext, StrContextValue},
    token::{any, take_while},
};
use winnow_ext::take_until_ext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Category {
    /// A regular Nemesis patch.
    Nemesis,

    /// An `animationsetdatasinglefile` patch.
    Adsf,

    /// An `animationdatasinglefile` patch.
    Asdsf,
}

/// Collects all relevant file paths within the given ID directory.
///
/// # Errors
/// Returns an error if path traversal fails.
pub(crate) fn collect_nemesis_paths(path: impl AsRef<Path>) -> Vec<(Category, PathBuf)> {
    jwalk::WalkDir::new(path)
        .into_iter()
        .par_bridge()
        .filter_map(|result| {
            let txt_path = {
                let path = result.ok()?.path();
                is_txt_file(&path).then_some(path)?
            };
            let txt_path_str = if let Some(path) = txt_path.to_str() {
                path
            } else {
                tracing::debug!(path = ?txt_path, "Skipping non-UTF-8 path");
                return None;
            };

            let category = if is_nemesis_file(&txt_path) {
                Category::Nemesis
            } else {
                classify_patch_path(txt_path_str)?
            };
            Some((category, txt_path))
        })
        .collect()
}

#[inline]
fn is_txt_file(path: &Path) -> bool {
    let is_txt = path.extension().is_some_and(|path| path.eq_ignore_ascii_case("txt"));
    let is_file = path.is_file();

    is_txt && is_file
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

/// Classifies a patch path.
fn classify_patch_path(input: &str) -> Option<Category> {
    patch_path.parse(input).ok()
}

fn patch_path(input: &mut &str) -> ModalResult<Category> {
    // Nemesis_Engine/mod/<mod_code>/
    seq! {
        take_until_ext(0.., Caseless("Nemesis_Engine")),
        Caseless("Nemesis_Engine").context(StrContext::Expected(StrContextValue::StringLiteral("Nemesis_Engine"))),
        path_sep1,
        Caseless("mod").context(StrContext::Expected(StrContextValue::StringLiteral("mod"))),
        path_sep1,
        take_while(1.., |c| !matches!(c, '/' | '\\')), // <mod_code>
        path_sep1,
    }
    .parse_next(input)?;

    let category = alt((
        Caseless("animationdatasinglefile").value(Category::Adsf),
        Caseless("animationsetdatasinglefile").value(Category::Asdsf),
        fail,
    ))
    .parse_next(input)?;

    path_sep1.parse_next(input)?;

    if category == Category::Asdsf {
        fail_if_manifest_patch.parse_next(input)?;
    }

    repeat::<_, _, (), _, _>(0.., any).parse_next(input)?;

    Ok(category)
}

// It is a valid path, but the patch itself targeting it is invalid.(HorsePower - Modernized Horse Riding (Total Riding Overhaul v1.2.0)
fn fail_if_manifest_patch(input: &mut &str) -> ModalResult<()> {
    let template_name = path_component.parse_next(input)?;
    path_sep1.parse_next(input)?;

    let file_name = path_component.parse_next(input)?;

    if template_name.eq_ignore_ascii_case("HorseProjectData~HorseProject")
        && file_name.eq_ignore_ascii_case("horseproject.txt")
    {
        return fail.parse_next(input);
    }

    Ok(())
}

/// Parses a single path component.
///
/// A component cannot contain `/` or `\`.
fn path_component<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    take_while(1.., |c: char| c != '/' && c != '\\').parse_next(input)
}

/// Parses 1 or more path separator.
///
/// Both `/` and `\` are accepted.
fn path_sep1(input: &mut &str) -> ModalResult<()> {
    take_while(1.., |c: char| c == '/' || c == '\\').void().parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_nemesis_patch_file() {
        let path = r"mod/Nemesis_engine/mod/id/shout_behavior/#id$0.txt";
        assert!(is_nemesis_file(path));
    }

    #[test]
    fn test_classify_anim_data_patch_files() {
        assert_eq!(
            patch_path.parse(
                r"Nemesis_Engine/mod/slide/animationdatasinglefile/DefaultFemale~1/SprintSlide~slide$0.txt"
            ).unwrap_or_else(|e| panic!("{e}")),
            Category::Adsf
        );

        assert_eq!(
            classify_patch_path(
                r"C:/MO2/mods/Nemesis_Engine/mod/slide/animationdatasinglefile/DefaultFemale~1/slide$0.txt"
            ),
            Some(Category::Adsf)
        );

        assert_eq!(
            classify_patch_path(
                r"Nemesis_Engine/mod/tkuc/animationdatasinglefile/FirstPerson~1/TKDodgeForward~791.txt"
            ),
            Some(Category::Adsf)
        );
    }

    #[test]
    fn test_classify_anim_set_data_patch_files() {
        assert_eq!(
            classify_patch_path(
                r"Nemesis_Engine//mod/slide/animationsetdatasinglefile/DefaultMaleData~DefaultMale/_MTSolo.txt"
            ),
            Some(Category::Asdsf)
        );

        assert_eq!(
            classify_patch_path(
                r"Nemesis_Engine/mod/slide/animationsetdatasinglefile/DefaultMaleData~DefaultMale/_MTSolo.txt"
            ),
            Some(Category::Asdsf)
        );
    }

    #[test]
    fn test_classify_invalid_patch_files() {
        assert_eq!(
            classify_patch_path(
                r"Nemesis_Engine//mod/slide/some_other_folder/DefaultFemale~1/slide$0.txt"
            ),
            None
        );

        assert_eq!(
            classify_patch_path(
                r"Nemesis_Engine//mod/slide/animation_data_single_file/DefaultFemale~1/slide$0.txt"
            ),
            None
        );

        // It is a valid path, but the patch itself targeting it is invalid.(HorsePower - Modernized Horse Riding (Total Riding Overhaul v1.2.0)
        assert_eq!(
            classify_patch_path(
                r"Nemesis_Engine/mod/hpmhr/animationsetdatasinglefile/HorseProjectData~HorseProject/horseproject.txt"
            ),
            None
        );
        assert_eq!(
            classify_patch_path(
                r"Nemesis_Engine/mod/hpmhr\animationsetdatasinglefile\HorseProjectData~HorseProject\horseproject.txt"
            ),
            None
        );
    }
}
