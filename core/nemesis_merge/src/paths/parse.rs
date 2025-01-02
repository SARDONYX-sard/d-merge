use snafu::OptionExt as _;
use std::path::Path;

/// Represents the parsed result of a Nemesis Engine input path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NemesisPath {
    /// The path relative to the Nemesis Engine directory,
    /// - e.g., `mod/flinch/0_master/#0001.txt`
    pub mod_code: String,
    /// hkx file stem.
    /// - e.g., `0_master`, `_1stperson/0_master`.
    pub template_name: String,
    /// class index
    ///
    /// e.g., `#0001.txt` -> `#0001`
    pub index: String,
}

#[derive(Debug, PartialEq, Eq, snafu::Snafu)]
#[snafu(visibility(pub))]
#[allow(clippy::enum_variant_names)]
pub enum NemesisPathError {
    NotFoundFileStem,
    /// Path does not contain 'Nemesis_Engine'
    NotContainEngineDir,
    NotFoundTemplateName,
    NotFoundIndexName,
}

type Result<T, E = NemesisPathError> = core::result::Result<T, E>;

/// Parses a Nemesis path from a `&Path`.
pub fn get_nemesis_id(path: &Path) -> Result<String> {
    let engine_index = path
        .iter()
        .position(|component| component.eq_ignore_ascii_case("Nemesis_Engine"))
        .with_context(|| NotContainEngineDirSnafu)?;

    let mut path = path.iter().skip(engine_index + 2);
    let mod_code = path
        .next()
        .map(|path| path.to_string_lossy().to_string())
        .ok_or(NemesisPathError::NotFoundTemplateName)?;

    Ok(mod_code)
}

/// Parses a Nemesis path from a `&Path`.
pub fn parse_nemesis_path(path: &Path) -> Result<NemesisPath> {
    let engine_index = path
        .iter()
        .position(|component| component.eq_ignore_ascii_case("Nemesis_Engine"))
        .with_context(|| NotContainEngineDirSnafu)?;
    let file_stem = path
        .file_stem()
        .map(|path| path.to_string_lossy().to_string())
        .with_context(|| NotFoundFileStemSnafu)?;

    let mut path = path.iter().skip(engine_index + 2);

    let mod_code = path
        .next()
        .map(|path| path.to_string_lossy().to_string())
        .ok_or(NemesisPathError::NotFoundTemplateName)?;

    let template_name = path.next();
    let template_name = if template_name
        .map(|path| path.eq_ignore_ascii_case("_1stperson"))
        .unwrap_or_default()
    {
        path.next()
            .map(|path| format!("_1stperson/{}", path.to_string_lossy()))
    } else {
        template_name.map(|path| path.to_string_lossy().to_string())
    }
    .ok_or(NemesisPathError::NotFoundTemplateName)?;

    Ok(NemesisPath {
        mod_code,
        template_name,
        index: file_stem,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_nemesis_path_valid() {
        let input_path = Path::new("/some/path/to/Nemesis_Engine/mod/flinch/0_master/#0106.txt");
        assert_eq!(
            parse_nemesis_path(input_path),
            Ok(NemesisPath {
                mod_code: "flinch".to_string(),
                template_name: "0_master".to_string(),
                index: "#0106".to_string(),
            })
        );

        let input_path = Path::new("../Nemesis_Engine/mod/flinch/0_master/#0106.txt");
        assert_eq!(
            parse_nemesis_path(input_path),
            Ok(NemesisPath {
                mod_code: "flinch".to_string(),
                template_name: "0_master".to_string(),
                index: "#0106".to_string(),
            })
        );

        let input_path =
            Path::new("/some/path/to/Nemesis_Engine/mod/flinch/_1stperson/0_master/#0106.txt");
        assert_eq!(
            parse_nemesis_path(input_path),
            Ok(NemesisPath {
                mod_code: "flinch".to_string(),
                template_name: "_1stperson/0_master".to_string(),
                index: "#0106".to_string(),
            })
        );
    }

    #[test]
    fn parse_nemesis_path_invalid() {
        let input_path = Path::new("/invalid/path/to/Engine/mod/flinch/0_master/#0106.txt");
        assert!(parse_nemesis_path(input_path).is_err());

        let input_path = Path::new("Nemesis_Engine/mod/flinch");
        assert!(parse_nemesis_path(input_path).is_err());
    }
}
