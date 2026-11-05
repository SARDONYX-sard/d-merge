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
//!   force_ext                   (present = write as NemesisExt)
//!   #0106.xml                   an edited template object (full object text)
//!   #mymod$0.xml                a new object
//! ```
//! [`Workspace::export`] writes the real mod: `<output>/Nemesis_Engine/mod/<code>/` for templates
//! Nemesis supports, `<output>/Nemesis_EngineExt/mod/<code>/meshes/...` for the others.

use std::{
    borrow::Cow,
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use gui_shared::settings::pages::patch::behavior::ParserMode;
use nemesis_diff::{View, XmlObject, apply_to_template, normalize, object_patch, objects, resolve};
use rayon::prelude::*;

/// Mod metadata (`info.ini`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ModInfo {
    pub(crate) name: String,
    pub(crate) author: String,
    pub(crate) site: String,
    /// The mod's version now, written for `Nemesis_EngineExt` only. e.g. `1.2.0`
    pub(crate) version: String,
    /// Mod codes it says it depends on, as it was read. See [`Workspace::deps`].
    pub(crate) deps: Vec<String>,
    /// The lines the editor has no field for, as they were read: the mod's, so they stay.
    ///
    /// e.g. `auto=true`
    pub(crate) other: Vec<String>,
}

impl ModInfo {
    /// `info.ini` of the mod, without a line end at the end. Nemesis wants `auto`, so a mod has it.
    fn exported(&self) -> String {
        let mut out = format!("name={}\nauthor={}\nsite={}", self.name, self.author, self.site);
        for line in &self.other {
            out.push('\n');
            out.push_str(line);
        }
        let is_auto = |line: &String| line.split_once('=').is_some_and(|(key, _)| key.trim() == "auto");
        if !self.other.iter().any(is_auto) {
            out.push_str("\nauto=null");
        }
        out
    }
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
    /// `id` -> index in `objects`. The lookups run for every object in several places, so a scan
    /// of `objects` (a few thousand) each time would be quadratic.
    object_index: fast_hash::HashMap<String, u32>,
    /// Edited template objects: id -> object text.
    pub(crate) edits: BTreeMap<String, String>,
    /// New objects: `#code$N` -> object text.
    pub(crate) added: BTreeMap<String, String>,
    /// `animationsetdatasinglefile`: the "objects" are anim sets (`Project~Data/file.txt`), patched
    /// as `Nemesis_Engine/mod/<code>/animationsetdatasinglefile/<id>`. No new objects.
    pub(crate) asdsf: bool,
    /// `animationdatasinglefile`: the "objects" are clip data files (`DefaultMale~1/Idle~mod$0.txt`),
    /// written as they are to `Nemesis_Engine/mod/<code>/animationdatasinglefile/<id>`.
    /// No template is read: new clips do not need one.
    pub(crate) adsf: bool,
    /// The new objects of the referenced mods, by id. Read only, and not written anywhere.
    /// See [`Workspace::ref_mods`].
    pub(crate) refs: BTreeMap<String, RefObject>,
    /// The events and variables the referenced mods add.
    pub(crate) ref_events: Vec<String>,
    pub(crate) ref_variables: Vec<String>,
}

/// A new object of a referenced mod. See [`Workspace::ref_mods`].
#[derive(Debug, Clone)]
pub(crate) struct RefObject {
    pub(crate) class: String,
    /// Shown as its name in the lists, e.g. `ref: amco`.
    pub(crate) label: String,
    pub(crate) text: String,
}

/// Where the works are kept: in the patch page's output directory, so for each profile, not in the mods.
///
/// e.g. `D:/out` -> `D:/out/.d_merge/caches/patch_editor/nemesis`
pub(crate) fn cache_root_of(output_dir: &str) -> PathBuf {
    Path::new(output_dir).join(".d_merge").join("caches").join("patch_editor").join("nemesis")
}

/// The directories of a mod's root that hold patches, in `<engine>/mod/<code>`.
const ENGINES: [&str; 2] = ["Nemesis_Engine", "Nemesis_EngineExt"];

