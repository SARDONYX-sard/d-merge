//! A patch being made: the edited objects of each behavior, kept by object id.
//!
//! # Why by id (not text positions)
//! Nemesis matches patches by object id, and ids stay valid when the template text changes
//! (positions would not). Only the changed objects are stored, so a 1.5 MB template costs nothing.
//!
//! # On disk (`<output>/.d_merge/caches/patch_editor/nemesis/<code>/`)
//! ```text
//! info.ini                      name / author / site
//! meshes~actors~draugr~behaviors~draugrbehavior.bin/
//!   template.path               the template xml this behavior was made from
//!   force_ext                   (present = write as NemesisExt)
//!   #0106.xml                   an edited template object (full object text)
//!   #mymod$0.xml                a new object
//! ```
//! [`Workspace::export`] writes the real mod: `<output>/Nemesis_Engine/mod/<code>/` for templates
//! Nemesis supports, `<output>/Nemesis_EngineExt/mod/<code>/meshes/...` for the others.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use nemesis_diff::{View, XmlObject, normalize, object_patch, objects, resolve};

/// Mod metadata (`info.ini`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ModInfo {
    pub(crate) name: String,
    pub(crate) author: String,
    pub(crate) site: String,
}

/// One template object, indexed once when the template is loaded.
#[derive(Debug, Clone)]
pub(crate) struct TemplateObject {
    pub(crate) id: String,
    pub(crate) class: String,
    /// Its `name` field, if any (e.g. `GetUpModList`), for the list and the filter.
    pub(crate) name: String,
    pub(crate) text: String,
}

/// Where a behavior's patches go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Target {
    /// `Nemesis_Engine/mod/<code>/[_1stperson/]<name>/`: only the templates Nemesis knows
    /// (`NEMESIS_*_MAP` of `behavior_merge_types`).
    Nemesis { name: String, first_person: bool },
    /// `Nemesis_EngineExt/mod/<code>/<meshes path>.bin/`: any template.
    Ext,
}

/// The work on one behavior file (e.g. `draugrbehavior`).
#[derive(Debug)]
pub(crate) struct Behavior {
    /// Shown on the tab: the template's lowercase file stem (`_1stperson/` prefixed).
    pub(crate) name: String,
    pub(crate) template_path: PathBuf,
    /// e.g. `meshes/actors/draugr/behaviors/draugrbehavior.bin` (what Nemesis / d-merge key on).
    pub(crate) meshes_path: String,
    /// The Nemesis folder, if Nemesis supports this template.
    pub(crate) nemesis: Option<(String, bool)>,
    /// Write as NemesisExt even if Nemesis supports it.
    pub(crate) force_ext: bool,
    pub(crate) objects: Vec<TemplateObject>,
    /// Edited template objects: id -> object text.
    pub(crate) edits: BTreeMap<String, String>,
    /// New objects: `#code$N` -> object text.
    pub(crate) added: BTreeMap<String, String>,
    /// `animationsetdatasinglefile`: the "objects" are anim sets (`Project~Data/file.txt`), patched
    /// as `Nemesis_Engine/mod/<code>/animationsetdatasinglefile/<id>`. No new objects.
    pub(crate) asdsf: bool,
}

/// File stem of the anim set data template (`meshes/animationsetdatasinglefile.bin|txt`).
pub(crate) const ASDSF: &str = "animationsetdatasinglefile";

