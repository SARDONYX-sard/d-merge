//! The user patch: what the conflict tool decided. A mod of its own (`upatch`) that loads last.
//!
//! It has its own editor in its own window, which can be opened on any page, whatever the main editor
//! has open: [`PatchTool`] again, kept in [`UserPatchView`]. What it is written to is decided by the
//! patch page's output directory, as its conflicts are: `<output>/.d_merge/user_patch`.
//!
//! `Use this` in the conflict window decides at once: the value goes in, and the mod is written, so
//! that the next merge has it. So does an edit in the window, a moment later. There is no `Export`.
//!
//! The window is [`super::ui::user_patch`].

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use egui_widgets::notify::Notification;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    patch::{ResolveRequest, USER_PATCH_CODE, user_patch_output},
};

use super::{
    PatchPaths, PatchTool,
    editor::resolve,
    workspace::{self, Workspace},
};

/// What `Edit...` in a conflict proposed: the mod's value in the user patch, to change it from there.
///
/// Pressing it decides nothing. If the window is closed with the object as it was proposed, all is as if
/// it had not been pressed: the user patch has its old value, and nothing was written. An edit of the
/// object keeps it. Other objects edited meanwhile stay either way.
#[derive(Debug)]
pub(super) struct Draft {
    /// How many behaviors there were: a higher one was opened by this.
    behaviors: usize,
    /// Of each behavior: `edits`, `added`.
    objects: Vec<(BTreeMap<String, String>, BTreeMap<String, String>)>,
    ref_mods: Vec<String>,
    name: String,
    /// The object, and the text proposed.
    behavior: usize,
    id: String,
    proposal: String,
    /// The references with the proposal: if they are others now, the user changed them.
    proposed_refs: Vec<String>,
}

impl Draft {
    /// What is there before the proposal. Taken with [`Self::with_proposal`].
    pub(super) fn before(workspace: &Workspace) -> Self {
        let behaviors = &workspace.behaviors;
        Self {
            behaviors: behaviors.len(),
            objects: behaviors.iter().map(|behavior| (behavior.edits.clone(), behavior.added.clone())).collect(),
            ref_mods: workspace.ref_mods.clone(),
            name: workspace.info.name.clone(),
            behavior: 0,
            id: String::new(),
            proposal: String::new(),
            proposed_refs: Vec::new(),
        }
    }

    /// `behavior`, `id`: the object it went to.
    pub(super) fn with_proposal(mut self, workspace: &Workspace, behavior: usize, id: &str) -> Self {
        self.behavior = behavior;
        id.clone_into(&mut self.id);
        workspace.behaviors[behavior].text(id).unwrap_or_default().clone_into(&mut self.proposal);
        self.proposed_refs.clone_from(&workspace.ref_mods);
        self
    }

    fn is_edited(&self, workspace: &Workspace) -> bool {
        workspace.behaviors.get(self.behavior).and_then(|behavior| behavior.text(&self.id))
            != Some(self.proposal.as_str())
    }

    /// Back to what was there, for this object. Other objects may have been edited since.
    fn undo(self, workspace: &mut Workspace) {
        let Some(behavior) = workspace.behaviors.get_mut(self.behavior) else { return };
        let (edit, added) = self
            .objects
            .get(self.behavior)
            .map_or((None, None), |(edits, added)| (edits.get(&self.id).cloned(), added.get(&self.id).cloned()));
        match edit {
            Some(text) => behavior.edits.insert(self.id.clone(), text),
            None => behavior.edits.remove(&self.id),
        };
        match added {
            Some(text) => behavior.added.insert(self.id, text),
            None => behavior.added.remove(&self.id),
        };
        // Opened for this, and nothing else was done in it.
        if self.behavior >= self.behaviors && behavior.edits.is_empty() && behavior.added.is_empty() {
            workspace.behaviors.remove(self.behavior);
        }
        if workspace.ref_mods == self.proposed_refs {
            workspace.ref_mods = self.ref_mods;
            workspace.reload_refs();
        }
        workspace.info.name = self.name;
    }
}

/// The window's state. See the module docs.
#[derive(Debug, Default)]
pub(super) struct UserPatchView {
    pub(super) open: bool,
    /// Comes above what it was opened from, on the next frame.
    pub(super) to_top: bool,
    /// Its editor, from the first time it is needed.
    pub(super) tool: Option<Box<PatchTool>>,
}

