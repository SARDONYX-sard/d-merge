//! One template being patched: its objects, and what the mod changes of them.

use std::{
    borrow::Cow,
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use gui_shared::settings::pages::patch::behavior::ParserMode;
use nemesis_diff::{View, XmlObject, normalize, object_patch, objects, resolve};

use super::{
    locations::{file_name_of, file_stem, joined, meshes_path, nemesis_folder},
    read_dir, read_text,
};

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

/// The class of the object holding the event and variable names.
pub(crate) const STRING_DATA: &str = "hkbBehaviorGraphStringData";

/// `id` -> position, for [`Behavior::template_object`].
pub(super) fn index_objects(objects: &[TemplateObject]) -> fast_hash::HashMap<String, u32> {
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
pub(super) fn class_attribute(text: &str) -> &str {
    text.split_once("class=\"").and_then(|(_, rest)| rest.split('"').next()).unwrap_or_default()
}

/// File stem of the anim set data template (`meshes/animationsetdatasinglefile.bin|txt`).
pub(crate) const ASDSF: &str = "animationsetdatasinglefile";

/// File stem of the clip data template (`meshes/animationdatasinglefile.bin|txt`).
pub(crate) const ADSF: &str = "animationdatasinglefile";

/// d-merge's `animationsetdatasinglefile.bin` (MessagePack) -> the original txt.
pub(super) fn asdsf_bin_to_txt(bytes: &[u8]) -> Result<String, String> {
    use skyrim_anim_data::{
        asdsf::alt::{AltAsdsf, ser::serialize_alt_asdsf},
        diff_line::DiffLines,
    };
    let asdsf: AltAsdsf<'_> = rmp_serde::from_slice(bytes).map_err(|e| e.to_string())?;
    serialize_alt_asdsf(asdsf, DiffLines::DEFAULT, Default::default()).map_err(|e| e.to_string())
}

/// Cache file name of an id: `/` (anim set ids) can not be in a file name.
pub(super) fn cache_file(id: &str) -> String {
    format!("{}.xml", id.replace('/', "%2F"))
}

pub(super) fn index(object: XmlObject<'_>) -> TemplateObject {
    let name = object
        .text
        .split_once("<hkparam name=\"name\">")
        .and_then(|(_, rest)| rest.split_once("</hkparam>"))
        .map(|(name, _)| name.to_string())
        .unwrap_or_default();
    TemplateObject { id: object.id.to_string(), class: object.class.to_string(), name, text: object.text.to_string() }
}

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
            crate::patch::anim::patch_file::validate_adsf(id, patch, mode)
        } else if self.asdsf {
            crate::patch::anim::patch_file::validate_asdsf(id, patch)
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
        let nemesis = Path::new("Nemesis_Engine");
        if self.asdsf {
            return joined(nemesis, &["mod", code, ASDSF]);
        }
        if self.adsf {
            return joined(nemesis, &["mod", code, ADSF]);
        }
        match self.target() {
            Target::Nemesis { name, first_person: true } => joined(nemesis, &["mod", code, "_1stperson", &name]),
            Target::Nemesis { name, first_person: false } => joined(nemesis, &["mod", code, &name]),
            Target::Ext => joined(Path::new("Nemesis_EngineExt"), &["mod", code, &self.meshes_path]),
        }
    }

    /// Cache folder name: unique per template (`0_master` exists in 1st and 3rd person).
    pub(super) fn cache_name(&self) -> String {
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
    pub(super) fn patch_folders_in(&self, mod_dir: &Path) -> Vec<PathBuf> {
        let mut folders = vec![mod_dir.join(&self.meshes_path)];
        if let Some((name, first_person)) = &self.nemesis {
            let folder = if *first_person { joined(mod_dir, &["_1stperson", name]) } else { mod_dir.join(name) };
            folders.push(folder);
        }
        folders
    }

    /// Takes in what the referenced mod at `mod_dir` adds to this template: its new objects, and the
    /// events and variables in its patch of the names object. Its edits of template objects are
    /// not: those are its own business, and it is merged with this mod anyway.
    pub(super) fn read_ref(&mut self, mod_dir: &Path) {
        if self.is_anim_data() {
            return;
        }
        let code = file_name_of(mod_dir);
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
    pub(super) fn read_ref_names(&mut self, patch: &str) {
        let Some(id) = self.object_of_class(STRING_DATA) else { return };
        let template = self.template_object(id).map_or("", |object| &object.text);
        let text = resolve(patch, View::Mod);
        let new_names = |field: &str| {
            let known = nemesis_diff::vars::names(template, field);
            let names = nemesis_diff::vars::names(&text, field);
            names.into_iter().filter(|name| !known.contains(name)).map(str::to_owned).collect::<Vec<String>>()
        };
        let (events, variables) = (new_names("eventNames"), new_names("variableNames"));
        self.ref_events.extend(events);
        self.ref_variables.extend(variables);
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
    pub(super) fn ref_text<'a>(id: &'a str, object: &'a RefObject) -> ObjectText<'a> {
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
    pub(crate) fn set_text(&mut self, id: &str, text: &str) {
        if let Some(added) = self.added.get_mut(id) {
            text.clone_into(added);
        } else if self.template_object(id).is_some_and(|o| normalize(&o.text) == normalize(text)) {
            self.edits.remove(id);
        } else if let Some(edited) = self.edits.get_mut(id) {
            text.clone_into(edited);
        } else {
            self.edits.insert(id.to_owned(), text.to_owned());
        }
    }

    /// The `#XXXX.txt` contents of an edited object, or the new object's text.
    pub(crate) fn patch(&self, code: &str, id: &str) -> Option<Cow<'_, str>> {
        if let Some(text) = self.added.get(id) {
            // Animation data has lines, not XML: nothing to normalize.
            return Some(if self.is_anim_data() { Cow::Borrowed(text) } else { normalize(text) });
        }
        object_patch(code, &self.template_object(id)?.text, self.edits.get(id)?).map(Cow::Owned)
    }

    /// `#code$N` with the next free `N`.
    pub(super) fn next_id(&self, code: &str) -> String {
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
