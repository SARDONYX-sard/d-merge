//! The work: what is being made (the behaviors and their edits), where it lives, and when it is
//! saved. Nothing here is drawn.
//!
//! ```text
//! workspace     the behaviors, their edits and new objects, the history
//! place         which work it is, and where its files are
//! autosave      when it is saved next
//! job           reading a mod / renaming its code on another thread (nothing saves meanwhile)
//! names         the events / variables / pointers of the active behavior, kept up to date
//! edits         a counter of edits: what is found again after one (the changes)
//! reloads       a counter of whole changes (read, loaded, closed): the problems are found again
//! user_patch    this is the user patch: written at each save, no `Export`
//! draft         what `Edit...` of a conflict proposed, until it is edited
//! ```

use std::rc::Rc;

use gui_shared::settings::pages::{patch::behavior::ParserMode, patch_editor::RecentWork};

use crate::patch::{
    editor::names::{Names, NamesCache},
    tool::{autosave::Autosave, job::Job, place::WorkPlace},
    user_patch::Draft,
    workspace::{PANDORA_DOC, Workspace, pandora},
};

/// See the module docs.
#[derive(Debug, Default)]
pub(in crate::patch) struct Work {
    pub(in crate::patch) workspace: Workspace,
    /// Which work is open, and where its files are.
    pub(in crate::patch) place: WorkPlace,
    /// When the work is saved next.
    pub(in crate::patch) autosave: Autosave,
    /// What runs on another thread: reading a mod, renaming its code. See [`crate::patch::tool::job`].
    pub(in crate::patch) job: Option<Job>,
    /// The names of the events / variables panel and the completion.
    pub(in crate::patch) names: NamesCache,
    /// Edits made, counted: what is found anew after one, e.g. [`crate::patch::side_area::changes::Pending`].
    pub(in crate::patch) edits: u64,
    /// The work changed as a whole, counted: see [`Self::reset_names`].
    pub(in crate::patch) reloads: u64,
    /// The patch page's parser mode: what d-merge will accept, so what is checked here.
    pub(in crate::patch) parser_mode: ParserMode,
    /// The works opened lately, the last one first. From the settings.
    pub(in crate::patch) recent: Vec<RecentWork>,
    /// `output` / `code` were taken from the settings.
    pub(in crate::patch) restored: bool,
    /// A mod was written since the conflict check last asked.
    pub(in crate::patch) mod_written: bool,
    /// What was edited of the active behavior since the Pandora file was last synced.
    pub(in crate::patch) pandora_edited: Vec<String>,
    /// The projects of the clip data template, read once. See [`crate::patch::anim::data::template_projects`].
    pub(in crate::patch) template_projects: Option<Vec<String>>,
    /// This is the user patch's work, in its window: see [`crate::patch::user_patch`].
    pub(in crate::patch) user_patch: bool,
    /// What `Edit...` proposed, until it is edited. See [`Draft`].
    pub(in crate::patch) draft: Option<Draft>,
    /// The value of a conflict `Edit...` asked for, taken in on the next frame: see
    /// [`crate::patch::user_patch::take_in_resolution`].
    pub(in crate::patch) resolution: Option<gui_shared::patch::ResolveRequest>,
}

impl Work {
    /// Something to save: it will be, a moment from now, and what is found of the edits is found again.
    pub(in crate::patch) fn mark_dirty(&mut self) {
        self.autosave.changed();
        self.edits += 1;
    }

    /// The text of object `id` changed, or it was added or deleted: `names` takes that one in.
    pub(in crate::patch) fn object_changed(&mut self, id: &str) {
        self.names.changed(id);
        self.mark_dirty();
    }

    /// The behaviors themselves changed (loaded, closed): `names` starts over, and what was found of
    /// the work before (the changes, the problems) is not of this one.
    pub(in crate::patch) fn reset_names(&mut self) {
        self.names.reset();
        self.edits += 1;
        self.reloads += 1;
    }

    /// The names of behavior `active`, with the changes since the last call taken in.
    ///
    /// Asked for every frame: an `Rc`, so the thousands of strings are not copied each time.
    pub(in crate::patch) fn names(&mut self, active: usize) -> Rc<Names> {
        if !self.names.is_of(active) {
            self.reset_names(); // another behavior: what was found of this one goes too
        }
        self.names.get(active, &self.workspace.behaviors[active])
    }

    /// Object `id` of behavior `active` was edited here: its part of the Pandora patch file follows
    /// at the next pause. See [`Self::sync_pandora_doc`].
    pub(in crate::patch) fn note_pandora_edit(&mut self, active: usize, id: &str) {
        if self.workspace.behaviors[active].pandora_doc.is_some()
            && !self.pandora_edited.iter().any(|edited| edited == id)
        {
            self.pandora_edited.push(id.to_owned());
        }
    }

    /// What was edited of behavior `active` since the last time goes to the other side: an object to
    /// its part of the Pandora patch file, the file to the objects. After a pause in typing, as it
    /// reads the whole file.
    pub(in crate::patch) fn sync_pandora_doc(&mut self, active: usize) {
        for id in std::mem::take(&mut self.pandora_edited) {
            let Some(behavior) = self.workspace.behaviors.get_mut(active) else { continue };
            if id != PANDORA_DOC {
                behavior.update_pandora_doc(&id);
                continue;
            }
            let (changed, _) = behavior.apply_pandora_doc();
            for changed in changed {
                self.object_changed(&changed);
            }
        }
    }

    /// The Pandora patch file of behavior `active`: the first time, it is written from what the work
    /// has changed so far.
    pub(in crate::patch) fn make_pandora_doc(&mut self, active: usize) {
        let code = self.place.mod_code.trim();
        let behavior = &mut self.workspace.behaviors[active];
        if behavior.pandora_doc.is_some() {
            return;
        }
        let doc = pandora::draft_doc(behavior, code);
        behavior.pandora_doc_ids = pandora_patch::doc::doc_object_ids(&doc);
        behavior.pandora_doc = Some(doc);
        self.mark_dirty();
    }

    /// A new object in behavior `active`, empty or a copy of `from`. Returns its id.
    pub(in crate::patch) fn new_object(&mut self, active: usize, from: Option<&str>) -> String {
        let code = self.place.mod_code.trim();
        let id = self.workspace.behaviors[active].add_object(code, from);
        self.object_changed(&id);
        id
    }

    /// Deletes a new object `id` of behavior `active`, or drops the edits of a template object.
    /// Returns whether the object is gone (its tab with it).
    pub(in crate::patch) fn revert_or_delete(&mut self, active: usize, id: &str) -> bool {
        let behavior = &mut self.workspace.behaviors[active];
        behavior.edits.remove(id);
        let is_pandora_doc = id == PANDORA_DOC && behavior.pandora_doc.take().is_some();
        let gone = behavior.added.remove(id).is_some() || is_pandora_doc;
        self.object_changed(id);
        gone
    }
}