impl PatchTool {
    fn for_user_patch(paths: PatchPaths<'_>) -> Self {
        let output = paths.patch_output_dir.trim();
        Self {
            is_user_patch: true,
            restored: true,
            patch_output: output.to_owned(),
            cache_root: workspace::cache_root_of(output),
            output: user_patch_output(output).display().to_string(),
            mod_code: USER_PATCH_CODE.to_owned(),
            template_dir: paths.template_dir.to_owned(),
            parser_mode: paths.parser_mode,
            behaviors_open: true,
            changed_only: true,
            ..Self::default()
        }
    }

    /// Opens or closes the window of the user patch.
    pub(crate) const fn toggle_user_patch(&mut self) {
        self.user_patch.open = !self.user_patch.open;
        self.user_patch.to_top = self.user_patch.open;
    }

    /// The user patch's editor, up to date. A new output directory has another user patch: what was
    /// open is saved where it belongs first.
    pub(super) fn user_patch_tool(
        &mut self,
        paths: PatchPaths<'_>,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) -> &mut Self {
        let output = paths.patch_output_dir.trim();
        if let Some(tool) = self.user_patch.tool.as_mut().filter(|tool| tool.patch_output != output) {
            tool.undo_unedited_draft();
            tool.save_now(notify, i18n);
            self.mod_written |= tool.mod_written;
            self.user_patch.tool = None;
        }
        let tool = self.user_patch.tool.get_or_insert_with(|| Box::new(Self::for_user_patch(paths)));
        tool.set_parser_mode(paths.parser_mode);
        if tool.template_dir != paths.template_dir {
            paths.template_dir.clone_into(&mut tool.template_dir);
        }
        tool.sync_workspace(notify, i18n);
        // Small, and what comes next uses it at once.
        tool.wait_job(notify, i18n);
        tool
    }

    /// Writes the user patch now. Saved again after: what the export wrote is part of the work.
    /// Returns the patches d-merge's parser rejects.
    pub(super) fn write_user_patch(&mut self) -> Result<Vec<String>, String> {
        self.workspace.save(&self.cache_root, USER_PATCH_CODE)?;
        let invalid = self.workspace.export(Path::new(&self.output), USER_PATCH_CODE, self.parser_mode)?.1;
        self.workspace.save(&self.cache_root, USER_PATCH_CODE)?;
        self.dirty_since = None;
        Ok(invalid)
    }

    /// `Use this`: the value goes in now, and the mod is written. The window need not be open.
    ///
    /// # Errors
    /// There is no output directory to keep it in, or the value could not be taken in.
    pub(crate) fn resolve_now(
        &mut self,
        request: &ResolveRequest,
        paths: PatchPaths<'_>,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) -> Result<(), String> {
        if paths.patch_output_dir.trim().is_empty() {
            return Err(i18n.t(I18nKey::NemesisNeedPatchOutput).to_owned());
        }
        let root = PathBuf::from(paths.template_dir);
        let tool = self.user_patch_tool(paths, notify, i18n);
        if tool.workspace.info.name.is_empty() {
            i18n.t(I18nKey::UserPatchTitle).clone_into(&mut tool.workspace.info.name);
        }
        // What the value points to is that mod's.
        tool.workspace.add_ref(Path::new(&request.mod_dir), USER_PATCH_CODE);
        resolve::apply(&mut tool.workspace, &root, request)?;
        tool.workspace.reload_refs(); // a behavior may have been opened
        tool.reset_names();
        tool.editor_for = None;
        tool.preview = None;
        let invalid = tool.write_user_patch()?;
        self.mod_written = true;
        if invalid.is_empty() { Ok(()) } else { Err(invalid.join("\n")) }
    }

    /// `Edit...`: the value goes in, and the window shows it.
    pub(crate) fn open_resolution(
        &mut self,
        request: ResolveRequest,
        paths: PatchPaths<'_>,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) {
        self.user_patch.open = true;
        self.user_patch.to_top = true;
        if paths.patch_output_dir.trim().is_empty() {
            return; // the window says what to do
        }
        self.user_patch_tool(paths, notify, i18n).pending_resolution = Some(request);
    }