/// The root of the Nemesis mod at `selected`: itself, or what it is the patch directory of.
/// `None`: it is no Nemesis mod.
///
/// e.g. `mods/Paraglider` containing `Nemesis_Engine/mod/para/` -> itself;
/// `mods/Paraglider/Nemesis_Engine/mod/para` -> `mods/Paraglider`.
pub(crate) fn mod_root_of(selected: &Path) -> Option<PathBuf> {
    let has_code = |engine: &str| {
        read_dir(&selected.join(engine).join("mod")).is_ok_and(|mut codes| codes.any(|path| path.is_dir()))
    };
    if ENGINES.iter().any(|engine| has_code(engine)) {
        return Some(selected.to_path_buf());
    }
    let is_named = |path: &Path, name: &str| path.file_name().is_some_and(|found| found.eq_ignore_ascii_case(name));
    let mods = selected.parent().filter(|path| is_named(path, "mod"))?;
    let engine = mods.parent().filter(|path| ENGINES.iter().any(|name| is_named(path, name)))?;
    engine.parent().map(Path::to_path_buf)
}

/// The directories of `code` in the mod at `root`, of the engines it has.
fn code_dirs(root: &Path, code: &str) -> Vec<PathBuf> {
    let dirs = ENGINES.iter().map(|engine| root.join(engine).join("mod").join(code));
    dirs.filter(|dir| dir.is_dir()).collect()
}

/// A patch directory of `code` in the mod at `root`, if it has one.
pub(crate) fn patch_dir_of_code(root: &Path, code: &str) -> Option<PathBuf> {
    code_dirs(root, code).into_iter().next()
}

/// The code of the mod at `selected` (its root or a patch directory). See [`mod_dir_of`].
pub(crate) fn code_of(selected: &Path) -> String {
    mod_dir_of(selected).file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
}

/// The patch directory of a mod, given its root (what a mod manager installs).
///
/// e.g., `mods/Paraglider` containing `Nemesis_Engine/mod/para/` -> `.../Nemesis_Engine/mod/para`.
/// A directory that is not a mod root is returned as is. If a mod has several codes, the first one
/// by name is taken.
///
/// The root is also where a later anim set generator finds `meshes/**/animations/*.hkx`.
fn mod_dir_of(selected: &Path) -> std::borrow::Cow<'_, Path> {
    let code_dir = ENGINES.iter().find_map(|engine| {
        let entries = std::fs::read_dir(selected.join(engine).join("mod")).ok()?;
        entries.flatten().map(|entry| entry.path()).filter(|path| path.is_dir()).min()
    });
    code_dir.map_or(std::borrow::Cow::Borrowed(selected), std::borrow::Cow::Owned)
}

/// The class of the object holding the event and variable names.
pub(crate) const STRING_DATA: &str = "hkbBehaviorGraphStringData";

/// `id` -> position, for [`Behavior::template_object`].
fn index_objects(objects: &[TemplateObject]) -> fast_hash::HashMap<String, u32> {
    #[expect(clippy::cast_possible_truncation, reason = "far fewer than 4 billion objects")]
    objects.iter().enumerate().map(|(index, object)| (object.id.clone(), index as u32)).collect()
}

/// One object of [`Behavior::texts`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct ObjectText<'a> {
    /// e.g. `#0106`, `#mymod$0`
    pub(crate) id: &'a str,
    /// e.g. `hkbClipGenerator`
    pub(crate) class: &'a str,
    /// The template's `name` field, e.g. `GetUpModList` (`""` for new objects).
    pub(crate) name: &'a str,
    pub(crate) text: &'a str,
    pub(crate) is_new: bool,
    pub(crate) is_edited: bool,
}

/// `<hkobject name="#m$0" class="hkbClipGenerator" ...>` -> `hkbClipGenerator` (`""` if missing).
fn class_attribute(text: &str) -> &str {
    text.split_once("class=\"").and_then(|(_, rest)| rest.split('"').next()).unwrap_or_default()
}

/// File stem of the anim set data template (`meshes/animationsetdatasinglefile.bin|txt`).
pub(crate) const ASDSF: &str = "animationsetdatasinglefile";

/// File stem of the clip data template (`meshes/animationdatasinglefile.bin|txt`).
pub(crate) const ADSF: &str = "animationdatasinglefile";

