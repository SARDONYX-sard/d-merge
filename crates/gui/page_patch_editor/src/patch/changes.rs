//! What the work changes, for the side bar's Changes view: like VS Code's Source Control, the files
//! changed, each marked `M` (modified) or `A` (added), under its behavior. Drawn by
//! [`super::ui::side_bar`].
//!
//! ```text
//! ▼ 0_master (2)
//!     M #0106  hkbModifierList  GetUpModList
//!     A #mymod$0  hkbClipGenerator
//! ```

use super::workspace::Workspace;

/// One changed object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Change<'a> {
    /// In `Workspace::behaviors`.
    pub(super) behavior: usize,
    /// e.g. `#0106`
    pub(super) id: &'a str,
    /// e.g. `hkbModifierList`
    pub(super) class: &'a str,
    /// The template's `name` field, e.g. `GetUpModList` (`""` for new objects).
    pub(super) name: &'a str,
    /// `A`: new. Else `M`: an edited template object.
    pub(super) added: bool,
}

impl Change<'_> {
    /// `A` or `M`, as Source Control marks them.
    pub(super) const fn mark(&self) -> &'static str {
        if self.added { "A" } else { "M" }
    }
}

/// Every change of the work: by behavior, the new objects first (as the lists show them).
pub(super) fn changes(workspace: &Workspace) -> Vec<Change<'_>> {
    let mut found = Vec::new();
    for (behavior, of) in workspace.behaviors.iter().enumerate() {
        let changed =
            of.texts().filter(|object| object.is_new || object.is_edited).filter(|object| !of.is_ref(object.id));
        found.extend(changed.map(|object| Change {
            behavior,
            id: object.id,
            class: object.class,
            name: object.name,
            added: object.is_new,
        }));
    }
    found
}

/// How many changes there are: the badge of the view's icon. Without going through the objects.
pub(super) fn count(workspace: &Workspace) -> usize {
    workspace.behaviors.iter().map(|behavior| behavior.edits.len() + behavior.added.len()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::workspace::{ADSF, Behavior};

    #[test]
    fn the_changes_are_listed_by_behavior_with_their_mark() {
        let mut workspace = Workspace::default();
        for files in [&["M~1/a.txt"][..], &[], &["F~1/b.txt", "F~1/c.txt"]] {
            let mut behavior = Behavior::open(&std::path::Path::new("meshes").join(format!("{ADSF}.bin"))).unwrap();
            for file in files {
                behavior.added.insert((*file).to_owned(), String::new());
            }
            workspace.behaviors.push(behavior);
        }
        let found = changes(&workspace);
        let listed: Vec<(usize, &str, &str)> =
            found.iter().map(|change| (change.behavior, change.id, change.mark())).collect();
        assert_eq!(listed, [(0, "M~1/a.txt", "A"), (2, "F~1/b.txt", "A"), (2, "F~1/c.txt", "A")]);
        assert_eq!(count(&workspace), 3);
    }
}
