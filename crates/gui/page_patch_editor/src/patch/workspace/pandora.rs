//! Writing the work as a Pandora mod: `<output>/Pandora_Engine/mod/<code>/`.
//!
//! The work is the same as for Nemesis: objects of templates, edited or new. What differs is how they
//! are said: operations with paths in one file per template, see [`pandora_patch`].

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use fast_hash::HashMap;
use nemesis_diff::open_kept_arrays;
use pandora_patch::{
    Op,
    diff::new_object,
    doc::{apply_doc, replace_object_ops},
    info_xml, object_ops, patch_file,
};
use skyrim_anim_data::projects::{pack_file_name, pack_file_names};

use super::{
    ADSF, ASDSF, Behavior, ExportFormat, PANDORA_DOC, Workspace,
    locations::{file_stem, joined, meshes_path},
    read_dir, read_text, write,
};

/// What [`Workspace::export_pandora`] did.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct PandoraExport {
    /// The patch files written.
    pub(crate) written: usize,
    /// What could not be written, and why. e.g. `0_master/#0106: not well formed`
    pub(crate) problems: Vec<String>,
}

/// The text of one template's patch file. `None`: this work changes nothing of it.
pub(super) fn behavior_patch(behavior: &Behavior, code: &str, problems: &mut Vec<String>) -> Option<String> {
    // Borrowed by the operations: they are all made before the first of them.
    let edits: Vec<(&str, &str, Cow<'_, str>)> = behavior
        .edits
        .iter()
        .filter_map(|(id, edited)| {
            let template = behavior.template_object(id)?;
            Some((id.as_str(), template.text.as_str(), open_kept_arrays(edited)))
        })
        .collect();
    let added: Vec<(&str, Cow<'_, str>)> =
        behavior.added.iter().map(|(id, text)| (id.as_str(), open_kept_arrays(text))).collect();

    let mut problem = |id: &str, why: &dyn std::fmt::Display| problems.push(format!("{}/{id}: {why}", behavior.name));
    let mut ops: Vec<Op<'_>> = Vec::new();
    for (id, template, edited) in &edits {
        match object_ops(template, edited) {
            Ok(of_object) => ops.extend(of_object),
            Err(err) => problem(id, &err),
        }
    }
    let mut new_objects = Vec::with_capacity(added.len());
    for (id, text) in &added {
        match new_object(text) {
            Ok(object) => new_objects.push(object),
            Err(err) => problem(id, &err),
        }
    }
    (!ops.is_empty() || !new_objects.is_empty()).then(|| patch_file(code, &new_objects, &ops))
}

impl Behavior {
    /// Its patch file in a Pandora mod: `<output>/Pandora_Engine/mod/<code>/patches/<pack file>.xml`.
    /// `None`: no project has this template, so Pandora has no name for it.
    ///
    /// e.g. `D:/mods/My/Pandora_Engine/mod/mymod/patches/defaultmale~0_master.xml`
    pub(crate) fn pandora_patch_path(&self, output: &Path, code: &str) -> Option<PathBuf> {
        let name = self.pandora_file.clone().or_else(|| pack_file_name(&self.meshes_path))?;
        Some(joined(output, &["Pandora_Engine", "mod", code, "patches", &format!("{name}.xml")]))
    }

    /// Takes in the Pandora patch file: its operations on the template become the edits of the objects,
    /// so that they show as any others do. Returns the objects that changed and the paths that lead
    /// nowhere. A file that isn't well formed changes nothing: the editor says what is wrong with it.
    pub(crate) fn apply_pandora_doc(&mut self) -> (Vec<String>, Vec<String>) {
        let Some(doc) = &self.pandora_doc else { return Default::default() };
        let applied = apply_doc(doc, |id| self.template_object(id).map(|object| object.text.as_str()));
        let Some(applied) = applied else { return Default::default() };

        let mut changed = std::mem::take(&mut self.pandora_doc_ids);
        for id in &changed {
            self.edits.remove(id);
            self.added.remove(id);
        }
        for (id, text) in applied.edited {
            self.pandora_doc_ids.push(id.clone());
            self.edits.insert(id, text);
        }
        for (id, text) in applied.added {
            self.pandora_doc_ids.push(id.clone());
            self.added.insert(id, text);
        }
        changed.extend(self.pandora_doc_ids.iter().cloned());
        changed.sort_unstable();
        changed.dedup();
        (changed, applied.failed)
    }

    /// An object was edited: its part of the Pandora patch file is written anew, and only that.
    /// A file that isn't well formed stays as it is, for its author to fix: it can't be split in parts.
    pub(crate) fn update_pandora_doc(&mut self, id: &str) {
        let Some(doc) = &self.pandora_doc else { return };
        let updated = if let Some(text) = self.added.get(id) {
            let text = open_kept_arrays(text);
            pandora_patch::tree::parse(&text).and_then(|object| replace_object_ops(doc, id, &[], Some(&object)))
        } else if let (Some(template), Some(text)) = (self.template_object(id), self.edits.get(id)) {
            let text = open_kept_arrays(text);
            object_ops(&template.text, &text).ok().and_then(|ops| replace_object_ops(doc, id, &ops, None))
        } else {
            replace_object_ops(doc, id, &[], None)
        };
        if let Some(updated) = updated {
            self.pandora_doc = Some(updated);
            if !self.pandora_doc_ids.iter().any(|known| known == id) {
                self.pandora_doc_ids.push(id.to_owned());
            }
        }
    }
}

/// The first text of a behavior's Pandora patch file: what the work has changed so far.
pub(crate) fn draft_doc(behavior: &Behavior, code: &str) -> String {
    let mut problems = Vec::new();
    let patch = behavior_patch(behavior, code, &mut problems);
    let body = patch.unwrap_or_else(|| patch_file(code, &[], &[]));
    let comment = crate::patch::editor::pandora::draft_comment(&problems);
    if comment.is_empty() {
        return body;
    }
    // After the declaration, which has to come first in a file.
    let (declaration, rest) = body.split_once('\n').unwrap_or((&body, ""));
    format!("{declaration}\n{comment}{rest}")
}

/// Pack file name -> the template, for what is under `template_root/meshes`.
///
/// A name without a project, e.g. `magicbehavior`, is taken for the third person's, which is what the
/// mods with such a name patch: `_1stperson` comes last.
fn templates_by_pack_file(template_root: &Path) -> Result<HashMap<String, (PathBuf, String)>, String> {
    let mut found = Vec::new();
    let mut folders = vec![template_root.join("meshes")];
    while let Some(folder) = folders.pop() {
        for path in read_dir(&folder)? {
            if path.is_dir() {
                folders.push(path);
            } else if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("bin") || ext.eq_ignore_ascii_case("xml"))
            {
                found.push(path);
            }
        }
    }
    found.sort_by_key(|path| (path.to_string_lossy().to_ascii_lowercase().contains("_1stperson"), path.clone()));

    let mut by_name = HashMap::default();
    for path in found {
        let meshes_path = meshes_path(&path);
        let names = pack_file_names(&meshes_path);
        let plain = names.first().and_then(|name| name.split_once('~')).map(|(_, stem)| stem.to_owned());
        for name in names.into_iter().chain(plain) {
            by_name.entry(name).or_insert_with(|| (path.clone(), meshes_path.clone()));
        }
    }
    Ok(by_name)
}

impl Workspace {
    /// Reads `<dir>/patches/*.xml` of a Pandora mod: each file is the patch file of one behavior, as it
    /// is. What has no template goes to `skipped`, as does what isn't in Pandora's patches: its
    /// animation data stays where it is, as an export doesn't touch it.
    ///
    /// # Errors
    /// If a file can't be read.
    pub(super) fn read_pandora_dir(
        &mut self,
        dir: &Path,
        template_root: &Path,
        skipped: &mut Vec<String>,
    ) -> Result<(), String> {
        let patches = dir.join("patches");
        if !patches.is_dir() {
            return Ok(());
        }
        let by_name = templates_by_pack_file(template_root)?;
        let mut files: Vec<PathBuf> = read_dir(&patches)?
            .filter(|path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("xml")))
            .collect();
        files.sort();
        for file in files {
            let name = file_stem(&file);
            let Some((template, meshes_path)) = by_name.get(&name.to_ascii_lowercase()) else {
                skipped.push(format!("patches/{name}.xml"));
                continue;
            };
            let at = self.open_behavior(template, meshes_path)?;
            let behavior = &mut self.behaviors[at];
            // Two names of one template in a mod: both are its, but a behavior has one file.
            if behavior.pandora_doc.is_some() {
                skipped.push(format!("patches/{name}.xml"));
                continue;
            }
            let text = read_text(&file)?;
            behavior.pandora_doc = Some(text.trim_start_matches('\u{feff}').to_owned());
            behavior.pandora_file = Some(name);
            let (_, failed) = behavior.apply_pandora_doc();
            skipped.extend(failed);
            self.export_format = ExportFormat::PandoraV5;
        }
        if dir.join(ASDSF).is_dir() {
            skipped.push(ASDSF.to_owned());
        }
        if dir.join(ADSF).is_dir() {
            skipped.push(ADSF.to_owned());
        }
        Ok(())
    }

    /// Writes the behaviors of the work as Pandora patches, and `info.xml`. Nothing else under `<output>`
    /// is touched, so a mod can have both Nemesis' and Pandora's.
    ///
    /// Anim data is not written: Pandora's format has less than what the work holds. It is told as a
    /// problem, so that it is not thought to be there.
    ///
    /// # Errors
    /// If a file can't be written.
    pub(crate) fn export_pandora(&self, output: &Path, code: &str) -> Result<PandoraExport, String> {
        let root = joined(output, &["Pandora_Engine", "mod", code]);
        let mut export = PandoraExport::default();
        for behavior in &self.behaviors {
            let has_changes = !behavior.edits.is_empty() || !behavior.added.is_empty();
            if behavior.is_anim_data() {
                if has_changes {
                    export
                        .problems
                        .push(format!("{}: not written: Pandora's format has no place for it", behavior.name));
                }
                continue;
            }
            // What was written by hand is what is meant.
            let hand_written = behavior.pandora_doc.as_ref().filter(|doc| {
                let well_formed = pandora_patch::tree::parse(doc).is_some();
                if !well_formed {
                    export.problems.push(format!("{}: {PANDORA_DOC}: not well formed", behavior.name));
                }
                well_formed
            });
            let text = match hand_written {
                Some(doc) => Some(doc.clone()),
                None if behavior.pandora_doc.is_some() => None,
                None => behavior_patch(behavior, code, &mut export.problems),
            };
            let Some(text) = text else { continue };
            let Some(file) = behavior.pandora_patch_path(output, code) else {
                export.problems.push(format!("{}: not written: no project has this template", behavior.name));
                continue;
            };
            write(&file, &text)?;
            export.written += 1;
        }
        if export.written > 0 {
            let info = &self.info;
            write(&root.join("info.xml"), &info_xml(code, &info.name, &info.author, &info.site))?;
        }
        Ok(export)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A work kept when this still counted as an edit: nothing is left of it in the changed objects.
    #[test]
    fn replacing_by_the_same_is_not_changed_also_from_an_old_work() {
        let dir = std::env::temp_dir().join(format!("d_merge_pandora_same_{}", std::process::id()));
        let mut master = behavior(&dir);
        let doc = "<patch>\n\t<Replace path=\"#0001/enable\">\n\t\t<hkparam name=\"enable\">true</hkparam>\n\t</Replace>\n</patch>\n";
        master.pandora_doc = Some(doc.to_owned());
        master.pandora_doc_ids = pandora_patch::doc::doc_object_ids(doc);
        master.edits.insert("#0001".to_owned(), "kept in the cache".to_owned());

        let (changed, failed) = master.apply_pandora_doc();
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(master.edits.is_empty());
        assert_eq!(changed, ["#0001"], "the list takes it out");
        assert!(failed.is_empty());
    }

    /// A template of one object, in a place that has a project.
    fn behavior(dir: &Path) -> Behavior {
        let meshes = dir.join("meshes/actors/character/behaviors");
        std::fs::create_dir_all(&meshes).unwrap();
        let path = meshes.join("0_master.xml");
        let template = "<hkpackfile><hksection name=\"__data__\">\n\
<hkobject name=\"#0001\" class=\"hkbStateMachine\" signature=\"0x1\">\n\
\t<hkparam name=\"enable\">true</hkparam>\n\
</hkobject>\n\
</hksection></hkpackfile>\n";
        std::fs::write(&path, template).unwrap();
        Behavior::open(&path).unwrap()
    }

    #[test]
    fn a_work_is_written_as_one_file_per_template() {
        let dir = std::env::temp_dir().join(format!("d_merge_pandora_export_{}", std::process::id()));
        let mut workspace = Workspace::default();
        let mut master = behavior(&dir);
        let edited = master.template_object("#0001").unwrap().text.replace(">true<", ">false<");
        master.edits.insert("#0001".to_owned(), edited);
        master.added.insert(
            "#mine$0".to_owned(),
            "<hkobject name=\"#mine$0\" class=\"hkbClipGenerator\" signature=\"0x2\"></hkobject>".to_owned(),
        );
        workspace.behaviors.push(master);
        workspace.info.name = "Mine".to_owned();

        let output = dir.join("out");
        let export = workspace.export_pandora(&output, "mine").unwrap();
        let root = output.join("Pandora_Engine/mod/mine");
        let file = std::fs::read_to_string(root.join("patches/defaultfemale~0_master.xml"));
        let info = std::fs::read_to_string(root.join("info.xml"));
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(export, PandoraExport { written: 1, problems: Vec::new() });
        let file = file.unwrap();
        assert!(file.contains("<Replace path=\"#0001/enable\">"), "{file}");
        assert!(file.contains("<hkparam name=\"enable\">false</hkparam>"), "{file}");
        assert!(file.contains("<Insert path=\"\">"), "{file}");
        assert!(info.unwrap().contains("<name>Mine</name>"));
    }

    #[test]
    fn a_file_written_by_hand_is_what_is_exported() {
        let dir = std::env::temp_dir().join(format!("d_merge_pandora_doc_{}", std::process::id()));
        let mut workspace = Workspace::default();
        let mut master = behavior(&dir);
        let draft = draft_doc(&master, "mine");
        assert!(draft.contains("<mine>"), "{draft}");
        master.set_text(PANDORA_DOC, "<mine><Remove path=\"#0001/enable\" /></mine>");
        assert_eq!(master.text(PANDORA_DOC), Some("<mine><Remove path=\"#0001/enable\" /></mine>"));
        assert!(master.edits.is_empty() && master.added.is_empty(), "it isn't an object");
        workspace.behaviors.push(master);

        let output = dir.join("out");
        let export = workspace.export_pandora(&output, "mine").unwrap();
        let file = std::fs::read_to_string(output.join("Pandora_Engine/mod/mine/patches/defaultfemale~0_master.xml"));

        workspace.behaviors[0].set_text(PANDORA_DOC, "<mine><broken></mine>");
        let broken = workspace.export_pandora(&dir.join("out2"), "mine").unwrap();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(export.written, 1);
        assert_eq!(file.unwrap(), "<mine><Remove path=\"#0001/enable\" /></mine>");
        assert_eq!(broken.written, 0);
        assert!(broken.problems[0].contains("not well formed"), "{:?}", broken.problems);
    }

    #[test]
    fn a_pandora_mod_is_read_and_written_back_under_its_names() {
        let dir = std::env::temp_dir().join(format!("d_merge_pandora_read_{}", std::process::id()));
        let _ = behavior(&dir); // the template
        let root = dir.join("mods/Mine");
        let mod_dir = root.join("Pandora_Engine/mod/nemesis");
        std::fs::create_dir_all(mod_dir.join("patches")).unwrap();
        std::fs::create_dir_all(mod_dir.join(ASDSF)).unwrap();
        std::fs::write(
            mod_dir.join("info.xml"),
            "<mod code=\"nemesis\"><name>My Mod</name><author>me</author><site>null</site></mod>",
        )
        .unwrap();
        // With a BOM, and under the other name of the template.
        let doc = "<mine><Remove path=\"#0001/enable\" /></mine>";
        std::fs::write(mod_dir.join("patches/DefaultMale~0_master.xml"), format!("\u{feff}{doc}")).unwrap();
        std::fs::write(mod_dir.join("patches/nothing_of_this_name.xml"), "<mine></mine>").unwrap();

        let (code_of_root, code_of_dir) = (super::super::code_of(&root), super::super::code_of(&mod_dir));
        let found_root = super::super::mod_root_of(&mod_dir);
        let (workspace, code, skipped) = Workspace::import(&mod_dir, &dir).unwrap();
        let export = workspace.export_pandora(&root, "nemesis").unwrap();
        let written = std::fs::read_to_string(mod_dir.join("patches/DefaultMale~0_master.xml"));
        let extra = mod_dir.join("patches/defaultfemale~0_master.xml").exists();
        let info = workspace.info.clone();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!((code_of_root.as_str(), code_of_dir.as_str(), code.as_str()), ("nemesis", "nemesis", "nemesis"));
        assert_eq!(found_root.as_ref(), Some(&root));
        assert_eq!(workspace.behaviors.len(), 1);
        assert_eq!(workspace.behaviors[0].pandora_doc.as_deref(), Some(doc));
        assert_eq!(workspace.behaviors[0].pandora_file.as_deref(), Some("DefaultMale~0_master"));
        assert_eq!(info.name, "My Mod");
        assert_eq!(info.site, "");
        assert!(skipped.contains(&"patches/nothing_of_this_name.xml".to_owned()), "{skipped:?}");
        assert!(skipped.contains(&ASDSF.to_owned()), "{skipped:?}");
        assert_eq!(export.written, 1);
        assert_eq!(written.unwrap(), doc);
        assert!(!extra, "not under another name as well");
    }

    #[test]
    fn a_patch_file_shows_as_edits_and_each_side_follows_the_other() {
        let dir = std::env::temp_dir().join(format!("d_merge_pandora_sync_{}", std::process::id()));
        let mut master = behavior(&dir);
        master.pandora_doc = Some(
            "<m>\n\t<!-- mine -->\n\t<Replace path=\"#0001/enable\">\n\t\t<hkparam name=\"enable\">false</hkparam>\n\t</Replace>\n\t<Insert path=\"\">\n\t\t<hkobject name=\"#m$0\" class=\"c\"></hkobject>\n\t</Insert>\n</m>\n"
                .to_owned(),
        );
        master.pandora_file = Some("0_master".to_owned());

        // The file -> the objects
        let (changed, failed) = master.apply_pandora_doc();
        assert_eq!(changed, ["#0001", "#m$0"]);
        assert!(failed.is_empty(), "{failed:?}");
        assert!(master.edits["#0001"].contains(">false<"));
        assert!(master.added.contains_key("#m$0"));

        // An object -> its part of the file, the rest as it was
        let edited = master.edits["#0001"].replace(">false<", ">maybe<");
        master.set_text("#0001", &edited);
        master.update_pandora_doc("#0001");
        let doc = master.pandora_doc.clone().unwrap();
        assert!(doc.contains(">maybe<") && !doc.contains(">false<"), "{doc}");
        assert!(doc.contains("<!-- mine -->") && doc.contains("#m$0"), "{doc}");

        // An edit taken back: its operation goes.
        let template = master.template_object("#0001").unwrap().text.clone();
        master.set_text("#0001", &template);
        master.update_pandora_doc("#0001");
        assert!(!master.pandora_doc.as_ref().unwrap().contains("#0001/enable"));

        // The file again: what it no longer says is gone from the objects.
        master.set_text(PANDORA_DOC, "<m></m>");
        let (changed, _) = master.apply_pandora_doc();
        assert!(changed.contains(&"#m$0".to_owned()), "{changed:?}");
        assert!(master.added.is_empty());
        // What isn't well formed changes nothing.
        master.set_text(PANDORA_DOC, "<m><a></m>");
        assert_eq!(master.apply_pandora_doc(), (Vec::new(), Vec::new()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn what_can_not_be_said_is_told_and_the_rest_is_written() {
        let dir = std::env::temp_dir().join(format!("d_merge_pandora_problem_{}", std::process::id()));
        let mut workspace = Workspace::default();
        let mut master = behavior(&dir);
        let other_class = master.template_object("#0001").unwrap().text.replace("hkbStateMachine", "hkbClipGenerator");
        master.edits.insert("#0001".to_owned(), other_class);
        workspace.behaviors.push(master);

        let export = workspace.export_pandora(&dir.join("out"), "mine").unwrap();
        let wrote_info = dir.join("out/Pandora_Engine/mod/mine/info.xml").exists();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(export.written, 0);
        assert_eq!(export.problems.len(), 1);
        assert!(export.problems[0].starts_with("0_master/#0001: "), "{:?}", export.problems);
        assert!(!wrote_info);
    }
}