impl Behavior {
    /// Reads and indexes the template (behavior xml, or the anim set data txt).
    /// A `.bin` (d-merge's shipped template) is converted to text first; ids are the template's.
    pub(crate) fn open(template_path: &Path) -> Result<Self, String> {
        if template_path.file_stem().is_some_and(|s| s.eq_ignore_ascii_case(ADSF)) {
            return Ok(Self {
                name: ADSF.to_string(),
                template_path: template_path.to_path_buf(),
                meshes_path: format!("meshes/{ADSF}.bin"),
                nemesis: Some((ADSF.to_string(), false)),
                force_ext: false,
                object_index: fast_hash::HashMap::default(),
                objects: Vec::new(),
                edits: BTreeMap::new(),
                added: BTreeMap::new(),
                asdsf: false,
                adsf: true,
                refs: BTreeMap::new(),
                ref_events: Vec::new(),
                ref_variables: Vec::new(),
            });
        }
        let bytes = std::fs::read(template_path).map_err(|e| format!("{}: {e}", template_path.display()))?;
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
                .map(|set| TemplateObject { id: set.id(), class: set.project, name: set.file, text: set.text })
                .collect::<Vec<_>>();
            return Ok(Self {
                name: ASDSF.to_string(),
                template_path: template_path.to_path_buf(),
                meshes_path: meshes_path(template_path),
                nemesis: Some((ASDSF.to_string(), false)),
                force_ext: false,
                object_index: index_objects(&objects),
                objects,
                edits: BTreeMap::new(),
                added: BTreeMap::new(),
                asdsf: true,
                adsf: false,
                refs: BTreeMap::new(),
                ref_events: Vec::new(),
                ref_variables: Vec::new(),
            });
        }
        let xml = if is_bin {
            hkx_io::template_bin_to_xml(&bytes).map_err(error)?
        } else {
            String::from_utf8_lossy(&bytes).replace("\r\n", "\n")
        };
        let meshes_path = meshes_path(template_path);
        let nemesis = nemesis_folder(&meshes_path);
        let stem = template_path.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
        let name = match &nemesis {
            Some((_, true)) => format!("_1stperson/{stem}"),
            _ => stem,
        };
        let objects: Vec<_> = objects(&xml).into_iter().map(index).collect();
        Ok(Self {
            name,
            template_path: template_path.to_path_buf(),
            meshes_path,
            nemesis,
            force_ext: false,
            object_index: index_objects(&objects),
            objects,
            edits: BTreeMap::new(),
            added: BTreeMap::new(),
            asdsf: false,
            adsf: false,
            refs: BTreeMap::new(),
            ref_events: Vec::new(),
            ref_variables: Vec::new(),
        })
    }

    /// Not a behavior but animation data: plain text, no objects with pointers, no new objects.
    pub(crate) const fn is_anim_data(&self) -> bool {
        self.asdsf || self.adsf
    }

    /// Patch file name of `id` (in [`Self::patch_dir`]): `#0106.txt`, or an anim data path as is.
    pub(crate) fn patch_file(&self, id: &str) -> String {
        if self.is_anim_data() { id.to_string() } else { format!("{id}.txt") }
    }

    /// d-merge's parser check of a patch, in the mode of the patch page.
    pub(crate) fn validate(&self, id: &str, patch: &str, mode: ParserMode) -> Result<(), String> {
        if self.adsf {
            super::anim_patch::validate_adsf(id, patch, mode)
        } else if self.asdsf {
            super::anim_patch::validate_asdsf(id, patch)
        } else {
            nemesis_diff::validate(patch)
        }
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
        if self.adsf {
            return Path::new("Nemesis_Engine").join("mod").join(code).join(ADSF);
        }
        match self.target() {
            Target::Nemesis { name, first_person } => {
                let dir = Path::new("Nemesis_Engine").join("mod").join(code);
                if first_person { dir.join("_1stperson").join(name) } else { dir.join(name) }
            }
            Target::Ext => Path::new("Nemesis_EngineExt").join("mod").join(code).join(&self.meshes_path),
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
    pub(crate) fn class_of(&self, id: &str) -> Option<&str> {
        if let Some(text) = self.added.get(id) {
            return Some(class_attribute(text));
        }
        self.template_object(id)
            .map(|o| o.class.as_str())
            .or_else(|| self.refs.get(id).map(|object| object.class.as_str()))
    }

    /// Is `id` an object of a referenced mod, which can be looked at but not changed?
    pub(crate) fn is_ref(&self, id: &str) -> bool {
        self.refs.contains_key(id) && !self.added.contains_key(id)
    }

    /// The folders of `mod_dir` that could hold patches of this template.
    ///
    /// e.g. `<mod_dir>/_1stperson/0_master`, `<mod_dir>/meshes/actors/x/behaviors/x.bin`
    fn patch_folders_in(&self, mod_dir: &Path) -> Vec<PathBuf> {
        let mut folders = vec![mod_dir.join(&self.meshes_path)];
        if let Some((name, first_person)) = &self.nemesis {
            let folder = if *first_person { mod_dir.join("_1stperson").join(name) } else { mod_dir.join(name) };
            folders.push(folder);
        }
        folders
    }

    /// Takes in what the referenced mod at `mod_dir` adds to this template: its new objects, and the
    /// events and variables in its patch of the names object. Its edits of template objects are
    /// not: those are its own business, and it is merged with this mod anyway.
    fn read_ref(&mut self, mod_dir: &Path) {
        if self.is_anim_data() {
            return;
        }
        let code = mod_dir.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        let files: Vec<PathBuf> = self
            .patch_folders_in(mod_dir)
            .iter()
            .filter_map(|folder| read_dir(folder).ok())
            .flatten()
            .filter(|path| file_stem(path).starts_with('#'))
            .collect();
        for file in files {
            let Ok(text) = read_text(&file) else { continue };
            let id = file_stem(&file);
            if self.template_object(&id).is_some_and(|object| object.class == STRING_DATA) {
                self.read_ref_names(&text);
            } else if self.template_object(&id).is_none() {
                let (class, label) = (class_attribute(&text).to_owned(), format!("ref: {code}"));
                self.refs.insert(id, RefObject { class, label, text });
            }
        }
    }

    /// The names in `patch` (of the names object) that the template does not have.
    fn read_ref_names(&mut self, patch: &str) {
        let Some(id) = self.object_of_class(STRING_DATA) else { return };
        let template = self.template_object(id).map(|object| object.text.clone()).unwrap_or_default();
        let text = resolve(patch, View::Mod);
        let new_names = |field: &str| {
            let known = nemesis_diff::vars::names(&template, field);
            let names = nemesis_diff::vars::names(&text, field);
            names.into_iter().filter(|name| !known.contains(name)).collect::<Vec<String>>()
        };
        self.ref_events.extend(new_names("eventNames"));
        self.ref_variables.extend(new_names("variableNames"));
    }

    /// One object with its current text, if it exists.
    pub(crate) fn object_text(&self, id: &str) -> Option<ObjectText<'_>> {
        if let Some((id, text)) = self.added.get_key_value(id) {
            return Some(ObjectText {
                id,
                class: class_attribute(text),
                name: "",
                text,
                is_new: true,
                is_edited: false,
            });
        }
        let Some(object) = self.template_object(id) else {
            let (id, object) = self.refs.get_key_value(id)?;
            return Some(Self::ref_text(id, object));
        };
        let edit = self.edits.get(id);
        Some(ObjectText {
            id: &object.id,
            class: &object.class,
            name: &object.name,
            text: edit.unwrap_or(&object.text),
            is_new: false,
            is_edited: edit.is_some(),
        })
    }

    /// `object` of a referenced mod as the lists see it: like an object of the template.
    fn ref_text<'a>(id: &'a str, object: &'a RefObject) -> ObjectText<'a> {
        ObjectText {
            id,
            class: &object.class,
            name: &object.label,
            text: &object.text,
            is_new: false,
            is_edited: false,
        }
    }

    /// Every object with its current text: the new ones first, then the template's.
    pub(crate) fn texts(&self) -> impl Iterator<Item = ObjectText<'_>> {
        let new = self.added.iter().map(|(id, text)| ObjectText {
            id,
            class: class_attribute(text),
            name: "",
            text,
            is_new: true,
            is_edited: false,
        });
        let template = self.objects.iter().map(|o| ObjectText {
            id: &o.id,
            class: &o.class,
            name: &o.name,
            text: self.edits.get(&o.id).unwrap_or(&o.text),
            is_new: false,
            is_edited: self.edits.contains_key(&o.id),
        });
        let refs = self.refs.iter().filter(|(id, _)| !self.added.contains_key(*id));
        new.chain(template).chain(refs.map(|(id, object)| Self::ref_text(id, object)))
    }

    pub(crate) fn template_object(&self, id: &str) -> Option<&TemplateObject> {
        self.object_index.get(id).map(|&index| &self.objects[index as usize])
    }

    /// The current text of `id`: its edit, a new object, or the template object.
    pub(crate) fn text(&self, id: &str) -> Option<&str> {
        self.edits
            .get(id)
            .or_else(|| self.added.get(id))
            .map(String::as_str)
            .or_else(|| self.template_object(id).map(|o| o.text.as_str()))
            .or_else(|| self.refs.get(id).map(|object| object.text.as_str()))
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
            // Animation data has lines, not XML: nothing to normalize.
            return Some(if self.is_anim_data() { text.clone() } else { normalize(text) });
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
            Some((from, text)) => text.replacen(&format!("name=\"{from}\""), &format!("name=\"{id}\""), 1),
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
    TemplateObject { id: object.id.to_string(), class: object.class.to_string(), name, text: object.text.to_string() }
}

