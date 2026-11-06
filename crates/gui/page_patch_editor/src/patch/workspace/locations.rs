//! Where things are: a mod's folders from what was selected, and template paths.

use std::path::{Path, PathBuf};

pub(in crate::patch) use stdx::path::joined;

use super::{
    mod_info::{pandora_info, parse_info},
    read_dir,
};

/// The directories of a mod's root that hold patches, in `<engine>/mod/<code>`.
pub(super) const ENGINES: [&str; 2] = ["Nemesis_Engine", "Nemesis_EngineExt"];

/// The directory of a mod's root with its Pandora patches, in `<here>/mod/<code>/patches`.
pub(super) const PANDORA_ENGINE: &str = "Pandora_Engine";

/// All that can hold a mod code.
const SOURCES: [&str; 3] = [ENGINES[0], ENGINES[1], PANDORA_ENGINE];

/// The name of the file or folder at `path`, borrowed if it is valid text.
pub(super) fn file_name_of(path: &Path) -> std::borrow::Cow<'_, str> {
    path.file_name().map_or(std::borrow::Cow::Borrowed(""), |name| name.to_string_lossy())
}

/// `<selected>/<engine>/mod`
fn mods_dir(selected: &Path, engine: &str) -> PathBuf {
    joined(selected, &[engine, "mod"])
}

/// The root of the Nemesis mod at `selected`: itself, or what it is the patch directory of.
/// `None`: it is no Nemesis mod.
///
/// e.g. `mods/Paraglider` containing `Nemesis_Engine/mod/para/` -> itself;
/// `mods/Paraglider/Nemesis_Engine/mod/para` -> `mods/Paraglider`.
pub(crate) fn mod_root_of(selected: &Path) -> Option<PathBuf> {
    let has_code =
        |engine: &str| read_dir(&mods_dir(selected, engine)).is_ok_and(|mut codes| codes.any(|path| path.is_dir()));
    if SOURCES.iter().any(|engine| has_code(engine)) {
        return Some(selected.to_path_buf());
    }
    let is_named = |path: &Path, name: &str| path.file_name().is_some_and(|found| found.eq_ignore_ascii_case(name));
    let mods = selected.parent().filter(|path| is_named(path, "mod"))?;
    let engine = mods.parent().filter(|path| SOURCES.iter().any(|name| is_named(path, name)))?;
    engine.parent().map(Path::to_path_buf)
}

/// The directories of `code` in the mod at `root`, of the engines it has.
pub(super) fn code_dirs(root: &Path, code: &str) -> Vec<PathBuf> {
    let dirs = ENGINES.iter().map(|engine| joined(root, &[engine, "mod", code]));
    dirs.filter(|dir| dir.is_dir()).collect()
}

/// A patch directory of `code` in the mod at `root`, if it has one.
pub(crate) fn patch_dir_of_code(root: &Path, code: &str) -> Option<PathBuf> {
    code_dirs(root, code).into_iter().next().or_else(|| pandora_dir(root, code))
}

/// The Pandora mod of `code` in the mod at `root`, if it has one.
pub(super) fn pandora_dir(root: &Path, code: &str) -> Option<PathBuf> {
    Some(joined(root, &[PANDORA_ENGINE, "mod", code])).filter(|dir| dir.is_dir())
}

/// A mod code in a mod's root, and the name its `info.ini` has for it.
///
/// e.g. `ecnpc`, `Elden Counter NPC`. The name is empty if it can't be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModCode {
    pub(crate) code: String,
    pub(crate) name: String,
}

/// All the mod codes of the mod whose root is `selected`, sorted, whichever engine has them: a code in
/// both of them is one work. Empty if `selected` is no mod's root, e.g. a patch directory.
///
/// e.g. `mods/Elden Counter` containing `Nemesis_Engine/mod/{ecdmg,ecnpc}` -> `ecdmg`, `ecnpc`
pub(crate) fn mod_codes_of_root(selected: &Path) -> Vec<ModCode> {
    let mut codes: Vec<ModCode> = Vec::new();
    for engine in SOURCES {
        let mods = mods_dir(selected, engine);
        let Ok(entries) = read_dir(&mods) else { continue };
        for dir in entries.filter(|path| path.is_dir()) {
            let code = file_name_of(&dir);
            if !codes.iter().any(|known| known.code.eq_ignore_ascii_case(&code)) {
                let name = std::fs::read_to_string(dir.join("info.ini")).map_or_else(
                    |_| pandora_info(&dir).map(|info| info.name).unwrap_or_default(),
                    |info| parse_info(&info).name,
                );
                codes.push(ModCode { code: code.into_owned(), name });
            }
        }
    }
    codes.sort_unstable_by(|a, b| stdx::text::cmp_ignore_case(&a.code, &b.code));
    codes
}

