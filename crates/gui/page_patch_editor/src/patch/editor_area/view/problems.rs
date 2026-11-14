//! The Problems panel of the patch editor: the shared one of the editor
//! ([`egui_code_editor::problems`]), with a group per object of the work. Found by
//! [`crate::patch::problems`].
//!
//! ```text
//! Problems  ✖ 1  ⚠ 1
//! ▼ #0106  0_master  (2)
//!     ✖ Unknown field names: "typoField".   Ln 3, Col 3     <- a click goes to the line
//! ```

use std::borrow::Cow;

use egui_code_editor::{ProblemGroup, ProblemsCursor, ProblemsPanel};
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{problems::ObjectProblems, workspace::Behavior};

/// Draws the panel. Returns where a click goes: `(behavior, object, line)`.
pub(super) fn problems_panel(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    groups: &[ObjectProblems],
    behaviors: &[Behavior],
    cursor: &mut ProblemsCursor,
    keys: &[egui_code_editor::ProblemsKey],
) -> Option<(usize, String, usize)> {
    let shown: Vec<ProblemGroup<'_>> = groups
        .iter()
        .map(|group| {
            let behavior = behaviors.get(group.behavior).map_or("", |behavior| behavior.name.as_str());
            ProblemGroup { title: Cow::Owned(format!("{}  {behavior}", group.id)), problems: &group.problems }
        })
        .collect();
    let (at, index) = ProblemsPanel::new("nemesis_problems", i18n.t(I18nKey::HkannoProblemsLabel))
        .empty_text(i18n.t(I18nKey::ProblemsNone))
        .cursor(cursor)
        .keys(keys)
        .show(ui, &shown, |_, _, _| {})?;
    let group = &groups[at];
    Some((group.behavior, group.id.clone(), group.problems[index].line))
}