    /// What `Edit...` proposed goes, unless it was edited. See [`Draft`].
    pub(super) fn undo_unedited_draft(&mut self) {
        self.merge_values = None;
        let Some(draft) = self.draft.take() else { return };
        if draft.is_edited(&self.workspace) {
            return;
        }
        let id = draft.id.clone();
        draft.undo(&mut self.workspace);
        self.reset_names();
        self.editor_for = None;
        self.preview = None;
        self.active = self.active.min(self.workspace.behaviors.len().saturating_sub(1));
        if self.workspace.behaviors.get(self.active).is_none_or(|behavior| behavior.text(&id).is_none()) {
            self.tabs.clear();
            self.selected = None;
        }
        self.session_seen = (self.active, self.selected.clone());
        // Something saved meanwhile (another object) had it in: the disk goes back too.
        if self.mod_written {
            self.dirty_since.get_or_insert_with(std::time::Instant::now);
        }
    }

    /// Takes everything out of the user patch, and writes that: nothing of it is merged any more.
    pub(super) fn clear_user_patch(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        self.draft = None;
        self.workspace.behaviors.clear();
        self.workspace.ref_mods.clear();
        self.reset_names();
        self.tabs.clear();
        self.active = 0;
        self.selected = None;
        self.editor_for = None;
        self.preview = None;
        match self.write_user_patch() {
            Ok(_) => self.mod_written = true,
            Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisExportFailed))),
        }
    }

    /// The window was closed: what it wrote is for the conflict check to see now, not at each pause.
    pub(super) fn close_user_patch(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let Some(tool) = &mut self.user_patch.tool else { return };
        tool.undo_unedited_draft();
        tool.save_now(notify, i18n);
        self.mod_written |= std::mem::take(&mut tool.mod_written);
    }
}

#[cfg(test)]
mod tests {
    use gui_shared::settings::{pages::patch_editor::PatchEditorSettings, ui::editor::LspOptions};

    use super::*;

