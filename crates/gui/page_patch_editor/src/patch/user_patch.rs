//! The user patch: what the conflict tool decided. A mod of its own (`upatch`) that loads last.
//!
//! `Use this` in the conflict window decides at once: the value goes in, and the mod is written, so
//! that the next merge has it. So this works on the user patch whatever the editor has open: it is
//! read from its cache, changed, and saved, leaving another mod being edited alone. If the editor has
//! the user patch open, that is the one changed.
//!
//! The window that shows all of it is [`super::ui::user_patch`].

use std::path::{Path, PathBuf};

use gui_shared::{
    patch::{ResolveRequest, USER_PATCH_CODE, USER_PATCH_OUTPUT},
    settings::pages::patch::behavior::ParserMode,
};

use super::{
    PatchTool,
    preview::Preview,
    resolve,
    workspace::{CACHE_ROOT, Workspace},
};

/// One changed object of the user patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Row {
    /// e.g. `0_master`
    pub(super) behavior: String,
    pub(super) id: String,
    /// e.g. `#0106 hkbModifierList GetUpModList`
    pub(super) label: String,
    /// The object's patch file.
    pub(super) patch: String,
}

/// Every object the user patch changes or adds, by behavior.
pub(super) fn rows(workspace: &Workspace) -> Vec<Row> {
    let mut rows = Vec::new();
    for behavior in &workspace.behaviors {
        for id in behavior.edits.keys().chain(behavior.added.keys()) {
            let Some(patch) = behavior.patch(USER_PATCH_CODE, id) else { continue };
            let (class, name) = behavior.object_text(id).map_or(("", ""), |object| (object.class, object.name));
            let label = format!("{id} {class} {name}").trim_end().to_owned();
            rows.push(Row { behavior: behavior.name.clone(), id: id.clone(), label, patch });
        }
    }
    rows
}

/// Keeps the user patch, and writes the mod. Saved again after: what the export wrote is part of the
/// work. Returns the patches d-merge's parser rejects.
fn save_and_export(workspace: &mut Workspace, mode: ParserMode) -> Result<Vec<String>, String> {
    let cache = Path::new(CACHE_ROOT);
    workspace.save(cache, USER_PATCH_CODE)?;
    let invalid = workspace.export(Path::new(USER_PATCH_OUTPUT), USER_PATCH_CODE, mode)?.1;
    workspace.save(cache, USER_PATCH_CODE)?;
    Ok(invalid)
}

/// The window's state. See the module docs.
#[derive(Debug, Default)]
pub(super) struct UserPatchView {
    pub(super) open: bool,
    pub(super) rows: Vec<Row>,
    /// The object shown: `(behavior, id)`.
    pub(super) selected: Option<(String, String)>,
    pub(super) preview: Option<Preview>,
    /// [`PatchTool::edit_revision`] of `rows`: an edit in the editor shows here.
    pub(super) seen_revision: u64,
}

impl PatchTool {
    /// Opens or closes the window of the user patch.
    pub(crate) fn toggle_user_patch(&mut self) {
        self.user_patch.open = !self.user_patch.open;
        if self.user_patch.open {
            self.refresh_user_patch();
        }
    }

    /// Does the editor have the user patch open?
    fn has_user_patch_open(&self) -> bool {
        self.loaded_for.as_deref() == Some(USER_PATCH_CODE)
    }

    /// Changes the user patch, saves it, and writes the mod. See the module docs.
    fn change_user_patch(&mut self, change: impl FnOnce(&mut Workspace) -> Result<(), String>) -> Result<(), String> {
        let invalid = if self.has_user_patch_open() {
            change(&mut self.workspace)?;
            self.reset_names();
            self.editor_for = None;
            self.preview = None;
            self.dirty_since = None;
            save_and_export(&mut self.workspace, self.parser_mode)?
        } else {
            let mut workspace = Workspace::load(Path::new(CACHE_ROOT), USER_PATCH_CODE)?;
            change(&mut workspace)?;
            save_and_export(&mut workspace, self.parser_mode)?
        };
        self.mod_written = true;
        if self.user_patch.open {
            self.refresh_user_patch();
        }
        if invalid.is_empty() { Ok(()) } else { Err(invalid.join("\n")) }
    }

