//! A patch being made: the edited objects of each behavior, kept by object id.
//!
//! # Why by id (not text positions)
//! Nemesis matches patches by object id, and ids stay valid when the template text changes
//! (positions would not). Only the changed objects are stored, so a 1.5 MB template costs nothing.
//!
//! # On disk (`<patch page's output>/.d_merge/caches/patch_editor/nemesis/<code>/`, see [`cache_root_of`])
//! ```text
//! info.ini                      name / author / site
//! meshes~actors~draugr~behaviors~draugrbehavior.bin/
//!   template.path               the template xml this behavior was made from
//!   #0106.xml                   an edited template object (full object text)
//!   #mymod$0.xml                a new object
//! ```
//! [`Workspace::export`] writes the real mod: `<output>/Nemesis_Engine/mod/<code>/` for templates
//! Nemesis supports, `<output>/Nemesis_EngineExt/mod/<code>/meshes/...` for the others, or for all of
//! them as [`ExportFormat::NemesisExt`].

mod behavior;
mod catalog;
mod export;
pub(crate) mod history;
mod import;
mod locations;
mod mod_info;
pub(in crate::patch) mod pandora;
pub(in crate::patch) mod rename;

use std::{
    borrow::Cow,
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use rayon::prelude::*;

pub(in crate::patch) use self::locations::{joined, nemesis_folder};
use self::{
    behavior::cache_file,
    locations::{anim_template_path, file_stem, patch_dirs_of},
    mod_info::{info_ini, parse_info},
};
pub(crate) use self::{
    behavior::{ADSF, ASDSF, Behavior, ObjectText, PANDORA_DOC, STRING_DATA, Target},
    catalog::{TemplateEntry, catalog},
    locations::{ModCode, code_of, mod_codes_of_root, mod_root_of, patch_dir_of_code},
    mod_info::ModInfo,
};

/// Where the works are kept: in the patch page's output directory, so for each profile, not in the mods.
///
/// e.g. `D:/out` -> `D:/out/.d_merge/caches/patch_editor/nemesis`
pub(crate) fn cache_root_of(output_dir: &str) -> PathBuf {
    joined(Path::new(output_dir), &[".d_merge", "caches", "patch_editor", "nemesis"])
}

/// See [`WorkspaceFile`].
const WORKSPACE_FILE: &str = "workspace.json";
/// See [`history`].
const HISTORY_FILE: &str = "history.json";
/// See [`Workspace::resolutions`].
const RESOLUTIONS_FILE: &str = "resolutions.json";

/// What a cache dir holds, as of the last save or load. See [`Workspace::save`].
#[derive(Debug, Clone, Default)]
pub(crate) struct SavedFiles {
    dir: PathBuf,
    /// File (relative to `dir`) -> a hash of its contents.
    hashes: BTreeMap<PathBuf, u64>,
}

/// A hash of what a file holds, to tell whether it changed.
fn text_hash(text: &str) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    let mut hasher = std::hash::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Removes a file of the cache dir, and the directory of a behavior this leaves empty.
fn remove_saved(dir: &Path, path: &Path) -> Result<(), String> {
    let file = dir.join(path);
    match std::fs::remove_file(&file) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(format!("{}: {err}", file.display())),
    }
    if let Some(parent) = file.parent().filter(|parent| *parent != dir) {
        let _not_empty = std::fs::remove_dir(parent);
    }
    Ok(())
}

/// Removes the `<behavior>/force_ext` files of a work kept when NemesisExt was chosen per behavior.
/// Returns whether there was one: the work is then written as [`ExportFormat::NemesisExt`].
fn take_old_force_ext(dir: &Path, entries: &[PathBuf]) -> bool {
    let mut found = false;
    for entry in entries {
        let file = entry.join("force_ext");
        if file.is_file() {
            found = true;
            let _gone_anyway = remove_saved(dir, file.strip_prefix(dir).unwrap_or(&file));
        }
    }
    found
}

/// A behavior of a cache dir: `<dir>/<behavior>/template.path` and its objects. `None`: no behavior.
/// `done`: counts the objects read.
fn read_cached_behavior(entry: &Path, done: &AtomicUsize) -> Result<Option<Behavior>, String> {
    let Ok(template) = std::fs::read_to_string(entry.join("template.path")) else {
        return Ok(None);
    };
    let mut behavior = Behavior::open(Path::new(template.trim()))?;
    behavior.pandora_file = std::fs::read_to_string(entry.join("pandora_file")).ok();
    for file in read_dir(entry)?.filter(|p| p.extension().is_some_and(|e| e == "xml")) {
        done.fetch_add(1, Ordering::Relaxed);
        let id = file_stem(&file).replace("%2F", "/");
        let text = read_text(&file)?;
        if id == PANDORA_DOC {
            behavior.pandora_doc_ids = pandora_patch::doc::doc_object_ids(&text);
            behavior.pandora_doc = Some(text);
        } else if behavior.template_object(&id).is_some() {
            behavior.edits.insert(id, text);
        } else {
            behavior.added.insert(id, text);
        }
    }
    // The file of a Pandora mod says what its edits are, also if the work was kept without them.
    if behavior.pandora_file.is_some() {
        behavior.apply_pandora_doc();
    }
    Ok(Some(behavior))
}

