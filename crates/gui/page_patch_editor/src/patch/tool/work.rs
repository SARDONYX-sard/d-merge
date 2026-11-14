//! Opening, saving and exporting a work.

use std::path::{Path, PathBuf};

use egui_widgets::notify::Notification;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::pages::patch_editor::RecentWork,
};
use nemesis_diff::is_existing_mod_code;

use super::job::{Done, Job, Kept};
use crate::patch::{
    FocusAt, PatchTool, editor, remember_recent,
    ui::code_choice::ModCodeChoice,
    user_patch, workspace,
    workspace::{Behavior, Workspace},
};

impl PatchTool {
    /// Goes to the work of `code`: [`Self::sync_workspace`] loads it, and what it saved of the output
    /// and the source wins over these. The current work is saved first, while these are still its own.
    pub(in crate::patch) fn switch_work(
        &mut self,
        code: &str,
        output: &str,
        source: &str,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) {
        self.save_now(notify, i18n);
        code.clone_into(&mut self.work.place.mod_code);
        output.clone_into(&mut self.work.place.output);
        source.clone_into(&mut self.work.place.mod_dir);
    }

    /// After the work is synced: takes the value in, and shows where it went. It is a proposal for now:
    /// see [`user_patch::Draft`]. So nothing of it is saved until there is an edit.
    pub(in crate::patch) fn take_in_resolution(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let Some(request) = self.pending_resolution.take() else { return };
        self.undo_unedited_draft();
        let before = user_patch::Draft::before(&self.work.workspace);
        let since = self.work.autosave.since();
        if self.work.workspace.info.name.is_empty() {
            i18n.t(I18nKey::UserPatchTitle).clone_into(&mut self.work.workspace.info.name);
        }
        // What the value points to is that mod's.
        self.work.workspace.add_ref(Path::new(&request.mod_dir), gui_shared::patch::USER_PATCH_CODE);
        // And what is taken from the others is theirs.
        for side in &request.sides {
            self.work.workspace.add_ref(Path::new(&side.mod_dir), gui_shared::patch::USER_PATCH_CODE);
        }
        let root = PathBuf::from(&self.work.place.template_dir);
        let shown = editor::resolve::apply(&mut self.work.workspace, &root, &request).map_or_else(
            |err| {
                notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisResolveFailed)));
                // It is for the user to edit: all the more if it could not be taken in.
                editor::resolve::object_of(&mut self.work.workspace, &root, &request)
            },
            Some,
        );
        let Some(editor::resolve::Resolved { behavior, id, line }) = shown else { return };
        self.work.workspace.reload_refs(); // a behavior may have been opened
        self.reset_names();
        self.sel.active = behavior;
        self.pane.opened = None;
        self.draft = Some(before.with_proposal(&self.work.workspace, behavior, &id));
        self.pane.merge_values = editor::resolve::merge_values(&request, &id);
        self.pane.merge_accepted.clear();
        self.reuse_resolution(behavior, &id, notify, i18n);
        self.sel.selected = Some(id.into());
        // What is shown is no change of the work yet.
        self.work.autosave.restore(since);
        self.work.autosave.seen_at(self.sel.active, self.sel.selected.as_ref());
        // Picking another value of the object shown has no other sign of having gone anywhere.
        self.pane.focus = line.map(FocusAt::Line);
    }

    /// The conflict just taken in was resolved before, by the same mods with the same values: it is
    /// resolved the same way at once. See [`editor::merge`].
    fn reuse_resolution(&mut self, behavior: usize, id: &str, notify: &mut Notification, i18n: &I18nMap) {
        let Some(values) = &self.pane.merge_values else { return };
        let Some(resolution) = editor::merge::find(&self.work.workspace.resolutions, values) else { return };
        let result = resolution.result.clone();
        let Some(text) = self.work.workspace.behaviors.get(behavior).and_then(|of| of.text(id)) else { return };
        let candidates = editor::merge::candidates(values, &[]);
        if let Some(resolved) = editor::merge::replace_block(text, &candidates, &result) {
            self.work.workspace.behaviors[behavior].set_text(id, &resolved);
            self.pane.merge_accepted = result;
            notify.info(i18n.t(I18nKey::MergeReused));
        }
    }

    /// The patch page's output directory may have changed: the works are others then. What is open is
    /// saved where it belongs first.
    pub(in crate::patch) fn set_patch_output(&mut self, output: &str, notify: &mut Notification, i18n: &I18nMap) {
        let output = output.trim();
        if self.work.place.patch_output == output {
            return;
        }
        if self.work.restored {
            self.close_work(notify, i18n);
        }
        output.clone_into(&mut self.work.place.patch_output);
        self.work.place.cache_root = workspace::cache_root_of(output);
    }

    /// Loads the work of the code when it changes, and saves pending edits.
    pub(in crate::patch) fn sync_workspace(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        if self.work.job.is_some() {
            return; // See `tool::job`: nothing is done to the work meanwhile.
        }
        let code = self.work.place.mod_code.trim();
        if is_existing_mod_code(code) && self.work.place.loaded_for.as_deref() != Some(code) {
            let code = code.to_owned();
            self.save_now(notify, i18n); // the previous workspace
            let root = self.work.place.cache_root.clone();
            self.work.job = Some(Job::spawn(I18nKey::NemesisReadingWork, code.clone(), 0, move |done| Done::Loaded {
                loaded: Workspace::load_counted(&root, &code, done),
                code,
            }));
            return;
        }

        // What is open is part of the work: back to it when it is opened again.
        let has_work = self.work.place.is_open() && !self.work.workspace.behaviors.is_empty();
        self.work.autosave.look_at(self.sel.active, self.sel.selected.as_ref(), has_work);

        if self.work.autosave.is_due() {
            self.save_now(notify, i18n);
        }
    }

    /// The work of `code` is loaded: see [`Self::sync_workspace`].
    pub(super) fn finish_load(
        &mut self,
        loaded: Result<Workspace, String>,
        code: String,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) {
        match loaded {
            Ok(workspace) => {
                self.work.workspace = workspace;
                if self.is_user_patch {
                    self.work.workspace.use_user_patch_format();
                }
                self.reset_names();
                self.tabs.clear();
            }
            Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::LoadFailedMessage))),
        }
        self.pane.opened = None;
        self.sel.selected = None;
        self.restore_session();
        remember_recent(
            &mut self.work.recent,
            RecentWork { output: self.work.place.output.trim().to_owned(), mod_code: code.clone() },
        );
        self.work.place.loaded_for = Some(code);
    }

    pub(in crate::patch) fn save_now(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        if self.work.job.is_some() {
            return; // A rename has taken the work: see `tool::job`.
        }
        self.sync_pandora_doc();
        let Some(code) = self.work.place.loaded_for.clone() else { return };
        if !self.work.autosave.take() {
            return;
        }
        let open = (self.sel.active, self.sel.selected.as_deref());
        self.work.workspace.session =
            super::session::of(&self.work.workspace.behaviors, &self.tabs, open, &self.work.place);
        // It has no `Export`: what is edited is what the next merge gets.
        if self.is_user_patch {
            match self.work.workspace.export(Path::new(&self.work.place.output), &code, self.work.parser_mode) {
                Ok((_, invalid)) if invalid.is_empty() => {}
                Ok((_, invalid)) => notify.warn(format!(
                    "{} ({}):\n{}",
                    i18n.t(I18nKey::NemesisRejected),
                    invalid.len(),
                    invalid.join("\n")
                )),
                Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisExportFailed))),
            }
            self.work.mod_written = true;
        }
        if let Err(err) = self.work.workspace.save(&self.work.place.cache_root, &code) {
            notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisSaveFailed)));
        }
        // The output may have changed.
        remember_recent(
            &mut self.work.recent,
            RecentWork { output: self.work.place.output.trim().to_owned(), mod_code: code },
        );
    }

    /// Back to what the work just loaded had open. See [`super::session::restore`].
    pub(super) fn restore_session(&mut self) {
        let session = std::mem::take(&mut self.work.workspace.session);
        let restored = super::session::restore(session, &self.work.workspace.behaviors, &mut self.tabs);

        self.sel.active = restored.active;
        self.sel.selected = restored.selected;
        if !restored.mod_dir.is_empty() {
            self.work.place.mod_dir = restored.mod_dir;
        }
        // The user patch's is decided by the patch page's.
        if !restored.output.is_empty() && !self.is_user_patch {
            self.work.place.output = restored.output;
        }
        self.work.autosave.seen_at(self.sel.active, self.sel.selected.as_ref());
    }

    /// Opens a work of the list of recent ones. If its cache is gone, the mod it was exported to is
    /// read instead; if that is gone too, it leaves the list.
    pub(in crate::patch) fn open_recent(&mut self, work: &RecentWork, notify: &mut Notification, i18n: &I18nMap) {
        if Workspace::exists(&self.work.place.cache_root, &work.mod_code) {
            self.switch_work(&work.mod_code, &work.output, "", notify, i18n);
        } else if let Some(dir) = workspace::patch_dir_of_code(Path::new(&work.output), &work.mod_code) {
            self.open_mod(&dir, notify, i18n);
        } else {
            notify.warn(format!("{}: {} ({})", i18n.t(I18nKey::NemesisRecentMissing), work.mod_code, work.output));
            self.work.recent.retain(|other| other.mod_code != work.mod_code);
        }
    }

    /// Back to the start screen. The work stays in its cache.
    pub(in crate::patch) fn close_work(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        self.save_now(notify, i18n);
        self.work.workspace = Workspace::default();
        self.reset_names();
        self.tabs.clear();
        self.work.place.loaded_for = None;
        self.work.place.mod_code.clear();
        self.work.place.output.clear();
        self.work.place.mod_dir.clear();
        self.sel.active = 0;
        self.sel.selected = None;
        self.pane.opened = None;
        self.panels.project_open = false;
    }

    /// `New` of the start screen: the work's first save makes it exist.
    pub(in crate::patch) fn create_work(&mut self, code: String, output: String, scope: workspace::TemplateScope) {
        self.panels.new_work = None;
        self.work.workspace.scope = scope;
        self.work.workspace.export_format = scope.default_format();
        self.work.place.mod_code = code;
        self.work.place.output = output;
        self.work.place.mod_dir.clear();
        self.panels.project_open = true; // Its name and author are still to be set.
        self.work.autosave.changed();
    }

    /// The folder dialog for [`Self::open_mod`].
    pub(in crate::patch) fn pick_mod(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        if let Some(dir) = gui_shared::rfd::FileDialog::new().pick_folder() {
            self.open_mod(&dir, notify, i18n);
        }
    }

    /// Opens a mod (its root or a patch directory). A work is of one mod code: of a root with several
    /// of them, the one to open is asked for, see [`crate::patch::ui::code_choice`].
    pub(in crate::patch) fn open_mod(&mut self, selected: &Path, notify: &mut Notification, i18n: &I18nMap) {
        let mut codes = workspace::mod_codes_of_root(selected);
        codes.retain(|mod_code| {
            is_existing_mod_code(&mod_code.code)
                && !mod_code.code.eq_ignore_ascii_case(gui_shared::patch::USER_PATCH_CODE)
        });
        if codes.len() > 1 {
            self.panels.code_choice = Some(ModCodeChoice { root: selected.to_path_buf(), codes });
            return;
        }
        let only = codes.first().and_then(|mod_code| workspace::patch_dir_of_code(selected, &mod_code.code));
        self.open_patch_dir(only.as_deref().unwrap_or(selected), notify, i18n);
    }

    /// The answer to the question of [`Self::open_mod`].
    pub(in crate::patch) fn open_chosen_code(&mut self, code: &str, notify: &mut Notification, i18n: &I18nMap) {
        let Some(choice) = self.panels.code_choice.take() else { return };
        match workspace::patch_dir_of_code(&choice.root, code) {
            Some(dir) => self.open_patch_dir(&dir, notify, i18n),
            None => notify.error(format!("{}: {}", i18n.t(I18nKey::NemesisNotAMod), choice.root.display())),
        }
    }

    /// Opens one mod code, like VS Code a folder: the work on it goes on if there is one, with its
    /// unexported edits. Else it is read, and exported back to where it is.
    fn open_patch_dir(&mut self, selected: &Path, notify: &mut Notification, i18n: &I18nMap) {
        let not_a_mod = |notify: &mut Notification| {
            notify.error(format!("{}: {}", i18n.t(I18nKey::NemesisNotAMod), selected.display()));
        };
        let Some(root) = workspace::mod_root_of(selected) else { return not_a_mod(notify) };
        let code = workspace::code_of(selected);
        if !is_existing_mod_code(&code) {
            return not_a_mod(notify);
        }
        if code.eq_ignore_ascii_case(gui_shared::patch::USER_PATCH_CODE) {
            notify.warn(i18n.t(I18nKey::NemesisUserPatchReserved));
            return;
        }
        let source = root.display().to_string();
        let is_source = |path: &str| path.eq_ignore_ascii_case(&source);
        match Workspace::cached_session(&self.work.place.cache_root, &code) {
            Some(session) if is_source(&session.mod_dir) || is_source(&session.output) => {
                self.switch_work(&code, &source, &source, notify, i18n);
                return;
            }
            Some(_) => notify.warn(format!("{}: {code}", i18n.t(I18nKey::NemesisWorkReplaced))),
            None => {}
        }
        self.read_mod(selected, source, None);
    }

    /// Replaces the work by what is in the mod at `selected`, as a job: see [`Self::finish_read`].
    /// `kept`: what a reload keeps of the work.
    fn read_mod(&mut self, selected: &Path, source: String, kept: Option<Kept>) {
        let (selected, templates) = (selected.to_path_buf(), PathBuf::from(&self.work.place.template_dir));
        let detail = selected.display().to_string();
        self.work.job = Some(Job::spawn(I18nKey::NemesisReadingWork, detail, 0, move |done| Done::Read {
            read: Workspace::import_counted(&selected, &templates, done),
            source,
            kept,
        }));
    }

    /// The mod is read: it is the work now. `source`: its root.
    pub(super) fn finish_read(
        &mut self,
        read: Result<(Workspace, String, Vec<String>), String>,
        source: String,
        kept: Option<Kept>,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) {
        let (workspace, code, skipped) = match read {
            Ok(read) => read,
            Err(err) => return notify.error(format!("{}: {err}", i18n.t(I18nKey::LoadFailedMessage))),
        };
        // Read again: the object that was being worked on comes back.
        let was_open =
            self.work.place.loaded_for.as_deref().is_some_and(|loaded| loaded == code).then(|| self.editor_place());
        self.save_now(notify, i18n);
        self.work.workspace = workspace;
        self.tabs.clear();
        self.work.place.mod_code.clone_from(&code);
        self.work.place.output.clone_from(&source);
        self.work.place.mod_dir = source;
        self.panels.new_work = None;
        // What was read is the work, rather than what may be in the cache.
        remember_recent(
            &mut self.work.recent,
            RecentWork { output: self.work.place.output.clone(), mod_code: code.clone() },
        );
        self.work.place.loaded_for = Some(code);
        // Read for the first time: what the mod has is the history's first revision, so the Changes
        // view lists only what this work changes in it.
        if kept.is_none() {
            self.work.workspace.commit_init();
        }
        // A reload: where it is exported to and the mods it refers to are not in the mod.
        if let Some(Kept { output, refs }) = kept {
            self.work.place.output = output;
            self.work.workspace.ref_mods = refs;
            self.work.workspace.reload_refs();
        }
        self.reset_names();
        self.work.mark_dirty();
        self.sel.active = 0;
        self.sel.selected = None;
        self.pane.opened = None;
        if let Some(Some(place)) = was_open {
            self.reopen_editor_place(&place);
        }
        if !skipped.is_empty() {
            notify.warn(format!("{}: {}", i18n.t(I18nKey::NemesisNoTemplateFor), skipped.join(", ")));
        }
    }

    /// `Reload from the mod`: discards the work's edits. Where it is exported to and the mods it
    /// refers to are not in the mod, so they stay.
    pub(in crate::patch) fn reload_from_mod(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let Some(code) = self.work.place.loaded_for.clone() else { return };
        let source = self.work.place.mod_dir.trim().to_owned();
        let Some(dir) = workspace::patch_dir_of_code(Path::new(&source), &code) else {
            notify.error(format!("{}: {source}", i18n.t(I18nKey::NemesisNotAMod)));
            return;
        };
        let kept = Kept { output: self.work.place.output.clone(), refs: self.work.workspace.ref_mods.clone() };
        self.read_mod(&dir, source, Some(kept));
    }
    pub(in crate::patch) fn add_behavior(&mut self, path: &Path, notify: &mut Notification, i18n: &I18nMap) {
        match Behavior::open(path) {
            Ok(behavior) => {
                if let Some(at) = self.work.workspace.behaviors.iter().position(|b| b.name == behavior.name) {
                    self.sel.active = at; // already there
                    return;
                }
                self.work.workspace.behaviors.push(behavior);
                self.work.workspace.reload_refs();
                self.sel.active = self.work.workspace.behaviors.len() - 1;
                self.sel.selected = None; // an id of the other behavior
                self.work.mark_dirty();
            }
            Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::LoadFailedMessage))),
        }
    }

    /// `Export` as Pandora: see [`workspace::ExportFormat`].
    fn export_pandora(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        self.save_now(notify, i18n);
        let Some(code) = self.work.place.loaded_for.clone() else { return };
        let output = self.work.place.output.trim().to_owned();
        match self.work.workspace.export_pandora(Path::new(&output), &code) {
            Ok(export) if export.problems.is_empty() => notify.success(format!(
                "{} ({}): {}",
                i18n.t(I18nKey::NemesisExportedPandora),
                export.written,
                workspace::joined(Path::new(&output), &["Pandora_Engine", "mod", &code]).display()
            )),
            Ok(export) => notify.warn(format!(
                "{} ({}):\n{}",
                i18n.t(I18nKey::NemesisExportedPandora),
                export.written,
                export.problems.join("\n")
            )),
            Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisExportFailed))),
        }
    }

    pub(in crate::patch) fn export(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        if self.work.workspace.export_format == workspace::ExportFormat::PandoraV5 {
            self.export_pandora(notify, i18n);
            return;
        }
        self.save_now(notify, i18n);
        let Some(code) = self.work.place.loaded_for.clone() else { return };
        let output = self.work.place.output.trim().to_owned();
        let exported = self.work.workspace.export(Path::new(&output), &code, self.work.parser_mode);
        self.work.mod_written |= exported.is_ok();
        match exported {
            Ok((written, invalid)) if invalid.is_empty() => notify.success(format!(
                "{} ({written}): {}",
                i18n.t(I18nKey::ExportedMessage),
                workspace::joined(Path::new(&output), &["Nemesis_Engine", "mod", &code]).display()
            )),
            Ok((_, invalid)) => notify.warn(format!(
                "{} ({}):\n{}",
                i18n.t(I18nKey::NemesisRejected),
                invalid.len(),
                invalid.join("\n")
            )),
            Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisExportFailed))),
        }
        // What it wrote is what the next export replaces.
        self.work.autosave.changed();
        self.save_now(notify, i18n);
    }
}
