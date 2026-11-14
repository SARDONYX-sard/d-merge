//! Opening, saving and exporting a work. What the editor shows after it is a [`Reshow`]: the work
//! says it, the editor does it.
//!
//! ```text
//! open_recent / open_mod -> switch_work (the cache)  -> sync_workspace -> job -> finish_load -> Reshow::Session
//!                        -> read_mod   (the mod)     -> job            -> finish_read -> Reshow::Place
//! close_work                                                                         -> Reshow::Nothing
//! ```

use std::path::{Path, PathBuf};

use egui_widgets::notify::Notification;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::pages::patch_editor::RecentWork,
};
use nemesis_diff::is_existing_mod_code;

use super::{
    job::{Done, Job, Kept, ReadMod},
    session::{Reshow, Shown},
    work_state::Work,
};
use crate::patch::{
    remember_recent,
    tool::selection::Selection,
    ui::code_choice::ModCodeChoice,
    workspace,
    workspace::{Behavior, Workspace},
};

impl Work {
    /// Goes to the work of `code`: [`Self::sync_workspace`] loads it, and what it saved of the output
    /// and the source wins over these. The current work is saved first, while these are still its own.
    pub(in crate::patch) fn switch_work(
        &mut self,
        shown: Shown<'_>,
        (code, output, source): (&str, &str, &str),
        notify: &mut Notification,
        i18n: &I18nMap,
    ) {
        self.save_now(shown, notify, i18n);
        code.clone_into(&mut self.place.mod_code);
        output.clone_into(&mut self.place.output);
        source.clone_into(&mut self.place.mod_dir);
    }

    /// The patch page's output directory may have changed: the works are others then. What is open is
    /// saved where it belongs first, and closed.
    pub(in crate::patch) fn set_patch_output(
        &mut self,
        shown: Shown<'_>,
        output: &str,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) -> Option<Reshow> {
        let output = output.trim();
        if self.place.patch_output == output {
            return None;
        }

        let closed = self.restored.then(|| self.close_work(shown, notify, i18n));
        output.clone_into(&mut self.place.patch_output);
        self.place.cache_root = workspace::cache_root_of(output);
        closed
    }

    /// Loads the work of the code when it changes, and saves pending edits.
    pub(in crate::patch) fn sync_workspace(&mut self, shown: Shown<'_>, notify: &mut Notification, i18n: &I18nMap) {
        if self.job.is_some() {
            return; // See `tool::job`: nothing is done to the work meanwhile.
        }

        let code = self.place.mod_code.trim();
        if is_existing_mod_code(code) && self.place.loaded_for.as_deref() != Some(code) {
            let code = code.to_owned();
            self.save_now(shown, notify, i18n); // the previous workspace
            let root = self.place.cache_root.clone();
            self.job = Some(Job::spawn(I18nKey::NemesisReadingWork, code.clone(), 0, move |done| Done::Loaded {
                loaded: Workspace::load_counted(&root, &code, done),
                code,
            }));
            return;
        }

        // What is open is part of the work: back to it when it is opened again.
        let has_work = self.place.is_open() && !self.workspace.behaviors.is_empty();
        self.autosave.look_at(shown.sel.active, shown.sel.selected.as_ref(), has_work);

        if self.autosave.is_due() {
            self.save_now(shown, notify, i18n);
        }
    }