/// What a work is written as. It is the same work: edits of the objects of templates.
///
/// Animation data is written to `Nemesis_Engine` in both Nemesis formats.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum ExportFormat {
    /// `Nemesis_Engine/mod/<code>`, and `Nemesis_EngineExt/mod/<code>` for what Nemesis doesn't know.
    #[default]
    Nemesis,
    /// `Nemesis_EngineExt/mod/<code>/meshes/...` for every template: only d-merge reads it.
    NemesisExt,
    /// `Pandora_Engine/mod/<code>` of Pandora Behaviour Engine v5. A later version that changes the
    /// format would be another variant.
    #[serde(alias = "Pandora")]
    PandoraV5,
}

impl ExportFormat {
    /// In the order of the export's list: `(format, label)`.
    pub(crate) const ALL: [(Self, &'static str); 3] =
        [(Self::Nemesis, "Nemesis"), (Self::NemesisExt, "Nemesis Ext"), (Self::PandoraV5, "Pandora v5")];

    /// Are the templates Nemesis knows written to `Nemesis_EngineExt` too? See [`Behavior::target`].
    #[inline]
    pub(crate) const fn is_ext(self) -> bool {
        matches!(self, Self::NemesisExt)
    }
}

/// Which templates a work can patch: chosen when it is made, and fixed. The export can still write it
/// in any [`ExportFormat`].
///
/// | scope    | templates                         | made for                    |
/// | -------- | --------------------------------- | --------------------------- |
/// | Nemesis  | those Nemesis knows (`nemesis`)   | Nemesis, and the others     |
/// | Extended | all, and template files of yours  | Nemesis Ext, Pandora v5     |
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum TemplateScope {
    #[default]
    Nemesis,
    Extended,
}

impl TemplateScope {
    /// In the order of the new work's list: `(scope, label)`.
    pub(crate) const ALL: [(Self, &'static str); 2] =
        [(Self::Nemesis, "Nemesis"), (Self::Extended, "Nemesis Ext / Pandora v5")];

    /// Can a work of this scope patch a template? `nemesis`: Nemesis knows it ([`TemplateEntry::nemesis`]).
    #[inline]
    pub(crate) const fn allows(self, nemesis: bool) -> bool {
        nemesis || matches!(self, Self::Extended)
    }

    /// The scope of a work read from a mod, or kept before there was one: `Extended` if it has what
    /// Nemesis can't read (a template it doesn't know, or patches in `Nemesis_EngineExt` / Pandora's).
    pub(crate) fn of(workspace: &Workspace) -> Self {
        let unknown = workspace.behaviors.iter().any(|behavior| !behavior.is_anim_data() && behavior.nemesis.is_none());
        if unknown || workspace.export_format != ExportFormat::Nemesis { Self::Extended } else { Self::Nemesis }
    }

    /// What a new work of this scope is exported as, until changed.
    pub(crate) const fn default_format(self) -> ExportFormat {
        match self {
            Self::Nemesis => ExportFormat::Nemesis,
            Self::Extended => ExportFormat::NemesisExt,
        }
    }
}

/// The whole patch being made.

#[derive(Debug, Default)]
pub(crate) struct Workspace {
    /// See [`Workspace::save`].
    saved: Option<SavedFiles>,
    pub(crate) info: ModInfo,
    pub(crate) behaviors: Vec<Behavior>,
    /// The mods this one depends on, by their patch directories, e.g.
    /// `D:/mods/a/Nemesis_Engine/mod/amco`.
    ///
    /// An add-on, or a user patch resolving a conflict, points to their objects (`#amco$17`) and uses
    /// their events. Those are read as they are, can be looked at and pointed to, and are not edited
    /// or exported: they come with that mod when merging.
    pub(crate) ref_mods: Vec<String>,
    /// The revisions of the work. See [`history`].
    pub(crate) history: history::History,
    /// How conflicts were resolved, to do it again. See [`crate::patch::editor::merge`].
    pub(crate) resolutions: Vec<crate::patch::editor::merge::Resolution>,
    /// What `Export` writes.
    pub(crate) export_format: ExportFormat,
    /// Which templates can be added. See [`TemplateScope`].
    pub(crate) scope: TemplateScope,
    /// What was open when it was saved. See [`Session`].
    pub(crate) session: Session,
    /// The files of the mod this work read or wrote, relative to the mod's root: what the next export
    /// replaces. e.g. `Nemesis_Engine/mod/para/0_master/#0106.txt`
    pub(crate) owned: std::collections::BTreeSet<String>,
}

/// What was open in a work, to go back to it when it is opened again.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct Session {
    /// The root of the mod the work was read from. Empty: it was made here.
    pub(crate) mod_dir: String,
    /// Where it is exported to: the mod's root, unless changed.
    pub(crate) output: String,
    /// The behavior shown, by name.
    pub(crate) active: String,
    /// The object shown.
    pub(crate) selected: Option<String>,
    pub(crate) tabs: Vec<SessionTab>,
}

/// An open tab of a [`Session`].
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct SessionTab {
    pub(crate) behavior: String,
    pub(crate) id: String,
    pub(crate) pinned: bool,
}

/// `workspace.json` in a work's cache: what is not a patch. The patches are files of their own.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct WorkspaceFile {
    /// `None`: a work kept before there was a choice.
    export_format: Option<ExportFormat>,
    /// `None`: a work kept before there was a choice, see [`TemplateScope::of`].
    template_scope: Option<TemplateScope>,
    ref_mods: Vec<String>,
    session: Session,
    owned: std::collections::BTreeSet<String>,
}

impl Workspace {
    /// The user patch is read by d-merge only (it is the conflict tool's): it is NemesisExt, which takes
    /// any template, whatever it was kept as.
    pub(crate) const fn use_user_patch_format(&mut self) {
        self.export_format = ExportFormat::NemesisExt;
        self.scope = TemplateScope::Extended;
    }

    /// Adds the mod at `selected` (its root or a patch directory) to [`Self::ref_mods`], except for
    /// the code `own_code`, which is this mod.
    pub(crate) fn add_ref(&mut self, selected: &Path, own_code: &str) {
        for dir in patch_dirs_of(selected) {
            let is_own = dir.file_name().is_some_and(|name| name.eq_ignore_ascii_case(own_code));
            let dir = dir.display().to_string();
            let is_known = self.ref_mods.iter().any(|known| known.eq_ignore_ascii_case(&dir));
            if !is_own && !is_known {
                self.ref_mods.push(dir);
            }
        }
        self.reload_refs();
    }

    /// Reads the referenced mods into every behavior. Call when they or the behaviors change.
    pub(crate) fn reload_refs(&mut self) {
        for behavior in &mut self.behaviors {
            behavior.refs.clear();
            behavior.ref_events.clear();
            behavior.ref_variables.clear();
            for dir in &self.ref_mods {
                behavior.read_ref(Path::new(dir));
            }
        }
    }

    /// `<root>/<code>`. `root`: see [`cache_root_of`].
    pub(crate) fn cache_dir(root: &Path, code: &str) -> PathBuf {
        root.join(code)
    }

    /// Is there a work with this code?
    pub(crate) fn exists(root: &Path, code: &str) -> bool {
        Self::cache_dir(root, code).is_dir()
    }

    /// Where the work with this code came from and goes to, if there is one.
    pub(crate) fn cached_session(root: &Path, code: &str) -> Option<Session> {
        let dir = Self::cache_dir(root, code);
        if !dir.is_dir() {
            return None;
        }
        let file = std::fs::read_to_string(dir.join(WORKSPACE_FILE)).unwrap_or_default();
        Some(sonic_rs::from_str::<WorkspaceFile>(&file).unwrap_or_default().session)
    }

    /// Loads a cached workspace (an empty one if there is none).
    #[cfg(test)]
    pub(crate) fn load(root: &Path, code: &str) -> Result<Self, String> {
        Self::load_counted(root, code, &AtomicUsize::new(0))
    }

    /// [`Self::load`], counting the objects read in `done`. See [`Self::import_counted`].
    pub(crate) fn load_counted(root: &Path, code: &str, done: &AtomicUsize) -> Result<Self, String> {
        let dir = Self::cache_dir(root, code);
        let mut workspace = Self::default();
        if !dir.is_dir() {
            return Ok(workspace);
        }
        workspace.info = std::fs::read_to_string(dir.join("info.ini")).map(|s| parse_info(&s)).unwrap_or_default();

        // Each one reads and indexes a whole template: at the same time, in the order of the directory.
        let entries: Vec<PathBuf> = read_dir(&dir)?.filter(|p| p.is_dir()).collect();
        let behaviors: Vec<Option<Behavior>> =
            entries.par_iter().map(|entry| read_cached_behavior(entry, done)).collect::<Result<_, String>>()?;
        workspace.behaviors = behaviors.into_iter().flatten().collect();
        // A work without the file, from before it existed, has nothing to restore.
        let file = std::fs::read_to_string(dir.join(WORKSPACE_FILE)).unwrap_or_default();
        let file: WorkspaceFile = sonic_rs::from_str(&file).unwrap_or_default();
        // Until then, what was read from Pandora was to go back to it.
        let from_pandora = workspace.behaviors.iter().any(|behavior| behavior.pandora_file.is_some());
        let format_of_old_work = if from_pandora { ExportFormat::PandoraV5 } else { ExportFormat::Nemesis };
        workspace.export_format = file.export_format.unwrap_or(format_of_old_work);
        if take_old_force_ext(&dir, &entries) && workspace.export_format == ExportFormat::Nemesis {
            workspace.export_format = ExportFormat::NemesisExt;
        }
        workspace.scope = file.template_scope.unwrap_or_else(|| TemplateScope::of(&workspace));
        workspace.ref_mods = file.ref_mods;
        workspace.session = file.session;
        workspace.owned = file.owned;
        // A work kept before there was a history has none.
        let history = std::fs::read_to_string(dir.join(HISTORY_FILE)).unwrap_or_default();
        workspace.history = sonic_rs::from_str(&history).unwrap_or_default();
        let resolutions = std::fs::read_to_string(dir.join(RESOLUTIONS_FILE)).unwrap_or_default();
        workspace.resolutions = sonic_rs::from_str(&resolutions).unwrap_or_default();
        workspace.reload_refs();
        let hashes = workspace.files_to_save()?.iter().map(|(path, text)| (path.clone(), text_hash(text))).collect();
        workspace.saved = Some(SavedFiles { dir, hashes });
        Ok(workspace)
    }

    /// What the cache dir holds: file (relative to it) -> contents. See [`Self::save`].
    fn files_to_save(&self) -> Result<BTreeMap<PathBuf, Cow<'_, str>>, String> {
        let mut files = BTreeMap::new();
        files.insert(PathBuf::from("info.ini"), Cow::Owned(info_ini(&self.info)));
        let file = WorkspaceFile {
            export_format: Some(self.export_format),
            template_scope: Some(self.scope),
            ref_mods: self.ref_mods.clone(),
            session: self.session.clone(),
            owned: self.owned.clone(),
        };
        let file = sonic_rs::to_string_pretty(&file).map_err(|e| e.to_string())?;
        files.insert(PathBuf::from(WORKSPACE_FILE), Cow::Owned(file));
        if !self.history.revisions.is_empty() {
            let history = sonic_rs::to_string(&self.history).map_err(|e| e.to_string())?;
            files.insert(PathBuf::from(HISTORY_FILE), Cow::Owned(history));
        }
        if !self.resolutions.is_empty() {
            let resolutions = sonic_rs::to_string(&self.resolutions).map_err(|e| e.to_string())?;
            files.insert(PathBuf::from(RESOLUTIONS_FILE), Cow::Owned(resolutions));
        }
        for behavior in &self.behaviors {
            let behavior_dir = PathBuf::from(behavior.cache_name());
            let template = behavior.template_path.display().to_string();
            files.insert(behavior_dir.join("template.path"), Cow::Owned(template));
            for (id, text) in behavior.edits.iter().chain(&behavior.added) {
                files.insert(behavior_dir.join(cache_file(id)), Cow::Borrowed(text.as_str()));
            }
            if let Some(name) = &behavior.pandora_file {
                files.insert(behavior_dir.join("pandora_file"), Cow::Borrowed(name.as_str()));
            }
            if let Some(doc) = &behavior.pandora_doc {
                files.insert(behavior_dir.join(cache_file(PANDORA_DOC)), Cow::Borrowed(doc.as_str()));
            }
        }
        Ok(files)
    }

    /// Writes the workspace to its cache dir.
    ///
    /// This runs on the UI thread a moment after each edit, and after opening a file, as what is open
    /// is part of the work. Writing all the files takes over a second with a few hundred of them
    /// (e.g., BFCO), which froze the app each time. So only what changed since the last save is
    /// written, and what is gone removed. [`Self::saved`] knows that without reading the disk. If it
    /// does not know (the first save, another place), the directory is written anew.
    pub(crate) fn save(&mut self, root: &Path, code: &str) -> Result<(), String> {
        let dir = Self::cache_dir(root, code);
        // Not known until this has succeeded: a failure leaves the disk in between.
        let known = self.saved.take().filter(|saved| saved.dir == dir && dir.is_dir());
        let files = self.files_to_save()?;
        let hashes: BTreeMap<PathBuf, u64> = files.iter().map(|(path, text)| (path.clone(), text_hash(text))).collect();

        if known.is_none() && dir.is_dir() {
            std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let was = known.map(|saved| saved.hashes).unwrap_or_default();
        for (path, text) in &files {
            if was.get(path) != hashes.get(path) {
                write(&dir.join(path), text)?;
            }
        }
        for path in was.keys().filter(|path| !hashes.contains_key(*path)) {
            remove_saved(&dir, path)?;
        }
        drop(files);
        self.saved = Some(SavedFiles { dir, hashes });
        Ok(())
    }

    /// The anim set files per project of the template: the keys of its json, `txt_projects`.
    /// e.g. (`DefaultMaleData~DefaultMale`, [`_MTSolo.txt`, `1HMSolo.txt`, ...]), and a creature's
    /// (`BearProjectData~BearProject`, [`FullBody.txt`]).
    ///
    /// Does not add the anim set behavior to the work: that happens when one is changed.
    pub(crate) fn anim_set_projects(&self, template_root: &Path) -> Vec<(String, Vec<String>)> {
        // The template lists the sets of a project together, in the file's order.
        let group = |behavior: &Behavior| -> Vec<(String, Vec<String>)> {
            let projects = behavior.objects.chunk_by(|a, b| a.class == b.class);
            projects.map(|sets| (sets[0].class.clone(), sets.iter().map(|set| set.name.clone()).collect())).collect()
        };
        match self.behaviors.iter().find(|behavior| behavior.asdsf) {
            Some(asdsf) => group(asdsf),
            None => Behavior::open(&anim_template_path(template_root, ASDSF))
                .map_or_else(|_| Vec::new(), |asdsf| group(&asdsf)),
        }
    }

    /// The position of the behavior of `template`, opened if not yet there. `meshes_path`: its id.
    pub(crate) fn open_behavior(&mut self, template: &Path, meshes_path: &str) -> Result<usize, String> {
        if let Some(at) =
            self.behaviors.iter().position(|behavior| behavior.meshes_path.eq_ignore_ascii_case(meshes_path))
        {
            return Ok(at);
        }
        self.behaviors.push(Behavior::open(template)?);
        Ok(self.behaviors.len() - 1)
    }

    /// The animation data behavior with this file stem ([`ADSF`], [`ASDSF`]), opened if not yet there.
    pub(crate) fn anim_behavior(&mut self, stem: &str, template_root: &Path) -> Result<&mut Behavior, String> {
        let is_it = |behavior: &Behavior| if stem == ADSF { behavior.adsf } else { behavior.asdsf };
        let existing = self.behaviors.iter().position(is_it);
        if existing.is_none() {
            self.behaviors.push(Behavior::open(&anim_template_path(template_root, stem))?);
        }
        let at = existing.unwrap_or(self.behaviors.len() - 1);
        Ok(&mut self.behaviors[at])
    }
}

fn read_dir(dir: &Path) -> Result<impl Iterator<Item = PathBuf> + use<>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(entries.flatten().map(|e| e.path()))
}

/// UTF-8 (lossy) without `\r`.
pub(in crate::patch) fn read_text(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(String::from_utf8_lossy(&bytes).replace("\r\n", "\n"))
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use gui_shared::settings::pages::patch::behavior::ParserMode;

    use super::{import::import_asdsf, locations::code_dirs, *};

    /// A work kept when the format was only `Pandora` is a Pandora v5 one.
    #[test]
    fn an_old_pandora_work_is_pandora_v5() {
        let file: WorkspaceFile = sonic_rs::from_str(r#"{"export_format":"Pandora"}"#).unwrap();
        assert_eq!(file.export_format, Some(ExportFormat::PandoraV5));
    }

    /// NemesisExt was once chosen per behavior by a `force_ext` file: the work becomes NemesisExt and
    /// the file goes, or choosing Nemesis again would not last.
    #[test]
    fn an_old_force_ext_makes_the_work_nemesis_ext() {
        let dir = std::env::temp_dir().join(format!("d_merge_force_ext_{}", std::process::id()));
        let entry = dir.join("meshes~x.bin");
        std::fs::create_dir_all(&entry).unwrap();
        std::fs::write(entry.join("force_ext"), "").unwrap();

        let found = take_old_force_ext(&dir, std::slice::from_ref(&entry));
        let gone = !entry.join("force_ext").exists();
        let again = take_old_force_ext(&dir, &[entry]);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(found && gone && !again);
    }

    /// Only a patched template Nemesis does not know is outside what Nemesis reads: not one it
    /// knows, not one without patches, not animation data.
    #[test]
    fn the_templates_outside_nemesis() {
        let template = |meshes_path: &str, nemesis: Option<(String, bool)>, patched: bool| {
            let mut behavior = Behavior::open(&std::path::Path::new("meshes").join(format!("{ADSF}.bin"))).unwrap();
            (behavior.adsf, behavior.meshes_path, behavior.nemesis) = (false, meshes_path.to_owned(), nemesis);
            if patched {
                behavior.edits.insert("#0001".to_owned(), String::new());
            }
            behavior
        };
        let mut workspace = Workspace::default();
        workspace.behaviors.push(template("meshes/troll.bin", None, true));
        workspace.behaviors.push(template("meshes/bear.bin", None, false));
        workspace.behaviors.push(template("meshes/0_master.bin", Some(("0_master".to_owned(), false)), true));
        let mut clips = Behavior::open(&std::path::Path::new("meshes").join(format!("{ADSF}.bin"))).unwrap();
        clips.added.insert("Male~1/m$0.txt".to_owned(), String::new());
        workspace.behaviors.push(clips);

        assert_eq!(workspace.outside_nemesis().collect::<Vec<_>>(), ["meshes/troll.bin"]);
    }

    /// A mod of Nemesis only is a Nemesis work; one written elsewhere is an `Extended` one.
    #[test]
    fn the_scope_of_a_read_mod() {
        let mut workspace = Workspace::default();
        assert_eq!(TemplateScope::of(&workspace), TemplateScope::Nemesis);
        assert!(!TemplateScope::Nemesis.allows(false) && TemplateScope::Extended.allows(false));
        for format in [ExportFormat::NemesisExt, ExportFormat::PandoraV5] {
            workspace.export_format = format;
            assert_eq!(TemplateScope::of(&workspace), TemplateScope::Extended, "{format:?}");
        }
    }

    /// The shipped `.bin` gives the same anim sets as the `.txt` template.
    #[test]
    #[ignore = "needs the template files of the repository"]
    fn asdsf_bin_matches_txt() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resource");
        let bin = Behavior::open(&root.join("assets/templates/meshes/animationsetdatasinglefile.bin")).unwrap();
        let txt = Behavior::open(&root.join("xml/templates/meshes/animationsetdatasinglefile.txt")).unwrap();
        assert_eq!(bin.objects.len(), txt.objects.len());
        let differ = bin.objects.iter().zip(&txt.objects).filter(|(a, b)| a.id != b.id || a.text != b.text).count();
        assert_eq!(differ, 0);
    }

    /// A referenced mod's objects can be looked at and pointed to, and are not this mod's.
    #[test]
    fn a_referenced_mod_is_read_but_not_written() {
        let dir = std::env::temp_dir().join(format!("d_merge_ref_mods_{}", std::process::id()));
        let write_file = |path: &Path, text: &str| {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        let template = dir.join("templates/meshes/x.xml");
        let object = "\t\t<hkobject name=\"#0106\" class=\"hkbModifierList\" signature=\"0xa4180ca1\">\n\t\t\t<hkparam name=\"enable\">true</hkparam>\n\t\t</hkobject>\n";
        write_file(
            &template,
            &format!("<hkpackfile>\n\t<hksection name=\"__data__\">\n{object}\t</hksection>\n</hkpackfile>\n"),
        );
        let new_object =
            "\t\t<hkobject name=\"#amco$0\" class=\"hkbClipGenerator\" signature=\"0x333b85b9\">\n\t\t</hkobject>\n";
        let ref_dir = dir.join("mods/a/Nemesis_Engine/mod/amco");
        write_file(&ref_dir.join("meshes/x.bin/#amco$0.txt"), new_object);

        let mut workspace = Workspace::default();
        workspace.behaviors.push(Behavior::open(&template).unwrap());
        workspace.add_ref(&dir.join("mods/a"), "upatch");
        workspace.add_ref(&dir.join("mods/a"), "upatch"); // once

        let behavior = &workspace.behaviors[0];
        let shown = behavior.texts().find(|object| object.id == "#amco$0").map(|object| object.name);
        let (class, is_ref, text, patch, edited) = (
            behavior.class_of("#amco$0").map(str::to_owned),
            behavior.is_ref("#amco$0"),
            behavior.text("#amco$0").map(str::to_owned),
            behavior.patch("upatch", "#amco$0").map(|patch| patch.into_owned()),
            behavior.edits.len() + behavior.added.len(),
        );
        let shown = shown.map(str::to_owned);

        // Kept with the work.
        let output = dir.join("out");
        let session = Session {
            mod_dir: "D:/mods/a".to_owned(),
            output: "D:/mods/b".to_owned(),
            active: workspace.behaviors[0].name.clone(),
            selected: Some("#0106".to_owned()),
            tabs: vec![SessionTab {
                behavior: workspace.behaviors[0].name.clone(),
                id: "#0106".to_owned(),
                pinned: true,
            }],
        };
        workspace.session = session.clone();
        workspace.save(&output, "upatch").unwrap();
        let loaded = Workspace::load(&output, "upatch").unwrap();
        assert_eq!(loaded.session, session);
        assert_eq!(Workspace::cached_session(&output, "upatch"), Some(session));
        assert_eq!(Workspace::cached_session(&output, "none"), None);
        let reloaded = loaded.behaviors[0].is_ref("#amco$0");
        let ref_count = loaded.ref_mods.len();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(ref_count, 1);
        assert_eq!(class.as_deref(), Some("hkbClipGenerator"));
        assert!(is_ref);
        assert_eq!(text.as_deref(), Some(new_object));
        assert_eq!(shown.as_deref(), Some("ref: amco"));
        assert_eq!(patch, None, "nothing to export");
        assert_eq!(edited, 0);
        assert!(reloaded);
    }

    /// The export replaces the files read: an anim set file that is no edit of the template must
    /// come out as it went in. Real mods have `tkuc$New.txt` and `$header$.txt`.
    #[test]
    fn anim_set_files_that_are_no_edits_are_kept() {
        let dir = std::env::temp_dir().join(format!("d_merge_import_asdsf_{}", std::process::id()));
        let set = "V3\n0\n0\n0\n1\n1\n2\n7891816\n";
        let write = |path: &str, text: &str| {
            let path = dir.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write("templates/meshes/animationsetdatasinglefile.txt", &format!("1\nAData\\A.txt\n1\nx.txt\n{set}"));
        write(&format!("mod/{ASDSF}/AData~A/m$New.txt"), set);
        write(&format!("mod/{ASDSF}/AData~A/$header$.txt"), "1\nNew.txt\n");
        let mut skipped = Vec::new();

        let behavior = import_asdsf(&dir.join("mod"), &dir.join("templates"), &mut skipped).unwrap().unwrap();
        let patch = |id: &str| behavior.patch("m", id).unwrap();
        let (new_set, header) = (patch("AData~A/m$New.txt"), patch("AData~A/$header$.txt"));
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(skipped.is_empty());
        assert_eq!(new_set, set);
        assert_eq!(header, "1\nNew.txt\n");
        behavior.validate("AData~A/m$New.txt", &new_set, ParserMode::Strict).unwrap();
    }

    /// By default a mod is exported to where it was read from: what the work did not read must stay,
    /// and what it no longer has must go.
    #[test]
    fn an_export_only_replaces_what_the_work_read() {
        let dir = std::env::temp_dir().join(format!("d_merge_export_owned_{}", std::process::id()));
        let root = dir.join("mod");
        let write = |path: &Path, text: &str| {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        let set = "V3\n0\n0\n0\n1\n1\n2\n7891816\n";
        write(
            &dir.join("templates/meshes/animationsetdatasinglefile.txt"),
            &format!("1\nAData\\A.txt\n1\nx.txt\n{set}"),
        );
        let code_dir = root.join("Nemesis_Engine/mod/abcd");
        let (patch, readme, unknown) = (
            code_dir.join(ASDSF).join("AData~A/abcd$New.txt"),
            code_dir.join("readme.md"),
            code_dir.join("no_such_template/#0001.txt"),
        );
        write(&patch, set);
        write(&readme, "keep");
        write(&unknown, "keep");

        let (mut workspace, code, skipped) = Workspace::import(&root, &dir.join("templates")).unwrap();
        let exported = workspace.export(&root, &code, ParserMode::Strict).map(|(written, _)| written);
        let was_written = patch.is_file();
        let kept = (readme.is_file(), unknown.is_file());

        // The object is removed from the work: its file goes with it, and its folder that is left empty.
        workspace.behaviors[0].added.clear();
        workspace.export(&root, &code, ParserMode::Strict).unwrap();
        let removed = (patch.is_file(), code_dir.join(ASDSF).is_dir());
        let kept_after = (readme.is_file(), unknown.is_file());
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(code, "abcd");
        assert_eq!(skipped, ["no_such_template"]);
        assert_eq!(exported, Ok(1));
        assert!(was_written);
        assert_eq!(kept, (true, true));
        assert_eq!(removed, (false, false));
        assert_eq!(kept_after, (true, true));
    }

    #[test]
    fn the_codes_of_a_mod_are_listed_once_with_their_names() {
        let dir = std::env::temp_dir().join(format!("d_merge_mod_codes_{}", std::process::id()));
        let root = dir.join("Elden Counter");
        let (npc, dmg, ext) = (
            root.join("Nemesis_Engine/mod/ecnpc"),
            root.join("Nemesis_Engine/mod/ecdmg"),
            root.join("Nemesis_EngineExt/mod/ECNPC"),
        );
        for path in [&npc, &dmg, &ext] {
            std::fs::create_dir_all(path).unwrap();
        }
        std::fs::write(npc.join("info.ini"), "name=NPC counter\nauthor=a\n").unwrap();

        let codes = mod_codes_of_root(&root);
        let of_patch_dir = mod_codes_of_root(&npc);
        std::fs::remove_dir_all(&dir).unwrap();

        let code = |code: &str, name: &str| ModCode { code: code.to_owned(), name: name.to_owned() };
        assert_eq!(codes, [code("ecdmg", ""), code("ecnpc", "NPC counter")]);
        assert_eq!(of_patch_dir, []);
    }

    #[test]
    fn a_mod_is_found_from_its_root_or_patch_directory() {
        let dir = std::env::temp_dir().join(format!("d_merge_mod_root_{}", std::process::id()));
        let root = dir.join("Paraglider");
        let (nemesis, ext) = (root.join("Nemesis_Engine/mod/para"), root.join("Nemesis_EngineExt/mod/para"));
        std::fs::create_dir_all(&nemesis).unwrap();
        std::fs::create_dir_all(&ext).unwrap();
        std::fs::create_dir_all(dir.join("NotAMod/sub")).unwrap();

        let from_root = mod_root_of(&root);
        let from_patch_dir = mod_root_of(&nemesis);
        let from_ext = mod_root_of(&ext);
        let not_a_mod = (mod_root_of(&dir.join("NotAMod")), mod_root_of(&dir.join("NotAMod/sub")));
        let both = code_dirs(&root, "para");
        let codes = (code_of(&root), code_of(&nemesis));
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(from_root.as_ref(), Some(&root));
        assert_eq!(from_patch_dir.as_ref(), Some(&root));
        assert_eq!(from_ext.as_ref(), Some(&root));
        assert_eq!(not_a_mod, (None, None));
        assert_eq!(both, [nemesis, ext]);
        assert_eq!(codes.0, "para");
        assert_eq!(codes.1, "para");
    }

    /// By default a mod is exported over itself: what the editor has no field for must stay.
    /// Saving runs after each edit and each file opened: it must not write what did not change.
    #[test]
    fn a_save_only_writes_what_changed() {
        let dir = std::env::temp_dir().join(format!("d_merge_save_changes_{}", std::process::id()));
        let root = dir.join("caches");
        let template = dir.join("meshes").join(format!("{ADSF}.bin"));
        let mut workspace = Workspace::default();
        workspace.behaviors.push(Behavior::open(&template).unwrap());
        let behavior = &mut workspace.behaviors[0];
        behavior.added.insert("D~1/a.txt".to_owned(), "a\n".to_owned());
        behavior.added.insert("D~1/b.txt".to_owned(), "b\n".to_owned());
        workspace.save(&root, "mine").unwrap();

        let folder = Workspace::cache_dir(&root, "mine").join(workspace.behaviors[0].cache_name());
        let (a, b) = (folder.join("D~1%2Fa.txt.xml"), folder.join("D~1%2Fb.txt.xml"));
        let read = |path: &Path| std::fs::read_to_string(path).unwrap();
        assert_eq!(read(&a), "a\n");

        // What is written on the disk marks it: a file written again loses the mark.
        std::fs::write(&a, "mark").unwrap();
        workspace.behaviors[0].added.insert("D~1/b.txt".to_owned(), "changed\n".to_owned());
        workspace.save(&root, "mine").unwrap();
        assert_eq!(read(&a), "mark", "it did not change");
        assert_eq!(read(&b), "changed\n");

        // Only what is open changed: no patch is written.
        workspace.session.active = ADSF.to_owned();
        workspace.save(&root, "mine").unwrap();
        assert_eq!(read(&a), "mark");
        assert!(read(&Workspace::cache_dir(&root, "mine").join(WORKSPACE_FILE)).contains(ADSF));

        // What is gone goes, and with its last file the directory of the behavior.
        workspace.behaviors[0].added.remove("D~1/b.txt");
        workspace.save(&root, "mine").unwrap();
        assert!(!b.exists());
        workspace.behaviors.clear();
        workspace.save(&root, "mine").unwrap();
        assert!(!folder.exists());

        // Loaded, it knows what is there: nothing is written.
        let mut workspace = Workspace::default();
        workspace.behaviors.push(Behavior::open(&template).unwrap());
        workspace.behaviors[0].added.insert("D~1/a.txt".to_owned(), "a\n".to_owned());
        workspace.save(&root, "mine").unwrap();
        std::fs::write(&a, "a\n").unwrap();
        let mut loaded = Workspace::load(&root, "mine").unwrap();
        std::fs::write(&a, "mark").unwrap();
        loaded.save(&root, "mine").unwrap();
        assert_eq!(read(&a), "mark");

        // Somewhere else, it knows nothing: all of it is written.
        loaded.save(&dir.join("other"), "mine").unwrap();
        let elsewhere = Workspace::cache_dir(&dir.join("other"), "mine").join(loaded.behaviors[0].cache_name());
        assert_eq!(read(&elsewhere.join("D~1%2Fa.txt.xml")), "a\n");

        // The directory was deleted by hand: the same.
        std::fs::remove_dir_all(Workspace::cache_dir(&dir.join("other"), "mine")).unwrap();
        loaded.save(&dir.join("other"), "mine").unwrap();
        assert_eq!(read(&elsewhere.join("D~1%2Fa.txt.xml")), "a\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_export_keeps_what_the_mod_had() {
        let info = parse_info("name=a\nauthor=b\nsite=c\nauto=true\nmine=1\n\n; note\n");
        assert_eq!(info.other, ["auto=true", "mine=1", "; note"]);
        assert_eq!(info.exported(), "name=a\nauthor=b\nsite=c\nauto=true\nmine=1\n; note");

        // Nemesis wants it.
        assert_eq!(parse_info("name=a\n").exported(), "name=a\nauthor=\nsite=\nauto=null");
        // `null` is no value of these, and no line to keep.
        let info = parse_info("version=null\ndeps=null\n");
        assert_eq!((info.version.as_str(), info.deps.len(), info.other.len()), ("", 0, 0));
    }

    #[test]
    fn an_ext_mod_says_its_version_and_deps() {
        let mut workspace = Workspace::default();
        assert_eq!(workspace.ext_info("name=a", "mine"), "name=a", "nothing known: as Nemesis'");

        workspace.info.version = " 1.2.0 ".into();
        workspace.info.deps = vec!["bfco".into(), "mine".into()];
        workspace.ref_mods =
            vec!["D:/mods/a/Nemesis_Engine/mod/amco".into(), "D:/mods/b/Nemesis_Engine/mod/bfco".into()];
        assert_eq!(workspace.ext_info("name=a", "mine"), "name=a\nversion=1.2.0\ndeps=amco,bfco");
    }

    #[test]
    fn info_round_trip() {
        let info = ModInfo {
            name: "My Mod".into(),
            author: "me".into(),
            site: "https://x".into(),
            version: "1.2.0".into(),
            deps: vec!["amco".into(), "bfco".into()],
            other: vec!["auto=true".into(), "; a comment".into()],
        };
        assert_eq!(parse_info(&info_ini(&info)), info);
    }
}
