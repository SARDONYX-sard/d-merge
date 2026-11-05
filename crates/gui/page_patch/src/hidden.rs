//! Where the hidden operations of the mods are kept: with the user patch, for each profile.
//! What they are is in [`HiddenPatches`].
//!
//! ```json
//! [ { "mod_code": "para", "target": "meshes/actors/character/behaviors/0_master.bin",
//!     "path": "#0106/hkbModifierList/enable" } ]
//! ```

use std::path::Path;

use behavior_merge::{HiddenPatch, HiddenPatches};

const HIDDEN_FILE: &str = "./.d_merge/user_patch/hidden_patches.json";

/// What is hidden now. A file that cannot be read is logged, and nothing is hidden.
pub(crate) fn load() -> HiddenPatches {
    load_from(Path::new(HIDDEN_FILE))
}

/// # Errors
/// The file could not be written.
pub(crate) fn save(hidden: &HiddenPatches) -> Result<(), String> {
    save_to(Path::new(HIDDEN_FILE), hidden)
}

fn load_from(path: &Path) -> HiddenPatches {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                tracing::error!("{}: {err}", path.display());
            }
            return HiddenPatches::default();
        }
    };
    match sonic_rs::from_str::<Vec<HiddenPatch>>(&text) {
        Ok(list) => HiddenPatches::new(list),
        Err(err) => {
            tracing::error!("{}: {err}", path.display());
            HiddenPatches::default()
        }
    }
}

fn save_to(path: &Path, hidden: &HiddenPatches) -> Result<(), String> {
    let list: Vec<&HiddenPatch> = hidden.iter().collect();
    let text = sonic_rs::to_string_pretty(&list).map_err(|err| err.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| format!("{}: {err}", parent.display()))?;
    }
    std::fs::write(path, text).map_err(|err| format!("{}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_hidden_is_kept() {
        let dir = std::env::temp_dir().join(format!("d_merge_hidden_{}", std::process::id()));
        let path = dir.join("sub").join("hidden.json");
        assert!(load_from(&path).is_empty(), "no file: nothing is hidden");

        let mut hidden = HiddenPatches::default();
        hidden.toggle("para", "meshes/a.bin", "#1/C/a");
        save_to(&path, &hidden).unwrap();
        let loaded = load_from(&path);

        std::fs::write(&path, "not json").unwrap();
        let broken = load_from(&path);
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(loaded, hidden);
        assert!(broken.is_empty());
    }
}
