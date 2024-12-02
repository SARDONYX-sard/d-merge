use std::path::{Path, PathBuf};

/// Represents the parsed result of a Nemesis Engine input path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NemesisPath {
    /// The path relative to the Nemesis Engine directory,
    /// e.g., `mod/flinch/0_master/#0001.txt`
    pub relevant_path: PathBuf,
    /// The name of the template, e.g., `0_master`.
    pub template_name: String,
    /// Indicates whether the path includes the `_1stperson` directory.
    pub is_1stperson: bool,
}

/// Parses a Nemesis Engine input path and extracts the relevant path, template name,
/// and whether it is from the `_1stperson` directory.
///
/// # Arguments
/// - `input_path`: A path object representing the input file path.
///
/// # Returns
/// - `Some(OutputPathResult)` if the input path contains the `Nemesis_Engine` directory.
/// - `None` if the input path does not match the expected structure.
pub fn parse_input_nemesis_path(input_path: &Path) -> Option<NemesisPath> {
    // Locate the "Nemesis_Engine" directory in the path.
    let start_index = input_path
        .iter()
        .position(|component| component.eq_ignore_ascii_case("Nemesis_Engine"))?;

    // Extract the components after "Nemesis_Engine/mod".
    let relevant_path: PathBuf = input_path.iter().skip(start_index + 2).collect();

    // Detect the presence of "_1stperson" and construct the relevant path.
    let mut is_1st_person = false;
    for component in relevant_path.components() {
        if component.as_os_str().eq_ignore_ascii_case("_1stperson") {
            is_1st_person = true;
            break;
        }
    }

    // Extract the template name.
    let template_name = input_path
        .iter()
        .nth(start_index + if is_1st_person { 4 } else { 3 })?
        .to_string_lossy()
        .to_string();

    Some(NemesisPath {
        relevant_path,
        template_name,
        is_1stperson: is_1st_person,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_output_path_with_master_test() {
        let input_path = Path::new("/some/path/to/Nemesis_Engine/mod/flinch/0_master/#0106.txt");
        assert_eq!(
            parse_input_nemesis_path(input_path),
            Some(NemesisPath {
                relevant_path: Path::new("flinch/0_master/#0106.txt").to_path_buf(),
                template_name: "0_master".to_string(),
                is_1stperson: false,
            })
        );

        let input_path = Path::new("../Nemesis_Engine/mod/flinch/0_master/#0106.txt");
        assert_eq!(
            parse_input_nemesis_path(input_path),
            Some(NemesisPath {
                relevant_path: Path::new("flinch/0_master/#0106.txt").to_path_buf(),
                template_name: "0_master".to_string(),
                is_1stperson: false,
            })
        );

        let input_path =
            Path::new("/some/path/to/Nemesis_Engine/mod/flinch/_1stperson/0_master/#0106.txt");
        assert_eq!(
            parse_input_nemesis_path(input_path),
            Some(NemesisPath {
                relevant_path: Path::new("flinch/_1stperson/0_master/#0106.txt").to_path_buf(),
                template_name: "0_master".to_string(),
                is_1stperson: true,
            })
        );
    }
}
