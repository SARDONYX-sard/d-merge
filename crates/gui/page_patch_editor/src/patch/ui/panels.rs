//! Which windows, forms and sections of the page are open, and what is typed in them. Each is `None`
//! / `false` while closed; nothing here is part of the work.
//!
//! ```text
//! top bar      project_open (details), rename_to (the rename form)
//! side bar     behaviors_open (BEHAVIORS)
//! start screen new_work, code_choice (a mod with several codes)
//! windows      picker (templates), class_picker + class_filter (New object), repeat,
//!              anim_data_dialog, duplicate_form + pending_duplicate, new_clips
//! asks         confirm_clear (the bulk delete)
//! ```

use crate::patch::{new_work, ui};

/// See the module docs.
#[derive(Debug, Default)]
pub(in crate::patch) struct Panels {
    /// The details of the bar above are shown. See [`ui::top_bar`].
    pub(in crate::patch) project_open: bool,
    /// The mod code being typed to rename the work to, while that form is open.
    pub(in crate::patch) rename_to: Option<String>,
    /// The side bar's behavior list is shown (else only its header).
    pub(in crate::patch) behaviors_open: bool,
    /// See [`new_work`]. `Some`: its form is shown on the start screen.
    pub(in crate::patch) new_work: Option<new_work::NewWork>,
    /// A mod with several codes was opened: which of them? See [`ui::code_choice`].
    pub(in crate::patch) code_choice: Option<ui::code_choice::ModCodeChoice>,
    /// The window of the templates to add. See [`ui::template_picker`].
    pub(in crate::patch) picker: ui::template_picker::TemplatePicker,
    /// The next quick pick window drawn takes the keyboard.
    pub(in crate::patch) focus_quick_pick: bool,
    /// The "New" class list is open, and its filter.
    pub(in crate::patch) class_picker: bool,
    pub(in crate::patch) class_filter: String,
    /// The window making many objects from one. See [`ui::repeat`].
    pub(in crate::patch) repeat: Option<ui::repeat::RepeatForm>,
    /// The "animation data" window.
    pub(super) anim_data_dialog: Option<ui::anim_data::AnimDataDialog>,
    /// The "duplicate to" window.
    pub(super) duplicate_form: Option<ui::anim_data::DuplicateForm>,
    /// What a row's menu asked to duplicate: `(files, to the project)`.
    pub(in crate::patch) pending_duplicate: Option<(Vec<String>, String)>,
    /// The "new clips" window.
    pub(super) new_clips: Option<ui::anim_data::NewClipsForm>,
    /// The bulk delete asks once more.
    pub(in crate::patch) confirm_clear: bool,
}

impl Panels {
    /// All closed but the side bar's behavior list: the user patch's start.
    pub(in crate::patch) fn with_behaviors_open() -> Self {
        Self { behaviors_open: true, ..Self::default() }
    }
}