impl Behavior {
    /// Reads and indexes the template (behavior xml, or the anim set data txt).
    /// A `.bin` (d-merge's shipped template) is converted to text first; ids are the template's.
    pub(crate) fn open(template_path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(template_path)
            .map_err(|e| format!("{}: {e}", template_path.display()))?;
        let is_bin = template_path.extension().is_some_and(|e| e.eq_ignore_ascii_case("bin"));
        let asdsf = template_path.file_stem().is_some_and(|s| s.eq_ignore_ascii_case(ASDSF));
        let error = |e: String| format!("{}: {e}", template_path.display());
        if asdsf {
            let text = if is_bin {
                asdsf_bin_to_txt(&bytes).map_err(error)?
            } else {
                String::from_utf8_lossy(&bytes).into_owned()
            };
            let objects = nemesis_diff::asdsf::anim_sets(&text.replace("\r\n", "\n"))
                .map_err(error)?
                .into_iter()
                .map(|set| TemplateObject {
                    id: set.id(),
                    class: set.project,
                    name: set.file,
                    text: set.text,
                })
                .collect();
            return Ok(Self {
                name: ASDSF.to_string(),
                template_path: template_path.to_path_buf(),
                meshes_path: meshes_path(template_path),
                nemesis: Some((ASDSF.to_string(), false)),
                force_ext: false,
                objects,
                edits: BTreeMap::new(),
                added: BTreeMap::new(),
                asdsf: true,
            });
        }
        let xml = if is_bin {
            hkx_io::template_bin_to_xml(&bytes).map_err(error)?
        } else {
            String::from_utf8_lossy(&bytes).replace("\r\n", "\n")
        };
        let meshes_path = meshes_path(template_path);
        let nemesis = nemesis_folder(&meshes_path);
        let stem = template_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let name = match &nemesis {
            Some((_, true)) => format!("_1stperson/{stem}"),
            _ => stem,
        };
        Ok(Self {
            name,
            template_path: template_path.to_path_buf(),
            meshes_path,
            nemesis,
            force_ext: false,
            objects: objects(&xml).into_iter().map(index).collect(),
            edits: BTreeMap::new(),
            added: BTreeMap::new(),
            asdsf: false,
        })
    }

    /// Patch file name of `id` (in [`Self::patch_dir`]): `#0106.txt`, or the anim set path as is.
    pub(crate) fn patch_file(&self, id: &str) -> String {
        if self.asdsf { id.to_string() } else { format!("{id}.txt") }
    }

    /// d-merge's parser check of a patch (behaviors only: anim set patches are plain text).
    pub(crate) fn validate(&self, patch: &str) -> Result<(), String> {
        if self.asdsf { Ok(()) } else { nemesis_diff::validate(patch) }
    }

    pub(crate) fn target(&self) -> Target {
        match &self.nemesis {
            Some((name, first_person)) if !self.force_ext => {
                Target::Nemesis { name: name.clone(), first_person: *first_person }
            }
            _ => Target::Ext,
        }
    }

    /// The patch folder of this behavior under `<output>`.
    ///
    /// e.g. `Nemesis_Engine/mod/mymod/_1stperson/0_master`,
    /// `Nemesis_EngineExt/mod/mymod/meshes/actors/troll/behaviors/trollbehavior.bin`
    pub(crate) fn patch_dir(&self, code: &str) -> PathBuf {
        if self.asdsf {
            return Path::new("Nemesis_Engine").join("mod").join(code).join(ASDSF);
        }
        match self.target() {
            Target::Nemesis { name, first_person } => {
                let dir = Path::new("Nemesis_Engine").join("mod").join(code);
                if first_person { dir.join("_1stperson").join(name) } else { dir.join(name) }
            }
            Target::Ext => {
                Path::new("Nemesis_EngineExt").join("mod").join(code).join(&self.meshes_path)
            }
        }
    }

    /// Cache folder name: unique per template (`0_master` exists in 1st and 3rd person).
    fn cache_name(&self) -> String {
        self.meshes_path.replace(['/', '\\'], "~")
    }
    /// The id of the first template object of `class` (e.g. the `hkbBehaviorGraphStringData`).
    pub(crate) fn object_of_class(&self, class: &str) -> Option<&str> {
        self.objects.iter().find(|o| o.class == class).map(|o| o.id.as_str())
    }

    /// The class of object `id` (a template object or a new one).
    pub(crate) fn class_of(&self, id: &str) -> Option<String> {
        if let Some(text) = self.added.get(id) {
            let class = text.split_once("class=\"")?.1.split('"').next()?;
            return Some(class.to_string());
        }
        self.template_object(id).map(|o| o.class.clone())
    }

    pub(crate) fn template_object(&self, id: &str) -> Option<&TemplateObject> {
        self.objects.iter().find(|o| o.id == id)
    }

    /// The current text of `id`: its edit, a new object, or the template object.
    pub(crate) fn text(&self, id: &str) -> Option<&str> {
        self.edits
            .get(id)
            .or_else(|| self.added.get(id))
            .map(String::as_str)
            .or_else(|| self.template_object(id).map(|o| o.text.as_str()))
    }

    /// Stores `text` as the new text of `id` (an edit back to the template removes the edit).
    pub(crate) fn set_text(&mut self, id: &str, text: String) {
        if let Some(added) = self.added.get_mut(id) {
            *added = text;
        } else if self.template_object(id).is_some_and(|o| normalize(&o.text) == normalize(&text)) {
            self.edits.remove(id);
        } else {
            self.edits.insert(id.to_string(), text);
        }
    }

