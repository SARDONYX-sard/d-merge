//! Which windows, forms and sections of the page are open, and what is typed in them. Each is `None`
//! / `false` while closed; nothing here is part of the work.
//!
//! ```text
//! top bar      project_open (details), rename_to (the rename form)
//! start screen new_work, code_choice (a mod with several codes)
//! windows      picker (templates), class_picker (New object), repeat,
//!              anim_data_dialog, duplicate_form, new_clips (see [`ui::windows`])
//! user patch   confirm_clear (the bulk delete), mod_export
//! ```

use crate::patch::{new_work, ui};

/// See the module docs.
#[derive(Debug, Default)]
pub(in crate::patch) struct Panels {
    /// The details of the bar above are shown. See [`ui::top_bar`].
    pub(in crate::patch) project_open: bool,
    /// The mod code being typed to rename the work to, while that form is open.
    pub(in crate::patch) rename_to: Option<String>,
    /// See [`new_work`]. `Some`: its form is shown on the start screen.
    pub(in crate::patch) new_work: Option<new_work::NewWork>,
    /// A mod with several codes was opened: which of them? See [`ui::code_choice`].
    pub(in crate::patch) code_choice: Option<ui::code_choice::ModCodeChoice>,
    /// The window of the templates to add. See [`ui::template_picker`].
    pub(in crate::patch) picker: ui::template_picker::TemplatePicker,
    /// The "New object" window. See [`ui::class_picker`].
    pub(in crate::patch) class_picker: ui::class_picker::ClassPicker,
    /// The window making many objects from one. See [`ui::repeat`].
    pub(in crate::patch) repeat: Option<ui::repeat::RepeatForm>,
    /// The "animation data" window.
    pub(super) anim_data_dialog: Option<ui::anim_data::AnimDataDialog>,
    /// The "duplicate to" window.
    pub(super) duplicate_form: Option<ui::anim_data::DuplicateForm>,
    /// The "new clips" window.
    pub(super) new_clips: Option<ui::anim_data::NewClipsForm>,
    /// The user patch's bulk delete asks once more.
    pub(in crate::patch) confirm_clear: bool,
    /// The user patch written as a mod of its own. See [`crate::patch::user_patch::ModExport`].
    pub(in crate::patch) mod_export: crate::patch::user_patch::ModExport,
}
