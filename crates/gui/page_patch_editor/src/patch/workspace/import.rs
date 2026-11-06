//! Reading a mod's patches into a work: only their blocks go on the templates.

use std::path::Path;

use nemesis_diff::{View, apply_to_template, resolve};

use super::{
    Workspace,
    behavior::{ADSF, ASDSF, Behavior},
    export::owned_key,
    locations::{anim_template_path, code_dirs, code_of, file_stem, meshes_path_of_folder, mod_root_of, patch_folders},
    mod_info::parse_info,
    read_dir, read_text,
};

/// `<mod>/animationdatasinglefile/<project>/<file>.txt` -> the clip data "behavior" (or `None`).
/// Files are kept as they are; see [`Behavior::adsf`].
pub(super) fn import_adsf(mod_dir: &Path, template_root: &Path) -> Result<Option<Behavior>, String> {
    let dir = mod_dir.join(ADSF);
    if !dir.is_dir() {
        return Ok(None);
    }
    let mut behavior = Behavior::open(&anim_template_path(template_root, ADSF))?;
    for project in read_dir(&dir)?.filter(|p| p.is_dir()) {
        let project_name = project.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        for file in read_dir(&project)?.filter(|p| p.is_file()) {
            let file_name = file.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
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
    let Some(template) = ["bin", "txt"]
        .iter()
        .map(|ext| template_root.join("meshes").join(format!("{ASDSF}.{ext}")))
        .find(|path| path.is_file())
    else {
        skipped.push(ASDSF.to_string());
        return Ok(None);
    };
    let mut behavior = Behavior::open(&template)?;
    for project in read_dir(&dir)?.filter(|p| p.is_dir()) {
        let project_name = project.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        for file in read_dir(&project)?.filter(|p| p.is_file()) {
            let file_name = file.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let wanted = format!("{project_name}/{file_name}");
            // Folder / file names are case-insensitive.
            let template_id =
                behavior.objects.iter().find(|o| o.id.eq_ignore_ascii_case(&wanted)).map(|o| o.id.clone());
            match template_id {
                Some(id) => {
                    let template = behavior.template_object(&id).map(|object| object.text.clone()).unwrap_or_default();
                    behavior.set_text(&id, apply_to_template(&read_text(&file)?, &template));
                }
                None => {
                    behavior.added.insert(wanted, read_text(&file)?);
                }
            }
        }
    }
    Ok(Some(behavior))
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
    pub(crate) fn import(selected: &Path, template_root: &Path) -> Result<(Self, String, Vec<String>), String> {
        let code = code_of(selected);
        let root = mod_root_of(selected);
        // A mod has its patches for templates Nemesis does not know in the other engine's directory.
        // Reading one of them would lose the other's at the next export to the same place.
        let dirs = root.as_ref().map_or_else(|| vec![selected.to_path_buf()], |root| code_dirs(root, &code));
        let info = dirs.iter().find_map(|dir| std::fs::read_to_string(dir.join("info.ini")).ok());
        let mut workspace = Self { info: info.map(|info| parse_info(&info)).unwrap_or_default(), ..Self::default() };
        let mut skipped = Vec::new();
        for dir in &dirs {
            workspace.read_patch_dir(dir, root.as_deref(), template_root, &mut skipped)?;
        }
        Ok((workspace, code, skipped))
    }

    /// Reads a `Nemesis_Engine/mod/<code>`-like directory into the work. `root`: the mod's root, what
    /// [`Self::owned`] is relative to.
    pub(super) fn read_patch_dir(
        &mut self,
        mod_dir: &Path,
        root: Option<&Path>,
        template_root: &Path,
        skipped: &mut Vec<String>,
    ) -> Result<(), String> {
        for folder in patch_folders(mod_dir) {
            let relative = folder.strip_prefix(mod_dir).unwrap_or(&folder);
            let relative = relative.to_string_lossy().replace('\\', "/");
            let Some(meshes_path) = meshes_path_of_folder(&relative) else {
                skipped.push(relative);
                continue;
            };
            // `.bin` (shipped templates) or `.xml` (a template_dir of xml files).
            let Some(template) = ["bin", "xml"]
                .iter()
                .map(|ext| template_root.join(Path::new(&meshes_path).with_extension(ext)))
                .find(|path| path.is_file())
            else {
                skipped.push(relative);
                continue;
            };
            let at = self.open_behavior(&template, &meshes_path)?;
            let behavior = &mut self.behaviors[at];
            behavior.force_ext |= relative.to_ascii_lowercase().starts_with("meshes/") && behavior.nemesis.is_some();
            for file in read_dir(&folder)?.filter(|p| p.extension().is_some_and(|e| e == "txt")) {
                let id = file_stem(&file);
                let patch = read_text(&file)?;
                if let Some(template) = behavior.template_object(&id).map(|object| object.text.clone()) {
                    behavior.set_text(&id, apply_to_template(&patch, &template));
                } else {
                    behavior.added.insert(id, resolve(&patch, View::Mod));
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
    pub(super) fn own_anim_files(&mut self, dir: &Path, root: Option<&Path>) {
        let Some(root) = root else { return };
        let Ok(projects) = read_dir(dir) else { return };
        for project in projects.filter(|path| path.is_dir()) {
            let Ok(files) = read_dir(&project) else { continue };
            let files = files.filter(|path| path.is_file());
            self.owned.extend(files.filter_map(|file| file.strip_prefix(root).ok().map(owned_key)));
        }
    }
}