    /// The `#XXXX.txt` contents of an edited object, or the new object's text.
    pub(crate) fn patch(&self, code: &str, id: &str) -> Option<String> {
        if let Some(text) = self.added.get(id) {
            return Some(normalize(text));
        }
        object_patch(code, &self.template_object(id)?.text, self.edits.get(id)?)
    }

    /// `#code$N` with the next free `N`.
    fn next_id(&self, code: &str) -> String {
        let prefix = format!("#{code}$");
        let next = self
            .added
            .keys()
            .filter_map(|id| id.strip_prefix(&prefix)?.parse::<usize>().ok())
            .max()
            .map_or(0, |n| n + 1);
        format!("{prefix}{next}")
    }

    /// Adds a new object of `class` with every field at its default (see [`nemesis_diff::schema::skeleton`]).
    pub(crate) fn add_class(&mut self, code: &str, class: &str) -> Option<String> {
        let id = self.next_id(code);
        let text = nemesis_diff::schema::skeleton(class, &id)?;
        self.added.insert(id.clone(), text);
        Some(id)
    }

    /// Adds a copy of `from` (or an empty object) as `#code$N`, with the next free `N`.
    pub(crate) fn add_object(&mut self, code: &str, from: Option<&str>) -> String {
        let id = self.next_id(code);
        let text = match from.and_then(|from| Some((from, self.text(from)?))) {
            Some((from, text)) => {
                text.replacen(&format!("name=\"{from}\""), &format!("name=\"{id}\""), 1)
            }
            None => {
                format!("\t\t<hkobject name=\"{id}\" class=\"\" signature=\"\">\n\t\t</hkobject>\n")
            }
        };
        self.added.insert(id.clone(), text);
        id
    }
}

/// d-merge's `animationsetdatasinglefile.bin` (MessagePack) -> the original txt.
fn asdsf_bin_to_txt(bytes: &[u8]) -> Result<String, String> {
    use skyrim_anim_data::{
        asdsf::alt::{AltAsdsf, ser::serialize_alt_asdsf},
        diff_line::DiffLines,
    };
    let asdsf: AltAsdsf<'_> = rmp_serde::from_slice(bytes).map_err(|e| e.to_string())?;
    serialize_alt_asdsf(asdsf, DiffLines::DEFAULT, Default::default()).map_err(|e| e.to_string())
}

/// Cache file name of an id: `/` (anim set ids) can not be in a file name.
fn cache_file(id: &str) -> String {
    format!("{}.xml", id.replace('/', "%2F"))
}

fn index(object: XmlObject<'_>) -> TemplateObject {
    let name = object
        .text
        .split_once("<hkparam name=\"name\">")
        .and_then(|(_, rest)| rest.split_once("</hkparam>"))
        .map(|(name, _)| name.to_string())
        .unwrap_or_default();
    TemplateObject {
        id: object.id.to_string(),
        class: object.class.to_string(),
        name,
        text: object.text.to_string(),
    }
}

/// The whole patch being made.
#[derive(Debug, Default)]
pub(crate) struct Workspace {
    pub(crate) info: ModInfo,
    pub(crate) behaviors: Vec<Behavior>,
}

impl Workspace {
    /// `<output>/.d_merge/caches/patch_editor/nemesis/<code>`
    pub(crate) fn cache_dir(output: &Path, code: &str) -> PathBuf {
        output.join(".d_merge").join("caches").join("patch_editor").join("nemesis").join(code)
    }

    /// Loads a cached workspace (an empty one if there is none).
    pub(crate) fn load(output: &Path, code: &str) -> Result<Self, String> {
        let dir = Self::cache_dir(output, code);
        let mut workspace = Self::default();
        if !dir.is_dir() {
            return Ok(workspace);
        }
        workspace.info = std::fs::read_to_string(dir.join("info.ini"))
            .map(|s| parse_info(&s))
            .unwrap_or_default();

        for entry in read_dir(&dir)?.filter(|p| p.is_dir()) {
            let Ok(template) = std::fs::read_to_string(entry.join("template.path")) else {
                continue;
            };
            let mut behavior = Behavior::open(Path::new(template.trim()))?;
            behavior.force_ext = entry.join("force_ext").exists();
            for file in read_dir(&entry)?.filter(|p| p.extension().is_some_and(|e| e == "xml")) {
                let id = file_stem(&file).replace("%2F", "/");
                let text = read_text(&file)?;
                if behavior.template_object(&id).is_some() {
                    behavior.edits.insert(id, text);
                } else {
                    behavior.added.insert(id, text);
                }
            }
            workspace.behaviors.push(behavior);
        }
        Ok(workspace)
    }

