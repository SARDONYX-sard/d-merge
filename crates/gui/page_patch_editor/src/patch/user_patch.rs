//! The user patch: what the conflict tool decided. A mod of its own (`upatch`) that loads last.
//!
//! It has its own editor in its own window, which can be opened on any page, whatever the main editor
//! has open: the patch tool again, kept by the main one. What it is written to is decided by the
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
    FocusAt, PatchPaths,
    editor::{merge, resolve},
    tool::{
        editing::{EditorArea, EditorPane},
        place::WorkPlace,
        selection::Selection,
        session::Reshow,
        work_state::Work,
    },
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

/// Writing the user patch as a mod of its own, e.g. into MO2's `mods` to manage or share it there:
/// to `output`, as `format`. Its own place (`<output>/.d_merge/user_patch`, NemesisExt) and what it
/// wrote there stay as they are.
///
/// As `Nemesis`, the templates Nemesis does not know can't be in `Nemesis_Engine`: they go to
/// `Nemesis_EngineExt` as always ([`workspace::Workspace::outside_nemesis`]), and the window says so.
///
/// ```text
/// [Export as mod…] -> Output: [D:/MO2/mods/My Patch] [Select]   Format: [Nemesis ▼]   [Export]
///                     ⚠ Nemesis does not know these templates: ... (1)   <- only as Nemesis
/// ```
#[derive(Debug, Default)]
pub(super) struct ModExport {
    pub(super) open: bool,
    /// The mod's root, e.g. `D:/MO2/mods/My Patch`.
    pub(super) output: String,
    /// Any of them. See the struct docs for `Nemesis`.
    pub(super) format: workspace::ExportFormat,
}

/// What [`write_as_mod`] wrote.
struct ModExported<'a> {
    output: &'a str,
    /// The patches d-merge's parser rejects, or why nothing was written.
    written: Result<Vec<String>, String>,
    /// For the Nemesis format: the templates Nemesis does not know (their patches are of no use there).
    outside: Vec<&'a str>,
}

/// Writes the user patch of `work` as a mod, where and how `export` says. `None`: no output yet.
fn write_as_mod<'a>(
    work: &'a mut Workspace,
    export: &'a ModExport,
    mode: gui_shared::settings::pages::patch::behavior::ParserMode,
) -> Option<ModExported<'a>> {
    let output = export.output.trim();
    if output.is_empty() {
        return None;
    }

    let root = Path::new(output);
    let chosen = export.format;
    let written = if chosen == workspace::ExportFormat::PandoraV5 {
        work.export_pandora(root, USER_PATCH_CODE).map(|export| export.problems)
    } else {
        // The files of its own place are what its next save replaces: not those of this mod.
        let owned = std::mem::take(&mut work.owned);
        let format = std::mem::replace(&mut work.export_format, chosen);
        let written = work.export(root, USER_PATCH_CODE, mode).map(|(_, invalid)| invalid);
        (work.owned, work.export_format) = (owned, format);
        written
    };

    let outside = match chosen {
        workspace::ExportFormat::Nemesis => work.outside_nemesis().collect(),
        _ => Vec::new(),
    };
    Some(ModExported { output, written, outside })
}

/// Tells how [`write_as_mod`] went: written, written with templates Nemesis does not know,
/// rejected patches, or failed.
fn report_mod_export(notify: &mut Notification, i18n: &I18nMap, exported: &ModExported<'_>) {
    let ModExported { output, written, outside } = exported;
    match written {
        Ok(problems) if problems.is_empty() && outside.is_empty() => {
            notify.success(format!("{}: {output}", i18n.t(I18nKey::ExportedMessage)));
        }
        Ok(problems) if problems.is_empty() => notify.warn(format!(
            "{}: {output}\n{} ({}):\n{}",
            i18n.t(I18nKey::ExportedMessage),
            i18n.t(I18nKey::UserPatchOutsideNemesis),
            outside.len(),
            outside.join("\n")
        )),
        Ok(problems) => {
            notify.warn(format!("{} ({}):\n{}", i18n.t(I18nKey::NemesisRejected), problems.len(), problems.join("\n")));
        }
        Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisExportFailed))),
    }
}

