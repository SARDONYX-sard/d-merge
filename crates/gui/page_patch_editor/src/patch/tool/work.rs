//! Opening, saving and exporting a work.

use std::{
    path::{Path, PathBuf},
    rc::Rc,
    time::Instant,
};

use egui_widgets::notify::Notification;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::pages::patch_editor::RecentWork,
};
use nemesis_diff::is_existing_mod_code;

use super::job::{Done, Job, Kept};
use crate::patch::{
    FocusAt, PatchTool, SAVE_DELAY, editor, remember_recent,
    ui::code_choice::ModCodeChoice,
    user_patch, workspace,
    workspace::{Behavior, Session, SessionTab, Workspace},
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
        code.clone_into(&mut self.mod_code);
        output.clone_into(&mut self.output);
        source.clone_into(&mut self.mod_dir);
    }

    /// After the work is synced: takes the value in, and shows where it went. It is a proposal for now:
    /// see [`user_patch::Draft`]. So nothing of it is saved until there is an edit.
    pub(in crate::patch) fn take_in_resolution(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let Some(request) = self.pending_resolution.take() else { return };
        self.undo_unedited_draft();
        let before = user_patch::Draft::before(&self.workspace);
        let dirty_since = self.dirty_since;
        if self.workspace.info.name.is_empty() {
            i18n.t(I18nKey::UserPatchTitle).clone_into(&mut self.workspace.info.name);
        }
        // What the value points to is that mod's.
        self.workspace.add_ref(Path::new(&request.mod_dir), gui_shared::patch::USER_PATCH_CODE);
        // And what is taken from the others is theirs.
        for side in &request.sides {
            self.workspace.add_ref(Path::new(&side.mod_dir), gui_shared::patch::USER_PATCH_CODE);
        }
        let root = PathBuf::from(&self.template_dir);
        let shown = editor::resolve::apply(&mut self.workspace, &root, &request).map_or_else(
            |err| {
                notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisResolveFailed)));
                // It is for the user to edit: all the more if it could not be taken in.
                editor::resolve::object_of(&mut self.workspace, &root, &request)
            },
            Some,
        );
        let Some(editor::resolve::Resolved { behavior, id, line }) = shown else { return };
        self.workspace.reload_refs(); // a behavior may have been opened
        self.reset_names();
        self.active = behavior;
        self.editor_for = None;
        self.draft = Some(before.with_proposal(&self.workspace, behavior, &id));
        self.merge_values = editor::resolve::merge_values(&request, &id);
        self.selected = Some(id.into());
        // What is shown is no change of the work yet.
        self.dirty_since = dirty_since;
        self.session_seen = (self.active, self.selected.clone());
        // Picking another value of the object shown has no other sign of having gone anywhere.
        self.focus_editor = line.map(FocusAt::Line);
    }

    /// The patch page's output directory may have changed: the works are others then. What is open is
    /// saved where it belongs first.
    pub(in crate::patch) fn set_patch_output(&mut self, output: &str, notify: &mut Notification, i18n: &I18nMap) {
        let output = output.trim();
        if self.patch_output == output {
            return;
        }
        if self.restored {
            self.close_work(notify, i18n);
        }
        output.clone_into(&mut self.patch_output);
        self.cache_root = workspace::cache_root_of(output);
    }

    /// Loads the work of the code when it changes, and saves pending edits.
    pub(in crate::patch) fn sync_workspace(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        if self.job.is_some() {
            return; // See `tool::job`: nothing is done to the work meanwhile.
        }
        let code = self.mod_code.trim();
        if is_existing_mod_code(code) && self.loaded_for.as_deref() != Some(code) {
            let code = code.to_owned();
            self.save_now(notify, i18n); // the previous workspace
            let root = self.cache_root.clone();
            self.job = Some(Job::spawn(I18nKey::NemesisReadingWork, code.clone(), 0, move |done| Done::Loaded {
                loaded: Workspace::load_counted(&root, &code, done),
                code,
            }));
            return;
        }

        // What is open is part of the work: back to it when it is opened again.
        if self.session_seen.0 != self.active || self.session_seen.1 != self.selected {
            self.session_seen = (self.active, self.selected.clone());
            if self.loaded_for.is_some() && !self.workspace.behaviors.is_empty() {
                self.dirty_since.get_or_insert_with(Instant::now);
            }
        }

        if self.dirty_since.is_some_and(|since| since.elapsed() >= SAVE_DELAY) {
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
                self.workspace = workspace;
                self.reset_names();
                self.tabs.clear();
            }
            Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::LoadFailedMessage))),
        }
        self.editor_for = None;
        self.selected = None;
        self.restore_session();
        remember_recent(&mut self.recent, RecentWork { output: self.output.trim().to_owned(), mod_code: code.clone() });
        self.loaded_for = Some(code);
    }

    pub(in crate::patch) fn save_now(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        if self.job.is_some() {
            return; // A rename has taken the work: see `tool::job`.
        }
        self.sync_pandora_doc();
        let Some(code) = self.loaded_for.clone() else { return };
        if self.dirty_since.take().is_none() {
            return;
        }
        self.workspace.session = self.current_session();
        // It has no `Export`: what is edited is what the next merge gets.
        if self.is_user_patch {
            match self.workspace.export(Path::new(&self.output), &code, self.parser_mode) {
                Ok((_, invalid)) if invalid.is_empty() => {}
                Ok((_, invalid)) => notify.warn(format!(
                    "{} ({}):\n{}",
                    i18n.t(I18nKey::NemesisRejected),
                    invalid.len(),
                    invalid.join("\n")
                )),
                Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisExportFailed))),
            }
            self.mod_written = true;
        }
        if let Err(err) = self.workspace.save(&self.cache_root, &code) {
            notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisSaveFailed)));
        }
        // The output may have changed.
        remember_recent(&mut self.recent, RecentWork { output: self.output.trim().to_owned(), mod_code: code });
    }

    /// What is open now, to write with the work.
    pub(in crate::patch) fn current_session(&self) -> Session {
        let behaviors = &self.workspace.behaviors;
        let name_of = |at: usize| behaviors.get(at).map_or_else(String::new, |behavior| behavior.name.clone());
        Session {
            mod_dir: self.mod_dir.clone(),
            output: self.output.clone(),
            active: name_of(self.active),
            selected: self.selected.as_deref().map(str::to_owned),
            tabs: self
                .tabs
                .all()
                .map(|tab| SessionTab { behavior: name_of(tab.behavior), id: tab.id.clone(), pinned: tab.pinned })
                .collect(),
        }
    }

    /// Back to what the work just loaded had open. What no longer exists is left out.
    pub(in crate::patch) fn restore_session(&mut self) {
        let session = std::mem::take(&mut self.workspace.session);
        let behaviors = &self.workspace.behaviors;
        let at_of = |name: &str| behaviors.iter().position(|behavior| behavior.name == name);
        for tab in &session.tabs {
            if let Some(at) = at_of(&tab.behavior)
                && behaviors[at].text(&tab.id).is_some()
            {
                self.tabs.restore(at, &tab.id, tab.pinned);
            }
        }
        self.active = at_of(&session.active).unwrap_or(0);
        self.selected = session
            .selected
            .filter(|id| behaviors.get(self.active).is_some_and(|b| b.text(id).is_some()))
            .map(Rc::from);
        if !session.mod_dir.is_empty() {
            self.mod_dir = session.mod_dir;
        }
        // The user patch's is decided by the patch page's.
        if !session.output.is_empty() && !self.is_user_patch {
            self.output = session.output;
        }
        self.session_seen = (self.active, self.selected.clone());
    }

    /// Opens a work of the list of recent ones. If its cache is gone, the mod it was exported to is
    /// read instead; if that is gone too, it leaves the list.
    pub(in crate::patch) fn open_recent(&mut self, work: &RecentWork, notify: &mut Notification, i18n: &I18nMap) {
        if Workspace::exists(&self.cache_root, &work.mod_code) {
            self.switch_work(&work.mod_code, &work.output, "", notify, i18n);
        } else if let Some(dir) = workspace::patch_dir_of_code(Path::new(&work.output), &work.mod_code) {
            self.open_mod(&dir, notify, i18n);
        } else {
            notify.warn(format!("{}: {} ({})", i18n.t(I18nKey::NemesisRecentMissing), work.mod_code, work.output));
            self.recent.retain(|other| other.mod_code != work.mod_code);
        }
    }

    /// Back to the start screen. The work stays in its cache.
    pub(in crate::patch) fn close_work(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        self.save_now(notify, i18n);
        self.workspace = Workspace::default();
        self.reset_names();
        self.tabs.clear();
        self.loaded_for = None;
        self.mod_code.clear();
        self.output.clear();
        self.mod_dir.clear();
        self.active = 0;
        self.selected = None;
        self.editor_for = None;
        self.project_open = false;
    }

    /// `New` of the start screen: the work's first save makes it exist.
    pub(in crate::patch) fn create_work(&mut self, code: String, output: String, scope: workspace::TemplateScope) {
        self.new_work = None;
        self.workspace.scope = scope;
        self.workspace.export_format = scope.default_format();
        self.mod_code = code;
        self.output = output;
        self.mod_dir.clear();
        self.project_open = true; // Its name and author are still to be set.
        self.dirty_since.get_or_insert_with(Instant::now);
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
            self.code_choice = Some(ModCodeChoice { root: selected.to_path_buf(), codes });
            return;
        }
        let only = codes.first().and_then(|mod_code| workspace::patch_dir_of_code(selected, &mod_code.code));
        self.open_patch_dir(only.as_deref().unwrap_or(selected), notify, i18n);
    }

    /// The answer to the question of [`Self::open_mod`].
    pub(in crate::patch) fn open_chosen_code(&mut self, code: &str, notify: &mut Notification, i18n: &I18nMap) {
        let Some(choice) = self.code_choice.take() else { return };
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
        match Workspace::cached_session(&self.cache_root, &code) {
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
    pub(in crate::patch) fn read_mod(&mut self, selected: &Path, source: String, kept: Option<Kept>) {
        let (selected, templates) = (selected.to_path_buf(), PathBuf::from(&self.template_dir));
        let detail = selected.display().to_string();
        self.job = Some(Job::spawn(I18nKey::NemesisReadingWork, detail, 0, move |done| Done::Read {
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
        let was_open = self.loaded_for.as_deref().is_some_and(|loaded| loaded == code).then(|| self.editor_place());
        self.save_now(notify, i18n);
        self.workspace = workspace;
        self.tabs.clear();
        self.mod_code.clone_from(&code);
        self.output.clone_from(&source);
        self.mod_dir = source;
        self.new_work = None;
        // What was read is the work, rather than what may be in the cache.
        remember_recent(&mut self.recent, RecentWork { output: self.output.clone(), mod_code: code.clone() });
        self.loaded_for = Some(code);
        // A reload: where it is exported to and the mods it refers to are not in the mod.
        if let Some(Kept { output, refs }) = kept {
            self.output = output;
            self.workspace.ref_mods = refs;
            self.workspace.reload_refs();
        }
        self.reset_names();
        self.mark_dirty();
        self.active = 0;
        self.selected = None;
        self.editor_for = None;
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
        let Some(code) = self.loaded_for.clone() else { return };
        let source = self.mod_dir.trim().to_owned();
        let Some(dir) = workspace::patch_dir_of_code(Path::new(&source), &code) else {
            notify.error(format!("{}: {source}", i18n.t(I18nKey::NemesisNotAMod)));
            return;
        };
        let kept = Kept { output: self.output.clone(), refs: self.workspace.ref_mods.clone() };
        self.read_mod(&dir, source, Some(kept));
    }
    /// Something to save.
    pub(in crate::patch) fn mark_dirty(&mut self) {
        self.dirty_since.get_or_insert_with(Instant::now);
    }

    pub(in crate::patch) fn add_behavior(&mut self, path: &Path, notify: &mut Notification, i18n: &I18nMap) {
        match Behavior::open(path) {
            Ok(behavior) => {
                if let Some(at) = self.workspace.behaviors.iter().position(|b| b.name == behavior.name) {
                    self.active = at; // already there
                    return;
                }
                self.workspace.behaviors.push(behavior);
                self.workspace.reload_refs();
                self.active = self.workspace.behaviors.len() - 1;
                self.selected = None; // an id of the other behavior
                self.mark_dirty();
            }
            Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::LoadFailedMessage))),
        }
    }

    /// `Export` as Pandora: see [`workspace::ExportFormat`].
    fn export_pandora(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        self.save_now(notify, i18n);
        let Some(code) = self.loaded_for.clone() else { return };
        let output = self.output.trim().to_owned();
        match self.workspace.export_pandora(Path::new(&output), &code) {
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
        if self.workspace.export_format == workspace::ExportFormat::PandoraV5 {
            self.export_pandora(notify, i18n);
            return;
        }
        self.save_now(notify, i18n);
        let Some(code) = self.loaded_for.clone() else { return };
        let output = self.output.trim().to_owned();
        let exported = self.workspace.export(Path::new(&output), &code, self.parser_mode);
        self.mod_written |= exported.is_ok();
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
        self.dirty_since.get_or_insert_with(Instant::now);
        self.save_now(notify, i18n);
    }
}