    /// Writes the whole workspace to its cache dir (replacing what was there).
    pub(crate) fn save(&self, output: &Path, code: &str) -> Result<(), String> {
        let dir = Self::cache_dir(output, code);
        if dir.is_dir() {
            std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        write(&dir.join("info.ini"), &info_ini(&self.info))?;
        for behavior in &self.behaviors {
            let behavior_dir = dir.join(behavior.cache_name());
            write(
                &behavior_dir.join("template.path"),
                &behavior.template_path.display().to_string(),
            )?;
            if behavior.force_ext {
                write(&behavior_dir.join("force_ext"), "")?;
            }
            for (id, text) in behavior.edits.iter().chain(&behavior.added) {
                write(&behavior_dir.join(cache_file(id)), text)?;
            }
        }
        Ok(())
    }

    /// Imports a mod: `.../Nemesis_Engine/mod/<code>` or `.../Nemesis_EngineExt/mod/<code>`.
    ///
    /// Each patch folder is resolved to a template xml under `template_root`:
    /// - `<name>/`, `_1stperson/<name>/`: by Nemesis' template table
    /// - `meshes/.../<x>.bin/` (NemesisExt): by that path (`.bin` -> `.xml`)
    ///
    /// Returns `(workspace, code, folders without a template)`.
    pub(crate) fn import(
        mod_dir: &Path,
        template_root: &Path,
    ) -> Result<(Self, String, Vec<String>), String> {
        let code =
            mod_dir.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let mut workspace = Self {
            info: std::fs::read_to_string(mod_dir.join("info.ini"))
                .map(|s| parse_info(&s))
                .unwrap_or_default(),
            behaviors: Vec::new(),
        };
        let mut skipped = Vec::new();
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
            let mut behavior = Behavior::open(&template)?;
            behavior.force_ext =
                relative.to_ascii_lowercase().starts_with("meshes/") && behavior.nemesis.is_some();
            for file in read_dir(&folder)?.filter(|p| p.extension().is_some_and(|e| e == "txt")) {
                let id = file_stem(&file);
                let text = resolve(&read_text(&file)?, View::Mod);
                if behavior.template_object(&id).is_some() {
                    behavior.set_text(&id, text);
                } else {
                    behavior.added.insert(id, text);
                }
            }
            workspace.behaviors.push(behavior);
        }
        if let Some(asdsf) = import_asdsf(mod_dir, template_root, &mut skipped)? {
            workspace.behaviors.push(asdsf);
        }
        Ok((workspace, code, skipped))
    }

    /// Writes the mod under `<output>` (replacing `Nemesis_Engine/mod/<code>` and
    /// `Nemesis_EngineExt/mod/<code>`): each behavior to its [`Behavior::patch_dir`], `info.ini`
    /// to each mod root used. Returns the files written and the patches d-merge's Nemesis parser
    /// rejects (`behavior/#id: error`).
    pub(crate) fn export(&self, output: &Path, code: &str) -> Result<(usize, Vec<String>), String> {
        let roots = [
            output.join("Nemesis_Engine").join("mod").join(code),
            output.join("Nemesis_EngineExt").join("mod").join(code),
        ];
        for root in roots.iter().filter(|r| r.is_dir()) {
            std::fs::remove_dir_all(root).map_err(|e| format!("{}: {e}", root.display()))?;
        }
        let mut info =
            nemesis_diff::info_ini(&self.info.name, &self.info.author, &self.info.site, false);
        info.truncate(info.trim_end().len());

        let (mut written, mut invalid) = (0, Vec::new());
        for behavior in &self.behaviors {
            let dir = output.join(behavior.patch_dir(code));
            for id in behavior.edits.keys().chain(behavior.added.keys()) {
                let Some(patch) = behavior.patch(code, id) else { continue };
                if let Err(err) = behavior.validate(&patch) {
                    invalid.push(format!("{}/{id}: {err}", behavior.name));
                }
                write(&dir.join(behavior.patch_file(id)), &patch)?;
                written += 1;
            }
        }
        for root in roots.iter().filter(|r| r.is_dir()) {
            write(&root.join("info.ini"), &info)?;
        }
        Ok((written, invalid))
    }
}