/// See [`WorkspaceFile`].
const WORKSPACE_FILE: &str = "workspace.json";

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

/// A behavior of a cache dir: `<dir>/<behavior>/template.path` and its objects. `None`: no behavior.
fn read_cached_behavior(entry: &Path) -> Result<Option<Behavior>, String> {
    let Ok(template) = std::fs::read_to_string(entry.join("template.path")) else {
        return Ok(None);
    };
    let mut behavior = Behavior::open(Path::new(template.trim()))?;
    behavior.force_ext = entry.join("force_ext").exists();
    for file in read_dir(entry)?.filter(|p| p.extension().is_some_and(|e| e == "xml")) {
        let id = file_stem(&file).replace("%2F", "/");
        let text = read_text(&file)?;
        if behavior.template_object(&id).is_some() {
            behavior.edits.insert(id, text);
        } else {
            behavior.added.insert(id, text);
        }
    }
    Ok(Some(behavior))
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
    ref_mods: Vec<String>,
    session: Session,
    owned: std::collections::BTreeSet<String>,
}

/// The patch directories of the mod at `selected`: all its codes, if it is a mod's root, else itself.
///
/// e.g. `mods/a` containing `Nemesis_Engine/mod/amco` -> that; `.../mod/amco` -> the same.
fn patch_dirs_of(selected: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for engine in ENGINES {
        if let Ok(codes) = read_dir(&selected.join(engine).join("mod")) {
            dirs.extend(codes.filter(|path| path.is_dir()));
        }
    }
    if dirs.is_empty() { vec![selected.to_path_buf()] } else { dirs }
}