/// The code of the mod at `selected` (its root or a patch directory). See [`mod_dir_of`].
pub(crate) fn code_of(selected: &Path) -> String {
    file_name_of(&mod_dir_of(selected)).into_owned()
}

/// The patch directory of a mod, given its root (what a mod manager installs).
///
/// e.g., `mods/Paraglider` containing `Nemesis_Engine/mod/para/` -> `.../Nemesis_Engine/mod/para`.
/// A directory that is not a mod root is returned as is. If a mod has several codes, the first one
/// by name is taken: to choose among them is for the one asking, see [`mod_codes_of_root`].
///
/// The root is also where a later anim set generator finds `meshes/**/animations/*.hkx`.
pub(super) fn mod_dir_of(selected: &Path) -> std::borrow::Cow<'_, Path> {
    let code_dir = SOURCES.iter().find_map(|engine| {
        let entries = std::fs::read_dir(mods_dir(selected, engine)).ok()?;
        entries.flatten().map(|entry| entry.path()).filter(|path| path.is_dir()).min()
    });
    code_dir.map_or(std::borrow::Cow::Borrowed(selected), std::borrow::Cow::Owned)
}

/// The patch directories of the mod at `selected`: all its codes, if it is a mod's root, else itself.
///
/// e.g. `mods/a` containing `Nemesis_Engine/mod/amco` -> that; `.../mod/amco` -> the same.
pub(super) fn patch_dirs_of(selected: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for engine in ENGINES {
        if let Ok(codes) = read_dir(&mods_dir(selected, engine)) {
            dirs.extend(codes.filter(|path| path.is_dir()));
        }
    }
    if dirs.is_empty() { vec![selected.to_path_buf()] } else { dirs }
}

/// The template of animation data, `.bin` or `.txt`. (`.bin` if neither exists: clip data needs none.)
pub(super) fn anim_template_path(template_root: &Path, stem: &str) -> PathBuf {
    let mut path = joined(template_root, &["meshes", stem]);
    path.set_extension("bin");
    if !path.is_file() {
        path.set_extension("txt");
        if !path.is_file() {
            path.set_extension("bin");
        }
    }
    path
}

pub(super) fn meshes_path(template: &Path) -> String {
    let parts: Vec<String> = template.components().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).collect();
    let from = parts.iter().rposition(|p| p == "meshes").unwrap_or_else(|| parts.len().saturating_sub(1));
    let joined = parts[from..].join("/");
    Path::new(&joined).with_extension("bin").to_string_lossy().replace('\\', "/")
}

/// The Nemesis folder `(name, 1st person)` of a template, if Nemesis supports it.
pub(in crate::patch) fn nemesis_folder(meshes_path: &str) -> Option<(String, bool)> {
    use behavior_merge_types::template_key::{NEMESIS_1ST_PERSON_MAP, NEMESIS_3RD_PERSON_MAP};
    let find = |map: &phf::Map<&'static str, &'static str>| {
        map.entries().find(|(_, path)| path.eq_ignore_ascii_case(meshes_path)).map(|(name, _)| (*name).to_string())
    };
    find(&NEMESIS_3RD_PERSON_MAP)
        .map(|name| (name, false))
        .or_else(|| find(&NEMESIS_1ST_PERSON_MAP).map(|name| (name, true)))
}

/// A patch folder (relative to the mod dir, `/` separated) -> its template's meshes path.
///
/// `0_master` / `_1stperson/0_master` by Nemesis' table, `meshes/.../x.bin` as is.
pub(super) fn meshes_path_of_folder(relative: &str) -> Option<String> {
    use behavior_merge_types::template_key::TemplateKey;
    if relative.to_ascii_lowercase().starts_with("meshes/") {
        return Some(relative.to_lowercase());
    }
    let (name, first_person) = match relative.split_once('/') {
        Some((prefix, name)) if prefix.eq_ignore_ascii_case("_1stperson") => (name, true),
        _ => (relative, false),
    };
    TemplateKey::from_nemesis_file(name, first_person).map(|key| key.as_str().to_lowercase())
}

/// Folders (anywhere under `mod_dir`) that directly hold `#*.txt` patch files.
pub(super) fn patch_folders(mod_dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![mod_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = read_dir(&dir) else { continue };
        let mut has_patch = false;
        for path in entries {
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("txt"))
                && file_stem(&path).starts_with('#')
            {
                has_patch = true;
            }
        }
        if has_patch {
            found.push(dir);
        }
    }
    found.sort();
    found
}

pub(super) fn file_stem(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}
