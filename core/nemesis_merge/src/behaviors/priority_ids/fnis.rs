use winnow::{
    ModalResult, Parser,
    ascii::Caseless,
    combinator::{alt, preceded, repeat, terminated},
    token::{any, take_while},
};
use winnow_ext::take_until_ext;

/// Parsed components of a `FNIS_*_List.txt` path.
///
/// The `behavior_object` is extracted from the **filename** rather than the
/// directory path, because some creatures share a `base_dir`
/// (e.g. `actors/canine` for both `dog` and `wolf`).
///
/// ## Filename format
/// - Humanoid: `FNIS_<namespace>_List.txt` — no `behavior_object` suffix
/// - Creature: `FNIS_<namespace>_<behavior_object>_List.txt`
///
/// ## `is_1st_person`
/// Only relevant when `behavior_object` is `None` (humanoid).
/// Determined from the path component immediately before `animations/`.
#[derive(Debug, PartialEq)]
pub(crate) struct FnisListPath<'a> {
    /// Creature behavior object key for phf lookup (e.g. `"wolf"`, `"dog"`).
    /// `None` for humanoid — use `is_1st_person` to select the HUMANOID entry.
    pub behavior_object: Option<&'a str>,

    /// `true` when the component before `animations/` is `_1stperson`.
    /// Only meaningful when `behavior_object` is `None`.
    pub is_1st_person: bool,

    /// The FNIS mod namespace (e.g. `FNISZoo`, `FNISFlyer`).
    pub namespace: &'a str,
}