/// Writes the user patch of `work` as a mod: see [`ModExport`] and [`write_as_mod`].
pub(super) fn export_as_mod(work: &mut Work, export: &ModExport, notify: &mut Notification, i18n: &I18nMap) {
    if let Some(exported) = write_as_mod(&mut work.workspace, export, work.parser_mode) {
        report_mod_export(notify, i18n, &exported);
    }
}

/// The user patch's work, as it starts: nothing open yet, kept where `paths` say.
pub(super) fn work_of(paths: PatchPaths<'_>) -> Work {
    let output = paths.patch_output_dir.trim();
    let mut workspace = Workspace::default();
    workspace.use_user_patch_format();
    let place = WorkPlace {
        patch_output: output.to_owned(),
        cache_root: workspace::cache_root_of(output),
        output: user_patch_output(output).display().to_string(),
        mod_code: USER_PATCH_CODE.to_owned(),
        template_dir: paths.template_dir.to_owned(),
        ..Default::default()
    };
    Work { workspace, place, restored: true, parser_mode: paths.parser_mode, user_patch: true, ..Default::default() }
}

impl Work {
    /// Writes the user patch now. Saved again after: what the export wrote is part of the work.
    /// Returns the patches d-merge's parser rejects.
    fn write_user_patch(&mut self) -> Result<Vec<String>, String> {
        self.workspace.save(&self.place.cache_root, USER_PATCH_CODE)?;
        let invalid = self.workspace.export(Path::new(&self.place.output), USER_PATCH_CODE, self.parser_mode)?.1;
        self.workspace.save(&self.place.cache_root, USER_PATCH_CODE)?;
        self.autosave.take();
        Ok(invalid)
    }

    /// `Use this`: the value of `request` goes in now (templates at `root`), and the mod is written.
    /// Returns the patches d-merge's parser rejects.
    ///
    /// # Errors
    /// The value could not be taken in, or written.
    pub(super) fn resolve_now(
        &mut self,
        request: &ResolveRequest,
        root: &Path,
        i18n: &I18nMap,
    ) -> Result<Vec<String>, String> {
        if self.workspace.info.name.is_empty() {
            i18n.t(I18nKey::UserPatchTitle).clone_into(&mut self.workspace.info.name);
        }
        // What the value points to is that mod's.
        self.workspace.add_ref(Path::new(&request.mod_dir), USER_PATCH_CODE);
        resolve::apply(&mut self.workspace, root, request)?;
        self.workspace.reload_refs(); // a behavior may have been opened
        self.reset_names();

        self.write_user_patch()
    }

    /// Takes everything out of the user patch, and writes that: nothing of it is merged any more.
    pub(super) fn clear_user_patch(&mut self, notify: &mut Notification, i18n: &I18nMap) -> Reshow {
        self.draft = None;
        self.workspace.behaviors.clear();
        self.workspace.ref_mods.clear();
        self.reset_names();
        match self.write_user_patch() {
            Ok(_) => self.mod_written = true,
            Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisExportFailed))),
        }
        Reshow::Nothing
    }
}

