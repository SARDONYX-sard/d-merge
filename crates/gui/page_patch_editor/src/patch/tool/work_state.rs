//! The work: what is being made (the behaviors and their edits), where it lives, and when it is
//! saved. Nothing here is drawn.
//!
//! ```text
//! workspace     the behaviors, their edits and new objects, the history
//! place         which work it is, and where its files are
//! autosave      when it is saved next
//! job           reading a mod / renaming its code on another thread (nothing saves meanwhile)
//! names         the events / variables / pointers of the active behavior, kept up to date
//! edits         a counter of edits: what is found again after one (the changes, the problems)
//! ```

use gui_shared::settings::pages::{patch::behavior::ParserMode, patch_editor::RecentWork};

use crate::patch::{
    editor::names::NamesCache,
    tool::{autosave::Autosave, job::Job, place::WorkPlace},
    workspace::Workspace,
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
    /// Edits made, counted: what is found anew after one, e.g. [`crate::patch::changes::Pending`].
    pub(in crate::patch) edits: u64,
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
}

impl Work {
    /// Something to save: it will be, a moment from now, and what is found of the edits is found again.
    pub(in crate::patch) fn mark_dirty(&mut self) {
        self.autosave.changed();
        self.edits += 1;
    }
}