impl Workspace {
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
    pub(crate) fn load(root: &Path, code: &str) -> Result<Self, String> {
        let dir = Self::cache_dir(root, code);
        let mut workspace = Self::default();
        if !dir.is_dir() {
            return Ok(workspace);
        }
        workspace.info = std::fs::read_to_string(dir.join("info.ini")).map(|s| parse_info(&s)).unwrap_or_default();

        // Each one reads and indexes a whole template: at the same time, in the order of the directory.
        let entries: Vec<PathBuf> = read_dir(&dir)?.filter(|p| p.is_dir()).collect();
        let behaviors: Vec<Option<Behavior>> =
            entries.par_iter().map(|entry| read_cached_behavior(entry)).collect::<Result<_, String>>()?;
        workspace.behaviors = behaviors.into_iter().flatten().collect();
        // A work without the file, from before it existed, has nothing to restore.
        let file = std::fs::read_to_string(dir.join(WORKSPACE_FILE)).unwrap_or_default();
        let file: WorkspaceFile = sonic_rs::from_str(&file).unwrap_or_default();
        workspace.ref_mods = file.ref_mods;
        workspace.session = file.session;
        workspace.owned = file.owned;
        workspace.reload_refs();
        let hashes = workspace.files_to_save()?.iter().map(|(path, text)| (path.clone(), text_hash(text))).collect();
        workspace.saved = Some(SavedFiles { dir, hashes });
        Ok(workspace)
    }