/// `<mod>/animationsetdatasinglefile/<project>/<set>.txt` -> an anim set "behavior" (or `None`).
/// Sets the template does not have are added to `skipped`.
fn import_asdsf(
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
        let project_name =
            project.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        for file in read_dir(&project)?.filter(|p| p.is_file()) {
            let file_name =
                file.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let wanted = format!("{project_name}/{file_name}");
            // Folder / file names are case-insensitive.
            let Some(id) = behavior
                .objects
                .iter()
                .find(|o| o.id.eq_ignore_ascii_case(&wanted))
                .map(|o| o.id.clone())
            else {
                skipped.push(format!("{ASDSF}/{wanted}"));
                continue;
            };
            behavior.set_text(&id, resolve(&read_text(&file)?, View::Mod));
        }
    }
    Ok(Some(behavior))
}
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

/// Every `.bin` / `.xml` template under `template_dir` (sorted; a `.bin` wins over its `.xml`).
pub(crate) fn catalog(template_dir: &Path) -> Vec<TemplateEntry> {
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
            let is_asdsf_txt = ext.as_deref() == Some("txt")
                && path.file_stem().is_some_and(|s| s.eq_ignore_ascii_case(ASDSF));
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
    found.into_values().collect()
}

fn meshes_path(template: &Path) -> String {
    let parts: Vec<String> =
        template.components().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).collect();
    let from =
        parts.iter().rposition(|p| p == "meshes").unwrap_or_else(|| parts.len().saturating_sub(1));
    let joined = parts[from..].join("/");
    Path::new(&joined).with_extension("bin").to_string_lossy().replace('\\', "/")
}

/// The Nemesis folder `(name, 1st person)` of a template, if Nemesis supports it.
fn nemesis_folder(meshes_path: &str) -> Option<(String, bool)> {
    use behavior_merge_types::template_key::{NEMESIS_1ST_PERSON_MAP, NEMESIS_3RD_PERSON_MAP};
    let find = |map: &phf::Map<&'static str, &'static str>| {
        map.entries()
            .find(|(_, path)| path.eq_ignore_ascii_case(meshes_path))
            .map(|(name, _)| (*name).to_string())
    };
    find(&NEMESIS_3RD_PERSON_MAP)
        .map(|name| (name, false))
        .or_else(|| find(&NEMESIS_1ST_PERSON_MAP).map(|name| (name, true)))
}

/// A patch folder (relative to the mod dir, `/` separated) -> its template's meshes path.
///
/// `0_master` / `_1stperson/0_master` by Nemesis' table, `meshes/.../x.bin` as is.
fn meshes_path_of_folder(relative: &str) -> Option<String> {
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
fn patch_folders(mod_dir: &Path) -> Vec<PathBuf> {
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

fn file_stem(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}
fn info_ini(info: &ModInfo) -> String {
    format!("name={}\nauthor={}\nsite={}\n", info.name, info.author, info.site)
}

fn parse_info(ini: &str) -> ModInfo {
    let mut info = ModInfo::default();
    for line in ini.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        let value = value.trim().to_string();
        match key.trim() {
            "name" => info.name = value,
            "author" => info.author = value,
            "site" => info.site = value,
            _ => {}
        }
    }
    info
}

fn read_dir(dir: &Path) -> Result<impl Iterator<Item = PathBuf>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(entries.flatten().map(|e| e.path()))
}

/// UTF-8 (lossy) without `\r`.
fn read_text(path: &Path) -> Result<String, String> {
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
    use super::*;

    /// The shipped `.bin` gives the same anim sets as the `.txt` template.
    #[test]
    #[ignore = "needs the template files of the repository"]
    fn asdsf_bin_matches_txt() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resource");
        let bin =
            Behavior::open(&root.join("assets/templates/meshes/animationsetdatasinglefile.bin"))
                .unwrap();
        let txt = Behavior::open(&root.join("xml/templates/meshes/animationsetdatasinglefile.txt"))
            .unwrap();
        assert_eq!(bin.objects.len(), txt.objects.len());
        let differ = bin
            .objects
            .iter()
            .zip(&txt.objects)
            .filter(|(a, b)| a.id != b.id || a.text != b.text)
            .count();
        assert_eq!(differ, 0);
    }

    #[test]
    fn info_round_trip() {
        let info = ModInfo { name: "My Mod".into(), author: "me".into(), site: "https://x".into() };
        assert_eq!(parse_info(&info_ini(&info)), info);
    }
}