    /// The work of `code` is loaded: see [`Self::sync_workspace`]. It shows what it had open.
    pub(super) fn finish_load(
        &mut self,
        loaded: Result<Workspace, String>,
        code: String,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) -> Reshow {
        match loaded {
            Ok(workspace) => {
                self.workspace = workspace;
                if self.user_patch {
                    self.workspace.use_user_patch_format();
                }
                self.reset_names();
            }
            Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::LoadFailedMessage))),
        }

        let output = self.place.output.trim().to_owned();
        remember_recent(&mut self.recent, RecentWork { output, mod_code: code.clone() });
        self.place.loaded_for = Some(code);
        Reshow::Session(std::mem::take(&mut self.workspace.session))
    }

    /// Saves the work now, with what is `shown`, if there is something to save.
    pub(in crate::patch) fn save_now(&mut self, shown: Shown<'_>, notify: &mut Notification, i18n: &I18nMap) {
        if self.job.is_some() {
            return; // A rename has taken the work: see `tool::job`.
        }
        self.sync_pandora_doc(shown.sel.active);
        if !self.autosave.is_dirty() {
            return;
        }
        self.workspace.session = super::session::of(&self.workspace.behaviors, shown, &self.place);
        self.save_as_is(notify, i18n);
    }

    /// Saves the work with the session it has, if there is something to save.
    fn save_as_is(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        if self.job.is_some() {
            return;
        }
        let Some(code) = self.place.loaded_for.clone() else { return };
        if !self.autosave.take() {
            return;
        }

        // It has no `Export`: what is edited is what the next merge gets.
        if self.user_patch {
            match self.workspace.export(Path::new(&self.place.output), &code, self.parser_mode) {
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

        if let Err(err) = self.workspace.save(&self.place.cache_root, &code) {
            notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisSaveFailed)));
        }

        // The output may have changed.
        let output = self.place.output.trim().to_owned();
        remember_recent(&mut self.recent, RecentWork { output, mod_code: code });
    }

    /// Back to what `session` had open: its tabs into `tabs`, and where the work was read from and
    /// exported to. Returns what it showed: `(behavior, object)`.
    pub(in crate::patch) fn restore_session(
        &mut self,
        session: workspace::Session,
        tabs: &mut crate::patch::editor::tabs::EditorTabs,
    ) -> (usize, Option<std::rc::Rc<str>>) {
        let restored = super::session::restore(session, &self.workspace.behaviors, tabs);
        self.take_restored_place((restored.mod_dir, restored.output));
        self.autosave.seen_at(restored.active, restored.selected.as_ref());
        (restored.active, restored.selected)
    }

    /// What a session restored says of the work's place: where it was read from and exported to.
    /// See [`super::session::restore`].
    fn take_restored_place(&mut self, (mod_dir, output): (String, String)) {
        if !mod_dir.is_empty() {
            self.place.mod_dir = mod_dir;
        }
        // The user patch's is decided by the patch page's.
        if !output.is_empty() && !self.user_patch {
            self.place.output = output;
        }
    }

    /// Opens a work of the list of recent ones. If its cache is gone, the mod it was exported to is
    /// read instead; if that is gone too, it leaves the list.
    ///
    /// Returns the question of [`Self::open_mod`], if it has one.
    pub(in crate::patch) fn open_recent(
        &mut self,
        shown: Shown<'_>,
        work: &RecentWork,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) -> Option<ModCodeChoice> {
        if Workspace::exists(&self.place.cache_root, &work.mod_code) {
            self.switch_work(shown, (&work.mod_code, &work.output, ""), notify, i18n);
            return None;
        }
        if let Some(dir) = workspace::patch_dir_of_code(Path::new(&work.output), &work.mod_code) {
            return self.open_mod(shown, &dir, notify, i18n);
        }
        notify.warn(format!("{}: {} ({})", i18n.t(I18nKey::NemesisRecentMissing), work.mod_code, work.output));
        self.recent.retain(|other| other.mod_code != work.mod_code);
        None
    }

    /// Back to the start screen. The work stays in its cache.
    pub(in crate::patch) fn close_work(
        &mut self,
        shown: Shown<'_>,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) -> Reshow {
        self.save_now(shown, notify, i18n);
        self.workspace = Workspace::default();
        self.reset_names();

        self.place.loaded_for = None;
        self.place.mod_code.clear();
        self.place.output.clear();
        self.place.mod_dir.clear();
        Reshow::Nothing
    }

    /// `New` of the start screen: the work's first save makes it exist.
    pub(in crate::patch) fn create_work(&mut self, code: String, output: String, scope: workspace::TemplateScope) {
        self.workspace.scope = scope;
        self.workspace.export_format = scope.default_format();
        self.place.mod_code = code;
        self.place.output = output;
        self.place.mod_dir.clear();
        self.autosave.changed();
    }

    /// The folder dialog for [`Self::open_mod`].
    pub(in crate::patch) fn pick_mod(
        &mut self,
        shown: Shown<'_>,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) -> Option<ModCodeChoice> {
        let dir = gui_shared::rfd::FileDialog::new().pick_folder()?;
        self.open_mod(shown, &dir, notify, i18n)
    }

    /// Opens a mod (its root or a patch directory). A work is of one mod code: of a root with several
    /// of them, the one to open is asked for: returned, see [`crate::patch::ui::code_choice`].
    pub(in crate::patch) fn open_mod(
        &mut self,
        shown: Shown<'_>,
        selected: &Path,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) -> Option<ModCodeChoice> {
        let mut codes = workspace::mod_codes_of_root(selected);
        codes.retain(|mod_code| {
            is_existing_mod_code(&mod_code.code)
                && !mod_code.code.eq_ignore_ascii_case(gui_shared::patch::USER_PATCH_CODE)
        });
        if codes.len() > 1 {
            return Some(ModCodeChoice { root: selected.to_path_buf(), codes });
        }

        let only = codes.first().and_then(|mod_code| workspace::patch_dir_of_code(selected, &mod_code.code));
        self.open_patch_dir(shown, only.as_deref().unwrap_or(selected), notify, i18n);
        None
    }

    /// The answer to the question of [`Self::open_mod`]: `code` of `choice`.
    pub(in crate::patch) fn open_chosen_code(
        &mut self,
        shown: Shown<'_>,
        (choice, code): (&ModCodeChoice, &str),
        notify: &mut Notification,
        i18n: &I18nMap,
    ) {
        match workspace::patch_dir_of_code(&choice.root, code) {
            Some(dir) => self.open_patch_dir(shown, &dir, notify, i18n),
            None => notify.error(format!("{}: {}", i18n.t(I18nKey::NemesisNotAMod), choice.root.display())),
        }
    }

    /// Opens one mod code, like VS Code a folder: the work on it goes on if there is one, with its
    /// unexported edits. Else it is read, and exported back to where it is.
    fn open_patch_dir(&mut self, shown: Shown<'_>, selected: &Path, notify: &mut Notification, i18n: &I18nMap) {
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
        match Workspace::cached_session(&self.place.cache_root, &code) {
            Some(session) if is_source(&session.mod_dir) || is_source(&session.output) => {
                self.switch_work(shown, (&code, &source, &source), notify, i18n);
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
        let (selected, templates) = (selected.to_path_buf(), PathBuf::from(&self.place.template_dir));
        let detail = selected.display().to_string();
        self.job = Some(Job::spawn(I18nKey::NemesisReadingWork, detail, 0, move |done| Done::Read {
            read: Workspace::import_counted(&selected, &templates, done),
            source,
            kept,
        }));
    }

    /// The mod is read: it is the work now. `source`: its root. Read again, the object that was being
    /// worked on comes back.
    pub(super) fn finish_read(
        &mut self,
        shown: Shown<'_>,
        (read, source, kept): (ReadMod, String, Option<Kept>),
        notify: &mut Notification,
        i18n: &I18nMap,
    ) -> Option<Reshow> {
        let (workspace, code, skipped) = match read {
            Ok(read) => read,
            Err(err) => {
                notify.error(format!("{}: {err}", i18n.t(I18nKey::LoadFailedMessage)));
                return None;
            }
        };

        let read_again = self.place.loaded_for.as_deref() == Some(code.as_str());
        let was_open = read_again.then(|| self.place_of(shown.sel)).flatten();
        self.save_now(shown, notify, i18n);

        self.workspace = workspace;
        self.place.mod_code.clone_from(&code);
        self.place.output.clone_from(&source);
        self.place.mod_dir = source;
        // What was read is the work, rather than what may be in the cache.
        let output = self.place.output.clone();
        remember_recent(&mut self.recent, RecentWork { output, mod_code: code.clone() });
        self.place.loaded_for = Some(code);

        // Read for the first time: what the mod has is the history's first revision, so the Changes
        // view lists only what this work changes in it.
        match kept {
            None => {
                self.workspace.commit_init();
            }
            // A reload: where it is exported to and the mods it refers to are not in the mod.
            Some(Kept { output, refs }) => {
                self.place.output = output;
                self.workspace.ref_mods = refs;
                self.workspace.reload_refs();
            }
        }
        self.reset_names();
        self.mark_dirty();

        if !skipped.is_empty() {
            notify.warn(format!("{}: {}", i18n.t(I18nKey::NemesisNoTemplateFor), skipped.join(", ")));
        }
        Some(Reshow::Place(was_open))
    }

    /// What `sel` shows, to find it again in another workspace: the behavior's `meshes_path` (e.g.
    /// `meshes/actors/character/behaviors/0_master.bin`) and the object id (e.g. `#0106`).
    fn place_of(&self, sel: &Selection) -> Option<(String, std::rc::Rc<str>)> {
        let behavior = self.workspace.behaviors.get(sel.active)?;
        Some((behavior.meshes_path.clone(), sel.selected.clone()?))
    }

    /// `Reload from the mod`: discards the work's edits. Where it is exported to and the mods it
    /// refers to are not in the mod, so they stay.
    pub(in crate::patch) fn reload_from_mod(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let Some(code) = self.place.loaded_for.clone() else { return };
        let source = self.place.mod_dir.trim().to_owned();
        let Some(dir) = workspace::patch_dir_of_code(Path::new(&source), &code) else {
            notify.error(format!("{}: {source}", i18n.t(I18nKey::NemesisNotAMod)));
            return;
        };
        let kept = Kept { output: self.place.output.clone(), refs: self.workspace.ref_mods.clone() };
        self.read_mod(&dir, source, Some(kept));
    }

    /// Opens the template at `path` as a behavior of the work, and shows it in `sel`.
    pub(in crate::patch) fn add_behavior(
        &mut self,
        sel: &mut Selection,
        path: &Path,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) {
        let behavior = match Behavior::open(path) {
            Ok(behavior) => behavior,
            Err(err) => return notify.error(format!("{}: {err}", i18n.t(I18nKey::LoadFailedMessage))),
        };
        if let Some(at) = self.workspace.behaviors.iter().position(|b| b.name == behavior.name) {
            sel.active = at; // already there
            return;
        }

        self.workspace.behaviors.push(behavior);
        self.workspace.reload_refs();
        sel.active = self.workspace.behaviors.len() - 1;
        sel.selected = None; // an id of the other behavior
        self.mark_dirty();
    }

    /// `Export`, in the work's format: see [`workspace::ExportFormat`].
    pub(in crate::patch) fn export(&mut self, shown: Shown<'_>, notify: &mut Notification, i18n: &I18nMap) {
        self.save_now(shown, notify, i18n);
        let Some(code) = self.place.loaded_for.clone() else { return };
        let output = self.place.output.trim().to_owned();

        if self.workspace.export_format == workspace::ExportFormat::PandoraV5 {
            self.export_pandora(Path::new(&output), &code, notify, i18n);
            return;
        }
        self.export_nemesis(Path::new(&output), &code, notify, i18n);
        // What it wrote is what the next export replaces.
        self.autosave.changed();
        self.save_now(shown, notify, i18n);
    }

    /// `Export` as Nemesis, to `output`.
    fn export_nemesis(&mut self, output: &Path, code: &str, notify: &mut Notification, i18n: &I18nMap) {
        let exported = self.workspace.export(output, code, self.parser_mode);
        self.mod_written |= exported.is_ok();
        match exported {
            Ok((written, invalid)) if invalid.is_empty() => notify.success(format!(
                "{} ({written}): {}",
                i18n.t(I18nKey::ExportedMessage),
                workspace::joined(output, &["Nemesis_Engine", "mod", code]).display()
            )),
            Ok((_, invalid)) => notify.warn(format!(
                "{} ({}):\n{}",
                i18n.t(I18nKey::NemesisRejected),
                invalid.len(),
                invalid.join("\n")
            )),
            Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisExportFailed))),
        }
    }

    /// `Export` as Pandora, to `output`.
    fn export_pandora(&self, output: &Path, code: &str, notify: &mut Notification, i18n: &I18nMap) {
        match self.workspace.export_pandora(output, code) {
            Ok(export) if export.problems.is_empty() => notify.success(format!(
                "{} ({}): {}",
                i18n.t(I18nKey::NemesisExportedPandora),
                export.written,
                workspace::joined(output, &["Pandora_Engine", "mod", code]).display()
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

    /// The rename of the code is done: the work is `work`, under `new`. Saved under it, and only then
    /// does the old one go. It shows what it had open.
    pub(super) fn finish_rename(
        &mut self,
        (work, old, new): (Workspace, &str, &str),
        notify: &mut Notification,
        i18n: &I18nMap,
    ) -> Reshow {
        self.workspace = work;
        new.clone_into(&mut self.place.mod_code);
        self.place.loaded_for = Some(new.to_owned());
        self.reset_names();
        self.mark_dirty();
        self.save_as_is(notify, i18n);

        // Only once it is saved under the new code does the old one go.
        let old_dir = Workspace::cache_dir(&self.place.cache_root, old);
        if Workspace::exists(&self.place.cache_root, new)
            && let Err(err) = std::fs::remove_dir_all(&old_dir)
        {
            notify.warn(format!("{}: {err}", old_dir.display()));
        }
        for work in self.recent.iter_mut().filter(|work| work.mod_code.eq_ignore_ascii_case(old)) {
            new.clone_into(&mut work.mod_code);
        }
        self.autosave.changed(); // the session restored is saved with it
        notify.success(format!("{}: {old} -> {new}", i18n.t(I18nKey::NemesisRenameCodeDone)));
        Reshow::Session(std::mem::take(&mut self.workspace.session))
    }
}