/// After the work is synced: the value of `request` goes in, and the editor shows where it went. It
/// is a proposal for now: see [`Draft`]. So nothing of it is saved until there is an edit.
pub(super) fn take_in_resolution(
    request: &ResolveRequest,
    (work, editor, sel): (&mut Work, &mut EditorArea, &mut Selection),
    notify: &mut Notification,
    i18n: &I18nMap,
) {
    undo_unedited_draft((work, editor, sel));
    let before = Draft::before(&work.workspace);
    let since = work.autosave.since();
    if work.workspace.info.name.is_empty() {
        i18n.t(I18nKey::UserPatchTitle).clone_into(&mut work.workspace.info.name);
    }
    // What the value points to is that mod's.
    work.workspace.add_ref(Path::new(&request.mod_dir), USER_PATCH_CODE);
    // And what is taken from the others is theirs.
    for side in &request.sides {
        work.workspace.add_ref(Path::new(&side.mod_dir), USER_PATCH_CODE);
    }

    let root = PathBuf::from(&work.place.template_dir);
    let shown = resolve::apply(&mut work.workspace, &root, request).map_or_else(
        |err| {
            notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisResolveFailed)));
            // It is for the user to edit: all the more if it could not be taken in.
            resolve::object_of(&mut work.workspace, &root, request)
        },
        Some,
    );
    let Some(resolve::Resolved { behavior, id, line }) = shown else { return };
    work.workspace.reload_refs(); // a behavior may have been opened
    work.reset_names();

    sel.active = behavior;
    editor.pane.opened = None;
    work.draft = Some(before.with_proposal(&work.workspace, behavior, &id));
    editor.pane.merge_values = resolve::merge_values(request, &id);
    editor.pane.merge_accepted.clear();
    reuse_resolution((work, &mut editor.pane), (behavior, &id), notify, i18n);
    sel.selected = Some(id.into());

    // What is shown is no change of the work yet.
    work.autosave.restore(since);
    work.autosave.seen_at(sel.active, sel.selected.as_ref());
    // Picking another value of the object shown has no other sign of having gone anywhere.
    editor.pane.focus = line.map(FocusAt::Line);
}

/// The conflict just taken in was resolved before, by the same mods with the same values: object
/// `(behavior, id)` is resolved the same way at once. See [`crate::patch::editor::merge`].
fn reuse_resolution(
    (work, pane): (&mut Work, &mut EditorPane),
    (behavior, id): (usize, &str),
    notify: &mut Notification,
    i18n: &I18nMap,
) {
    let Some(values) = &pane.merge_values else { return };
    let Some(resolution) = merge::find(&work.workspace.resolutions, values) else { return };
    let result = resolution.result.clone();
    let Some(text) = work.workspace.behaviors.get(behavior).and_then(|of| of.text(id)) else { return };

    let candidates = merge::candidates(values, &[]);
    if let Some(resolved) = merge::replace_block(text, &candidates, &result) {
        work.workspace.behaviors[behavior].set_text(id, &resolved);
        pane.merge_accepted = result;
        notify.info(i18n.t(I18nKey::MergeReused));
    }
}

/// What `Edit...` proposed goes, unless it was edited. See [`Draft`].
pub(super) fn undo_unedited_draft((work, editor, sel): (&mut Work, &mut EditorArea, &mut Selection)) {
    editor.pane.merge_values = None;
    let Some(draft) = work.draft.take() else { return };
    if draft.is_edited(&work.workspace) {
        return;
    }

    let id = draft.id.clone();
    draft.undo(&mut work.workspace);
    work.reset_names();
    editor.pane.opened = None;
    editor.pane.preview = None;
    sel.active = sel.active.min(work.workspace.behaviors.len().saturating_sub(1));
    if work.workspace.behaviors.get(sel.active).is_none_or(|behavior| behavior.text(&id).is_none()) {
        editor.tabs.clear();
        sel.selected = None;
    }
    work.autosave.seen_at(sel.active, sel.selected.as_ref());
    // Something saved meanwhile (another object) had it in: the disk goes back too.
    if work.mod_written {
        work.autosave.changed();
    }
}

#[cfg(test)]
mod tests {
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
        let work = work_of(paths("D:/out"));
        assert_eq!(Path::new(&work.place.output), Path::new("D:/out/.d_merge/user_patch"));
        assert_eq!(work.place.mod_code, USER_PATCH_CODE);
        assert!(work.place.cache_root.starts_with("D:/out/.d_merge/caches"));
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
}