    fn paths(output: &str) -> PatchPaths<'_> {
        PatchPaths {
            patch_output_dir: output,
            template_dir: "",
            parser_mode: gui_shared::settings::pages::patch::behavior::ParserMode::Strict,
        }
    }

    #[test]
    fn its_place_is_decided_by_the_patch_pages_output() {
        let tool = PatchTool::for_user_patch(paths("D:/out"));
        assert_eq!(Path::new(&tool.output), Path::new("D:/out/.d_merge/user_patch"));
        assert_eq!(tool.mod_code, USER_PATCH_CODE);
        assert!(tool.cache_root.starts_with("D:/out/.d_merge/caches"));
    }

    #[test]
    fn it_needs_the_output_directory() {
        let (mut notify, i18n) = (Notification::default(), I18nMap::new());
        let request = ResolveRequest {
            target: "meshes/x.bin".into(),
            path: "#1/C/a".into(),
            mod_dir: "D:/mods/a".into(),
            edit: false,
            sides: Vec::new(),
            keep_all_events: false,
        };
        let mut main = PatchTool::default();
        assert!(main.resolve_now(&request, paths(""), &mut notify, &i18n).is_err());
        assert!(main.user_patch.tool.is_none());
    }

    /// A proposal of `Edit...`: as a conflict's `Edit...` does, with a mod's clip of animation data.
    fn proposed(dir: &Path) -> (PatchTool, Notification, I18nMap) {
        let file = dir.join("mods/a/animationdatasinglefile/D~1/Idle~m$0.txt");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "theirs\n").unwrap();
        let output = dir.join("out").display().to_string();
        let (mut notify, i18n) = (Notification::default(), I18nMap::new());
        let mut main = PatchTool::default();
        let request = ResolveRequest {
            target: "meshes/animationdatasinglefile.bin".into(),
            path: "D~1/Idle~m$0/name".into(),
            mod_dir: dir.join("mods/a").display().to_string(),
            edit: true,
            sides: Vec::new(),
            keep_all_events: false,
        };
        main.open_resolution(request, paths(&output), &mut notify, &i18n);
        main.user_patch.tool.as_mut().unwrap().take_in_resolution(&mut notify, &i18n);
        (main, notify, i18n)
    }

    /// Opening it to look and closing it decides nothing.
    #[test]
    fn an_edit_that_was_not_made_changes_nothing() {
        let dir = std::env::temp_dir().join(format!("d_merge_draft_none_{}", std::process::id()));
        let (mut main, mut notify, i18n) = proposed(&dir);
        let tool = main.user_patch.tool.as_mut().unwrap();
        assert_eq!(tool.workspace.behaviors[0].text("D~1/Idle~m$0.txt"), Some("theirs\n"), "it shows the value");
        assert!(tool.dirty_since.is_none(), "nothing to save yet");

        main.close_user_patch(&mut notify, &i18n);
        let tool = main.user_patch.tool.as_ref().unwrap();
        let (behaviors, refs, name) =
            (tool.workspace.behaviors.len(), tool.workspace.ref_mods.len(), tool.workspace.info.name.clone());
        let written = dir.join("out").exists();
        let wrote = main.take_mod_written();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(behaviors, 0, "the behavior was opened for it");
        assert_eq!(refs, 0);
        assert_eq!(name, "");
        assert!(!written, "nothing is on the disk");
        assert!(!wrote, "the conflict check has nothing to follow");
    }

    #[test]
    fn an_edit_that_was_made_stays() {
        let dir = std::env::temp_dir().join(format!("d_merge_draft_edit_{}", std::process::id()));
        let (mut main, mut notify, i18n) = proposed(&dir);
        let tool = main.user_patch.tool.as_mut().unwrap();
        tool.workspace.behaviors[0].set_text("D~1/Idle~m$0.txt", "mine\n");
        tool.mark_dirty();

        main.close_user_patch(&mut notify, &i18n);
        let tool = main.user_patch.tool.as_ref().unwrap();
        let text = tool.workspace.behaviors[0].text("D~1/Idle~m$0.txt").map(str::to_owned);
        let wrote = main.take_mod_written();
        let exported =
            dir.join("out/.d_merge/user_patch/Nemesis_Engine/mod/upatch/animationdatasinglefile/D~1/Idle~m$0.txt");
        let on_disk = std::fs::read_to_string(exported).ok();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(text.as_deref(), Some("mine\n"));
        assert!(wrote);
        assert_eq!(on_disk.as_deref(), Some("mine\n"));
    }

    /// The old value of the user patch comes back, and what else was edited stays.
    #[test]
    fn the_old_value_comes_back() {
        let dir = std::env::temp_dir().join(format!("d_merge_draft_old_{}", std::process::id()));
        let mut workspace = Workspace::default();
        let adsf = Path::new("meshes").join("animationdatasinglefile.bin");
        workspace.behaviors.push(super::super::workspace::Behavior::open(&adsf).unwrap());
        workspace.behaviors[0].added.insert("D~1/Idle~m$0.txt".to_owned(), "old\n".to_owned());
        let before = Draft::before(&workspace);
        workspace.behaviors[0].added.insert("D~1/Idle~m$0.txt".to_owned(), "theirs\n".to_owned());
        let draft = before.with_proposal(&workspace, 0, "D~1/Idle~m$0.txt");
        workspace.behaviors[0].added.insert("D~1/Other.txt".to_owned(), "other\n".to_owned());

        assert!(!draft.is_edited(&workspace));
        draft.undo(&mut workspace);
        let _ = dir;
        assert_eq!(workspace.behaviors[0].text("D~1/Idle~m$0.txt"), Some("old\n"));
        assert_eq!(workspace.behaviors[0].text("D~1/Other.txt"), Some("other\n"));
    }

    /// Its window draws, on any page, and the main tool does not take its code.
    #[test]
    fn the_window_draws_and_clears() {
        let dir = std::env::temp_dir().join(format!("d_merge_user_patch_{}", std::process::id()));
        let output = dir.display().to_string();
        let (mut notify, i18n) = (Notification::default(), I18nMap::new());
        let mut main = PatchTool::default();
        let (mut settings, mut editor) = (PatchEditorSettings::default(), LspOptions::default());
        main.toggle_user_patch();

        let ctx = egui::Context::default();
        for _ in 0..3 {
            let mut drawn = ctx.run_ui(egui::RawInput::default(), |ui| {
                let ctx = ui.ctx().clone();
                main.user_patch_window(&ctx, &i18n, &mut notify, paths(&output), &mut settings, &mut editor);
            });
            drawn.textures_delta.clear();
        }
        let tool = main.user_patch.tool.as_mut().unwrap();
        assert_eq!(tool.loaded_for.as_deref(), Some(USER_PATCH_CODE));

        tool.clear_user_patch(&mut notify, &i18n);
        let cache = workspace::cache_root_of(&output).join(USER_PATCH_CODE);
        let kept = cache.is_dir();
        main.close_user_patch(&mut notify, &i18n);
        let written = main.take_mod_written();
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(kept);
        assert!(written, "the conflict check follows");
    }
}