    /// What the cache dir holds: file (relative to it) -> contents. See [`Self::save`].
    fn files_to_save(&self) -> Result<BTreeMap<PathBuf, Cow<'_, str>>, String> {
        let mut files = BTreeMap::new();
        files.insert(PathBuf::from("info.ini"), Cow::Owned(info_ini(&self.info)));
        let file =
            WorkspaceFile { ref_mods: self.ref_mods.clone(), session: self.session.clone(), owned: self.owned.clone() };
        let file = sonic_rs::to_string_pretty(&file).map_err(|e| e.to_string())?;
        files.insert(PathBuf::from(WORKSPACE_FILE), Cow::Owned(file));
        for behavior in &self.behaviors {
            let behavior_dir = PathBuf::from(behavior.cache_name());
            let template = behavior.template_path.display().to_string();
            files.insert(behavior_dir.join("template.path"), Cow::Owned(template));
            if behavior.force_ext {
                files.insert(behavior_dir.join("force_ext"), Cow::Borrowed(""));
            }
            for (id, text) in behavior.edits.iter().chain(&behavior.added) {
                files.insert(behavior_dir.join(cache_file(id)), Cow::Borrowed(text.as_str()));
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
    fn read_patch_dir(
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
    fn own_anim_files(&mut self, dir: &Path, root: Option<&Path>) {
        let Some(root) = root else { return };
        let Ok(projects) = read_dir(dir) else { return };
        for project in projects.filter(|path| path.is_dir()) {
            let Ok(files) = read_dir(&project) else { continue };
            let files = files.filter(|path| path.is_file());
            self.owned.extend(files.filter_map(|file| file.strip_prefix(root).ok().map(owned_key)));
        }
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

    /// The mod codes this mod depends on: what it said when it was read, and the mods it refers to.
    ///
    /// e.g. a reference `D:/mods/a/Nemesis_Engine/mod/amco` -> `amco`
    fn deps(&self, own_code: &str) -> Vec<String> {
        let refs = self.ref_mods.iter().filter_map(|dir| Path::new(dir).file_name()?.to_str());
        let all: std::collections::BTreeSet<&str> =
            self.info.deps.iter().map(String::as_str).chain(refs).filter(|code| *code != own_code).collect();
        all.into_iter().map(str::to_owned).collect()
    }

    /// `info.ini` of `Nemesis_EngineExt`: `info`, with what only that one has. What is not known is left
    /// out, so that a mod without any of it reads like Nemesis'.
    fn ext_info(&self, info: &str, own_code: &str) -> String {
        let mut out = info.to_owned();
        if !self.info.version.trim().is_empty() {
            out.push_str(&format!("\nversion={}", self.info.version.trim()));
        }
        let deps = self.deps(own_code);
        if !deps.is_empty() {
            out.push_str(&format!("\ndeps={}", deps.join(",")));
        }
        out
    }

    /// Writes the mod under `<output>`: each behavior to its [`Behavior::patch_dir`], `info.ini` to
    /// each mod root used. Returns the files written and the patches d-merge's Nemesis parser rejects
    /// (`behavior/#id: error`).
    ///
    /// By default `<output>` is the mod it was read from, so only [`Self::owned`] is replaced: what
    /// this work did not read stays (a readme, the patches of a template that is not installed).
    pub(crate) fn export(
        &mut self,
        output: &Path,
        code: &str,
        mode: ParserMode,
    ) -> Result<(usize, Vec<String>), String> {
        for file in &self.owned {
            remove_owned(output, file)?;
        }
        self.owned.clear();
        let info = self.info.exported();

        let (mut written, mut invalid) = (0, Vec::new());
        for behavior in &self.behaviors {
            let relative = behavior.patch_dir(code);
            for id in behavior.edits.keys().chain(behavior.added.keys()) {
                let Some(patch) = behavior.patch(code, id) else { continue };
                if let Err(err) = behavior.validate(id, &patch, mode) {
                    invalid.push(format!("{}/{id}: {err}", behavior.name));
                }
                let file = Path::new(&relative).join(behavior.patch_file(id));
                write(&output.join(&file), &patch)?;
                self.owned.insert(owned_key(&file));
                written += 1;
            }
        }
        let roots = ENGINES.map(|engine| output.join(engine).join("mod").join(code));
        let [nemesis, ext] = roots;
        if nemesis.is_dir() {
            write(&nemesis.join("info.ini"), &info)?;
        }
        if ext.is_dir() {
            write(&ext.join("info.ini"), &self.ext_info(&info, code))?;
        }
        Ok((written, invalid))
    }
}

/// e.g. `Nemesis_Engine\\mod\\para\\0_master\\#0106.txt` -> `Nemesis_Engine/mod/para/0_master/#0106.txt`
fn owned_key(relative: &Path) -> String {
    relative.to_string_lossy().replace('\\', "/")
}

/// Removes a file of [`Workspace::owned`], and the directories this leaves empty.
fn remove_owned(output: &Path, file: &str) -> Result<(), String> {
    let path = output.join(file);
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(format!("{}: {err}", path.display())),
    }
    let mut dir = path.parent();
    while let Some(parent) = dir.filter(|parent| *parent != output) {
        if std::fs::remove_dir(parent).is_err() {
            break; // Not empty, or not there.
        }
        dir = parent.parent();
    }
    Ok(())
}

/// The template of animation data, `.bin` or `.txt`. (`.bin` if neither exists: clip data needs none.)
fn anim_template_path(template_root: &Path, stem: &str) -> PathBuf {
    let meshes = template_root.join("meshes");
    let path = |ext: &str| meshes.join(format!("{stem}.{ext}"));
    if path("bin").is_file() || !path("txt").is_file() { path("bin") } else { path("txt") }
}

/// `<mod>/animationdatasinglefile/<project>/<file>.txt` -> the clip data "behavior" (or `None`).
/// Files are kept as they are; see [`Behavior::adsf`].
fn import_adsf(mod_dir: &Path, template_root: &Path) -> Result<Option<Behavior>, String> {
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
fn import_asdsf(mod_dir: &Path, template_root: &Path, skipped: &mut Vec<String>) -> Result<Option<Behavior>, String> {
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
    found.into_values().collect()
}

fn meshes_path(template: &Path) -> String {
    let parts: Vec<String> = template.components().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).collect();
    let from = parts.iter().rposition(|p| p == "meshes").unwrap_or_else(|| parts.len().saturating_sub(1));
    let joined = parts[from..].join("/");
    Path::new(&joined).with_extension("bin").to_string_lossy().replace('\\', "/")
}

/// The Nemesis folder `(name, 1st person)` of a template, if Nemesis supports it.
pub(super) fn nemesis_folder(meshes_path: &str) -> Option<(String, bool)> {
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
    let deps = info.deps.join(",");
    let mut out = format!(
        "name={}\nauthor={}\nsite={}\nversion={}\ndeps={deps}\n",
        info.name, info.author, info.site, info.version
    );
    for line in &info.other {
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn parse_info(ini: &str) -> ModInfo {
    let mut info = ModInfo::default();
    for line in ini.lines().filter(|line| !line.trim().is_empty()) {
        let Some((key, value)) = line.split_once('=') else {
            info.other.push(line.to_owned());
            continue;
        };
        let value = value.trim().to_string();
        match key.trim() {
            "name" => info.name = value,
            "author" => info.author = value,
            "site" => info.site = value,
            "version" | "deps" if value.eq_ignore_ascii_case("null") => {}
            "version" => info.version = value,
            "deps" => {
                let codes = value.split(',').map(str::trim).filter(|code| !code.is_empty());
                info.deps = codes.map(str::to_owned).collect();
            }
            _ => info.other.push(line.to_owned()),
        }
    }
    info
}

fn read_dir(dir: &Path) -> Result<impl Iterator<Item = PathBuf>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(entries.flatten().map(|e| e.path()))
}

/// UTF-8 (lossy) without `\r`.
pub(super) fn read_text(path: &Path) -> Result<String, String> {
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
            behavior.patch("upatch", "#amco$0"),
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
