//! Reading a mod's patches into a work: only their blocks go on the templates.

use std::{
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

use nemesis_diff::{View, apply_to_template, resolve};

use super::{
    ExportFormat, TemplateScope, Workspace,
    behavior::{ADSF, ASDSF, Behavior},
    export::owned_key,
    locations::{
        anim_template_path, code_dirs, code_of, file_name_of, file_stem, joined, meshes_path_of_folder, mod_root_of,
        pandora_dir, patch_folders,
    },
    mod_info::{pandora_info, parse_info},
    read_dir, read_text,
};

/// `<mod>/animationdatasinglefile/<project>/<file>.txt` -> the clip data "behavior" (or `None`).
/// Files are kept as they are; see [`Behavior::adsf`].
pub(crate) fn import_adsf(mod_dir: &Path, template_root: &Path) -> Result<Option<Behavior>, String> {
    let dir = mod_dir.join(ADSF);
    if !dir.is_dir() {
        return Ok(None);
    }
    let mut behavior = Behavior::open(&anim_template_path(template_root, ADSF))?;
    for project in read_dir(&dir)?.filter(|p| p.is_dir()) {
        let project_name = file_name_of(&project);
        for file in read_dir(&project)?.filter(|p| p.is_file()) {
            let file_name = file_name_of(&file);
            behavior.added.insert(format!("{project_name}/{file_name}"), read_text(&file)?);
        }
    }
    Ok(Some(behavior))
}

/// `<mod>/animationsetdatasinglefile/<project>/<file>.txt` -> an anim set "behavior" (or `None`).
///
/// An edit of a set of the template is read as the edited set. Every other file (`tkuc$New.txt`,
/// `$header$.txt`, a project the template does not have) is kept as it is: the export replaces the
/// files read, so what is not kept would be lost.
pub(super) fn import_asdsf(
    mod_dir: &Path,
    template_root: &Path,
    skipped: &mut Vec<String>,
) -> Result<Option<Behavior>, String> {
    let dir = mod_dir.join(ASDSF);
    if !dir.is_dir() {
        return Ok(None);
    }
    let mut template = joined(template_root, &["meshes", ASDSF]);
    if !["bin", "txt"].iter().any(|ext| {
        template.set_extension(ext);
        template.is_file()
    }) {
        skipped.push(ASDSF.to_string());
        return Ok(None);
    };
    let mut behavior = Behavior::open(&template)?;
    for project in read_dir(&dir)?.filter(|p| p.is_dir()) {
        let project_name = file_name_of(&project);
        for file in read_dir(&project)?.filter(|p| p.is_file()) {
            let file_name = file_name_of(&file);
            let wanted = format!("{project_name}/{file_name}");
            // Folder / file names are case-insensitive.
            let template_id =
                behavior.objects.iter().find(|o| o.id.eq_ignore_ascii_case(&wanted)).map(|o| o.id.clone());
            match template_id {
                Some(id) => {
                    let patch = read_text(&file)?;
                    let text =
                        apply_to_template(&patch, behavior.template_object(&id).map_or("", |object| &object.text));
                    behavior.set_text(&id, &text);
                }
                None => {
                    behavior.added.insert(wanted, read_text(&file)?);
                }
            }
        }
    }
    Ok(Some(behavior))
}

/// The id of the object a patch file patches: the `name` of its first `<hkobject>`, which is what
/// Nemesis and d-merge go by, not the file's name. `None`: the file has none (e.g. animation data).
///
/// e.g. BFCO's `1hm_behavior/#3018.txt` patches `#3108`: its name has the digits swapped, but the
/// patch is right, and must be read as `#3108`'s, or its blocks would go on another object.
fn patched_id(patch: &str) -> Option<&str> {
    let head = patch.lines().find(|line| line.trim_start().starts_with("<hkobject"))?;
    let (_, rest) = head.split_once("name=\"")?;
    rest.split('"').next().filter(|id| id.starts_with('#'))
}

impl Workspace {
    /// Imports a mod. `selected` is its root (see [`mod_dir_of`]) or
    /// `.../Nemesis_Engine/mod/<code>` or `.../Nemesis_EngineExt/mod/<code>`.
    ///
    /// Each patch folder is resolved to a template xml under `template_root`:
    /// - `<name>/`, `_1stperson/<name>/`: by Nemesis' template table
    /// - `meshes/.../<x>.bin/` (NemesisExt): by that path (`.bin` -> `.xml`)
    ///
    /// Returns `(workspace, code, folders without a template)`.
    ///
    /// # Errors
    /// `selected` is not a mod, or a patch could not be read.
    pub fn import(selected: &Path, template_root: &Path) -> Result<(Self, String, Vec<String>), String> {
        Self::import_counted(selected, template_root, &AtomicUsize::new(0))
    }

    /// [`Self::import`], counting the patch files read in `done`: a big mod has thousands, and the
    /// editor shows how far it is (see `tool::job`).
    ///
    /// # Errors
    /// See [`Self::import`].
    pub fn import_counted(
        selected: &Path,
        template_root: &Path,
        done: &AtomicUsize,
    ) -> Result<(Self, String, Vec<String>), String> {
        let code = code_of(selected);
        let root = mod_root_of(selected);
        // A mod has its patches for templates Nemesis does not know in the other engine's directory.
        // Reading one of them would lose the other's at the next export to the same place.
        let dirs = root.as_ref().map_or_else(|| vec![selected.to_path_buf()], |root| code_dirs(root, &code));
        let pandora = root.as_ref().and_then(|root| pandora_dir(root, &code));
        let info = dirs.iter().find_map(|dir| std::fs::read_to_string(dir.join("info.ini")).ok());
        let info = info
            .map(|info| parse_info(&info))
            .or_else(|| pandora.as_deref().and_then(pandora_info))
            .unwrap_or_default();
        let mut workspace = Self { info, ..Self::default() };
        let mut skipped = Vec::new();
        for dir in &dirs {
            workspace.read_patch_dir(dir, root.as_deref(), template_root, &mut skipped, done)?;
        }
        if let Some(pandora) = &pandora {
            workspace.read_pandora_dir(pandora, template_root, &mut skipped)?;
        }
        // A mod of Nemesis only: its work shows only the templates Nemesis knows.
        workspace.scope = TemplateScope::of(&workspace);
        Ok((workspace, code, skipped))
    }

    /// Reads a `Nemesis_Engine/mod/<code>`-like directory into the work. `root`: the mod's root, what
    /// [`Self::owned`] is relative to.
    fn read_patch_dir(
        &mut self,
        mod_dir: &Path,
        root: Option<&Path>,
        template_root: &Path,
        skipped: &mut Vec<String>,
        done: &AtomicUsize,
    ) -> Result<(), String> {
        for folder in patch_folders(mod_dir) {
            let relative = folder.strip_prefix(mod_dir).unwrap_or(&folder);
            let relative = relative.to_string_lossy().replace('\\', "/");
            let Some(meshes_path) = meshes_path_of_folder(&relative) else {
                skipped.push(relative);
                continue;
            };
            // `.bin` (shipped templates) or `.xml` (a template_dir of xml files).
            let mut template = template_root.join(&meshes_path);
            if !["bin", "xml"].iter().any(|ext| {
                template.set_extension(ext);
                template.is_file()
            }) {
                skipped.push(relative);
                continue;
            };
            let at = self.open_behavior(&template, &meshes_path)?;
            let behavior = &mut self.behaviors[at];
            // A template Nemesis knows, patched in `Nemesis_EngineExt`: the mod is written as NemesisExt.
            if relative.to_ascii_lowercase().starts_with("meshes/") && behavior.nemesis.is_some() {
                self.export_format = ExportFormat::NemesisExt;
            }
            for file in read_dir(&folder)?.filter(|p| p.extension().is_some_and(|e| e == "txt")) {
                done.fetch_add(1, Ordering::Relaxed);
                let patch = read_text(&file)?;
                let id = patched_id(&patch).map_or_else(|| file_stem(&file), str::to_owned);
                if let Some(template) = behavior.template_object(&id) {
                    let text = apply_to_template(&patch, &template.text);
                    behavior.set_text(&id, &text);
                } else {
                    behavior.added.insert(id, resolve(&patch, View::Mod).into_owned());
                }
                self.owned.extend(root.and_then(|root| file.strip_prefix(root).ok()).map(owned_key));
            }
        }
        if !self.behaviors.iter().any(|behavior| behavior.asdsf)
            && let Some(asdsf) = import_asdsf(mod_dir, template_root, skipped)?
        {
            self.behaviors.push(asdsf);
            self.own_anim_files(&mod_dir.join(ASDSF), root);
        }
        if !self.behaviors.iter().any(|behavior| behavior.adsf)
            && let Some(adsf) = import_adsf(mod_dir, template_root)?
        {
            self.behaviors.push(adsf);
            self.own_anim_files(&mod_dir.join(ADSF), root);
        }
        Ok(())
    }

    /// The files read from an animation data directory: `<dir>/<project>/<file>`.
    fn own_anim_files(&mut self, dir: &Path, root: Option<&Path>) {
        let Some(root) = root else { return };
        let Ok(projects) = read_dir(dir) else { return };
        for project in projects.filter(|path| path.is_dir()) {
            let Ok(files) = read_dir(&project) else { continue };
            let files = files.filter(|path| path.is_file());
            self.owned.extend(files.filter_map(|file| file.strip_prefix(root).ok().map(owned_key)));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    /// The shipped templates of the repository.
    fn template_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resource/assets/templates")
    }

    /// On all the mods in a folder (`PATCH_EDITOR_MODS`, e.g. MO2's `mods`): a behavior patch that
    /// d-merge's parser accepts as the mod has it is still accepted once read and written again.
    ///
    /// e.g., `Combat Gameplay Overhaul` deletes elements of `#0521` with blank lines. They were read as
    /// lines, and the written patch had a block adding blank lines, which the parser rejects.
    ///
    /// `PATCH_EDITOR_ONLY`: only the mods whose folder name contains it, with the patches that break
    /// printed whole.
    /// A file is read as the patch of the object it names, not of its file name.
    #[test]
    fn a_patch_is_of_the_object_it_names() {
        let patch = "		<hkobject name=\"#3108\" class=\"hkbClipTriggerArray\" signature=\"0x59c23a0f\">
";
        assert_eq!(patched_id(patch), Some("#3108"));
        assert_eq!(
            patched_id(
                "V3
1
"
            ),
            None,
            "animation data"
        );
    }

    #[test]
    #[ignore = "needs real mods"]
    fn a_patch_read_and_written_again_still_parses() {
        let mods = std::env::var("PATCH_EDITOR_MODS").unwrap();
        let only = std::env::var("PATCH_EDITOR_ONLY").ok();
        let (mut total, mut broken) = (0, 0);
        let mod_dirs = std::fs::read_dir(&mods).unwrap().flatten().map(|entry| entry.path());
        let mod_dirs = mod_dirs.filter(|dir| only.as_deref().is_none_or(|only| dir.to_string_lossy().contains(only)));
        for mod_dir in mod_dirs {
            let Ok(codes) = std::fs::read_dir(mod_dir.join("Nemesis_Engine").join("mod")) else { continue };
            for code_dir in codes.flatten().map(|entry| entry.path()).filter(|dir| dir.is_dir()) {
                let Ok((workspace, code, _)) = Workspace::import(&code_dir, &template_root()) else { continue };
                for behavior in workspace.behaviors.iter().filter(|behavior| !behavior.is_anim_data()) {
                    for id in behavior.edits.keys() {
                        // By the object it patches: e.g. BFCO's `#3018.txt` is `#3108`'s.
                        let files =
                            behavior.patch_folders_in(&code_dir).into_iter().filter_map(|dir| read_dir(&dir).ok());
                        let mut texts = files.flatten().filter_map(|path| read_text(&path).ok());
                        let Some(original) = texts.find(|text| patched_id(text) == Some(id.as_str())) else { continue };
                        if nemesis_diff::validate(&original).is_err() {
                            continue; // d-merge rejects it as well: not ours to fix.
                        }
                        total += 1;
                        let written = behavior.patch(&code, id).unwrap_or_default();
                        if let Err(err) = nemesis_diff::validate(&written) {
                            broken += 1;
                            let first = err.lines().next().unwrap_or_default();
                            println!("{} {}/{id}: {first}", code_dir.display(), behavior.name);
                            if only.is_some() {
                                println!("--- original\n{original}\n--- written\n{written}\n--- error\n{err}");
                            }
                        }
                    }
                }
            }
        }
        println!("{broken} of {total} patches broken by writing them again");
        assert_eq!(broken, 0);
    }
}
