//! The templates there are to patch.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use super::{
    TemplateScope,
    behavior::ASDSF,
    locations::{meshes_path, nemesis_folder},
    read_dir,
};

/// `meshes/` relative, lowercase, `/` separated, `.bin` (what Nemesis / d-merge key templates by).
///
/// e.g. `C:/resource/xml/templates/meshes/actors/Draugr/behaviors/draugrbehavior.xml`
/// -> `meshes/actors/draugr/behaviors/draugrbehavior.bin`
/// A template file found by [`catalog`].
#[derive(Debug, Clone)]
pub(crate) struct TemplateEntry {
    /// e.g. `meshes/actors/draugr/behaviors/draugrbehavior.bin`
    pub(crate) meshes_path: String,
    pub(crate) path: PathBuf,
    /// Nemesis supports it (else only NemesisExt can patch it).
    pub(crate) nemesis: bool,
}

/// Every `.bin` / `.xml` template under `template_dir` that a work of `scope` can patch (sorted; a
/// `.bin` wins over its `.xml`).
pub(crate) fn catalog(template_dir: &Path, scope: TemplateScope) -> Vec<TemplateEntry> {
    let mut found: BTreeMap<String, TemplateEntry> = BTreeMap::new();
    let mut stack = vec![template_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = read_dir(&dir) else { continue };
        for path in entries {
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
            let is_asdsf_txt =
                ext.as_deref() == Some("txt") && path.file_stem().is_some_and(|s| s.eq_ignore_ascii_case(ASDSF));
            if !matches!(ext.as_deref(), Some("bin" | "xml")) && !is_asdsf_txt {
                continue;
            }
            let meshes_path = meshes_path(&path);
            if !meshes_path.starts_with("meshes/") {
                continue;
            }
            let is_bin = ext.as_deref() == Some("bin");
            if is_bin || !found.contains_key(&meshes_path) {
                let nemesis = nemesis_folder(&meshes_path).is_some();
                found.insert(meshes_path.clone(), TemplateEntry { meshes_path, path, nemesis });
            }
        }
    }
    found.into_values().filter(|entry| scope.allows(entry.nemesis)).collect()
}