/// Parse a `FNIS_*_List.txt` path and extract [`FnisListPath`].
///
/// Both `/` and `\` are accepted as path separators.
/// Returns `None` if the path does not match the expected structure.
pub(crate) fn parse_fnis_list_path(path: &str) -> Option<FnisListPath<'_>> {
    parse_fnis_components.parse(path).ok()
}
fn parse_fnis_components<'a>(input: &mut &'a str) -> ModalResult<FnisListPath<'a>> {
    // Capture everything up to `animations/`, extract the last component for `is_1st_person`
    let before_animations: &str = terminated(
        take_until_ext(1.., (alt(('/', '\\')), Caseless("animations"), alt(('/', '\\')))),
        (alt(('/', '\\')), Caseless("animations"), alt(('/', '\\'))),
    )
    .parse_next(input)?;

    let is_1st_person = before_animations
        .rsplit(['/', '\\'])
        .next()
        .is_some_and(|s| s.eq_ignore_ascii_case("_1stperson"));

    let namespace = terminated(take_while(1.., |c| !matches!(c, '/' | '\\')), alt(('/', '\\')))
        .parse_next(input)?;

    // filename: `FNIS_<namespace>[_<behavior_object>]_List.txt`
    Caseless("FNIS_").parse_next(input)?;
    Caseless(namespace).parse_next(input)?;

    // `_<behavior_object>_List.txt` or `_List.txt`
    let behavior_object: Option<&str> = alt((
        // creature: `_<behavior_object>_List.txt`
        preceded(
            '_',
            terminated(take_until_ext(1.., Caseless("_List.txt")), Caseless("_List.txt")),
        )
        .map(Some),
        // humanoid: `_List.txt`
        Caseless("_List.txt").map(|_| None),
    ))
    .parse_next(input)?;

    repeat::<_, _, (), _, _>(0.., any).parse_next(input)?;

    Ok(FnisListPath { behavior_object, is_1st_person, namespace })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(path: &str) -> FnisListPath<'_> {
        parse_fnis_list_path(path).unwrap_or_else(|| panic!("failed to parse: {path}"))
    }

    #[test]
    fn humanoid_character() {
        let p = parse(
            "C:/Skyrim/Data/meshes/actors/character/animations/FNISFlyer/FNIS_FNISFlyer_List.txt",
        );
        assert_eq!(
            p,
            FnisListPath { behavior_object: None, is_1st_person: false, namespace: "FNISFlyer" }
        );
    }

    #[test]
    fn humanoid_1st_person() {
        let p = parse(
            "C:/Skyrim/Data/meshes/actors/character/_1stperson/animations/FNISFlyer/FNIS_FNISFlyer_List.txt",
        );
        assert_eq!(
            p,
            FnisListPath { behavior_object: None, is_1st_person: true, namespace: "FNISFlyer" }
        );
    }

    #[test]
    fn creature_wolf() {
        let p = parse(
            "C:/Skyrim/Data/meshes/actors/canine/animations/FNISZoo/FNIS_FNISZoo_wolf_List.txt",
        );
        assert_eq!(
            p,
            FnisListPath {
                behavior_object: Some("wolf"),
                is_1st_person: false,
                namespace: "FNISZoo"
            }
        );
    }

    #[test]
    fn creature_dog() {
        let p = parse(
            "C:/Skyrim/Data/meshes/actors/canine/animations/FNISZoo/FNIS_FNISZoo_dog_List.txt",
        );
        assert_eq!(
            p,
            FnisListPath {
                behavior_object: Some("dog"),
                is_1st_person: false,
                namespace: "FNISZoo"
            }
        );
    }

    #[test]
    fn dlc_creature_vampirebrute() {
        let p = parse(
            "C:/Skyrim/Data/meshes/actors/dlc01/vampirebrute/animations/FNISZoo/FNIS_FNISZoo_vampirebrute_List.txt",
        );
        assert_eq!(
            p,
            FnisListPath {
                behavior_object: Some("vampirebrute"),
                is_1st_person: false,
                namespace: "FNISZoo"
            }
        );
    }

    #[test]
    fn auxbones_tail() {
        let p = parse(
            "C:/Skyrim/Data/meshes/auxbones/tail/animations/FNISZoo/FNIS_FNISZoo_tail_List.txt",
        );
        assert_eq!(
            p,
            FnisListPath {
                behavior_object: Some("tail"),
                is_1st_person: false,
                namespace: "FNISZoo"
            }
        );
    }

    #[test]
    fn dlc_plant_caveworm() {
        let p = parse(
            "C:/Skyrim/Data/meshes/dlc01/plants/caveworm/animations/FNISZoo/FNIS_FNISZoo_caveworm_List.txt",
        );
        assert_eq!(
            p,
            FnisListPath {
                behavior_object: Some("caveworm"),
                is_1st_person: false,
                namespace: "FNISZoo"
            }
        );
    }

    #[test]
    fn backslash_separator() {
        let p = parse(
            r"C:\Skyrim\Data\meshes\actors\character\animations\FNISFlyer\FNIS_FNISFlyer_List.txt",
        );
        assert_eq!(
            p,
            FnisListPath { behavior_object: None, is_1st_person: false, namespace: "FNISFlyer" }
        );
    }

    #[test]
    fn backslash_1st_person() {
        let p = parse(
            r"C:\Skyrim\Data\meshes\actors\character\_1stperson\animations\FNISFlyer\FNIS_FNISFlyer_List.txt",
        );
        assert_eq!(
            p,
            FnisListPath { behavior_object: None, is_1st_person: true, namespace: "FNISFlyer" }
        );
    }

    #[test]
    fn mo2_manual_mode() {
        let p = parse(
            "C:/MO2/mods/FNISZoo/meshes/actors/canine/animations/FNISZoo/FNIS_FNISZoo_dog_List.txt",
        );
        assert_eq!(
            p,
            FnisListPath {
                behavior_object: Some("dog"),
                is_1st_person: false,
                namespace: "FNISZoo"
            }
        );
    }

    #[test]
    fn relative_path() {
        let p = parse(
            r"..\..\dummy\fnis_test_mods\FNIS Zoo 5.0.1\Meshes\actors\dlc02\riekling\animations\FNISZoo\FNIS_FNISZoo_riekling_List.txt",
        );
        assert_eq!(
            p,
            FnisListPath {
                behavior_object: Some("riekling"),
                is_1st_person: false,
                namespace: "FNISZoo"
            }
        );
    }

    #[test]
    fn invalid_missing_animations() {
        assert!(
            parse_fnis_list_path("meshes/actors/character/FNISFlyer/FNIS_FNISFlyer_List.txt")
                .is_none()
        );
    }
}