    /// `Use this`: the value goes in now, with no editor involved.
    ///
    /// `name`: the mod's name, if it has none yet.
    pub(crate) fn resolve_now(
        &mut self,
        request: &ResolveRequest,
        template_dir: &str,
        name: &str,
    ) -> Result<(), String> {
        let root = PathBuf::from(template_dir);
        self.change_user_patch(|workspace| {
            if workspace.info.name.is_empty() {
                name.clone_into(&mut workspace.info.name);
            }
            // What the value points to is that mod's.
            workspace.add_ref(Path::new(&request.mod_dir), USER_PATCH_CODE);
            resolve::apply(workspace, &root, request)?;
            workspace.reload_refs(); // a behavior may have been opened
            Ok(())
        })
    }

    /// Reads what the window shows again.
    pub(super) fn refresh_user_patch(&mut self) {
        self.user_patch.rows = if self.has_user_patch_open() {
            rows(&self.workspace)
        } else {
            Workspace::load(Path::new(CACHE_ROOT), USER_PATCH_CODE)
                .map_or_else(|_| Vec::new(), |workspace| rows(&workspace))
        };
        self.user_patch.seen_revision = self.edit_revision;
        self.show_user_patch_object(self.user_patch.selected.clone());
    }

    /// Shows this object in the window, if the user patch has it.
    pub(super) fn show_user_patch_object(&mut self, object: Option<(String, String)>) {
        let view = &mut self.user_patch;
        let row = object
            .as_ref()
            .and_then(|(behavior, id)| view.rows.iter().find(|row| row.behavior == *behavior && row.id == *id));
        view.preview = row.map(|row| Preview::new(&row.patch));
        view.selected = row.and(object);
    }

    /// Puts changed block `block` of the object back to the template.
    pub(super) fn revert_user_patch_block(&mut self, behavior: &str, id: &str, block: usize) -> Result<(), String> {
        self.change_user_patch(|workspace| {
            let Some(behavior) = workspace.behaviors.iter_mut().find(|found| found.name == behavior) else {
                return Ok(());
            };
            let Some(template) = behavior.template_object(id) else { return Ok(()) };
            let Some(text) = behavior.text(id) else { return Ok(()) };
            let reverted = nemesis_diff::revert_hunk(&template.text, text, block);
            behavior.set_text(id, reverted);
            Ok(())
        })
    }

    /// Takes the object out of the user patch.
    pub(super) fn remove_user_patch_object(&mut self, behavior: &str, id: &str) -> Result<(), String> {
        self.change_user_patch(|workspace| {
            if let Some(behavior) = workspace.behaviors.iter_mut().find(|found| found.name == behavior) {
                behavior.edits.remove(id);
                behavior.added.remove(id);
            }
            Ok(())
        })
    }

    /// Opens the object in the editor: the editor goes to the user patch.
    pub(super) fn open_user_patch_object(&mut self, behavior: String, id: String) {
        self.pending_user_patch = true;
        self.pending_open = Some((behavior, id));
    }
}

#[cfg(test)]
mod tests {
    use super::{super::workspace::Behavior, *};

    #[test]
    fn rows_are_what_the_user_patch_writes() {
        assert!(rows(&Workspace::default()).is_empty());

        let path = Path::new("meshes").join("animationdatasinglefile.bin");
        let mut behavior = Behavior::open(&path).unwrap();
        behavior.added.insert("D~1/Idle~348.txt".to_owned(), "Idle\n348\n1\n0\n0\n0\n".to_owned());
        let workspace = Workspace { behaviors: vec![behavior], ..Workspace::default() };

        let found = rows(&workspace);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].behavior, "animationdatasinglefile");
        assert_eq!(found[0].id, "D~1/Idle~348.txt");
        assert_eq!(found[0].patch, "Idle\n348\n1\n0\n0\n0\n");
    }
}
