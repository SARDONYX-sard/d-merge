//! Checkbox tree of the files under the selected root directories (the "Tree" selection mode).
//!
//! # Check state model
//! Only **files** are stored in `checked`. A directory's checkbox is derived from its files:
//! ```text
//! all files checked   -> [✔]
//! some files checked  -> [-]   (indeterminate)
//! no file checked     -> [ ]
//! ```
//! Clicking a directory checks/unchecks every file below it.
//! This avoids keeping directory flags in sync with their children.

use fast_hash::HashSet;
use serde_hkx_for_gui::DirEntry;

/// Loaded tree + user selection. Not persisted (a tree may contain thousands of files).
#[derive(Default)]
pub(crate) struct TreeView {
    /// One entry per root directory.
    pub(super) roots: Vec<DirEntry>,

    /// Roots `roots` was loaded from; reload when the settings differ.
    loaded_for: Option<Vec<String>>,

    /// Checked **file** paths.
    pub(super) checked: HashSet<String>,

    /// Expanded directory paths.
    pub(super) expanded: HashSet<String>,
}

impl TreeView {
    /// Loads the tree if `roots` changed since the last load.
    ///
    /// # Errors
    /// Returns the walker errors joined by newlines. The previous tree is kept in that case.
    pub(crate) fn ensure_loaded(&mut self, roots: &[String]) -> Result<(), String> {
        if self.loaded_for.as_deref() == Some(roots) {
            return Ok(());
        }
        // Remember the attempt even on failure, so a broken path is not re-walked every frame.
        self.loaded_for = Some(roots.to_vec());

        // NOTE: Synchronous on purpose: it runs only when the roots change, and the walker is parallel.
        let entries = serde_hkx_for_gui::load_dir_node(roots.to_vec())
            .map_err(|errors| errors.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"))?;

        self.roots = entries;
        self.checked.clear();
        self.expanded = self.roots.iter().map(|root| root.path.clone()).collect();
        Ok(())
    }

    /// Drops the tree (e.g., "All Clear").
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    /// Checked files in tree order. These are the conversion inputs.
    pub(crate) fn checked_files(&self) -> Vec<String> {
        let mut out = Vec::new();
        for root in &self.roots {
            for_each_file(root, &mut |file| {
                if self.checked.contains(&file.path) {
                    out.push(file.path.clone());
                }
            });
        }
        out
    }

    pub(crate) fn checked_count(&self) -> usize {
        self.checked.len()
    }

    pub(crate) fn has_checked(&self) -> bool {
        !self.checked.is_empty()
    }

    pub(crate) fn is_any_expanded(&self) -> bool {
        !self.expanded.is_empty()
    }

    /// Checks every file, or unchecks all when something is already checked.
    pub(crate) fn toggle_select_all(&mut self) {
        if self.checked.is_empty() {
            for root in &self.roots {
                for_each_file(root, &mut |file| {
                    self.checked.insert(file.path.clone());
                });
            }
        } else {
            self.checked.clear();
        }
    }

    /// Expands every directory, or collapses all when something is expanded.
    pub(crate) fn toggle_expand_all(&mut self) {
        if self.expanded.is_empty() {
            for root in &self.roots {
                for_each_dir(root, &mut |dir| {
                    self.expanded.insert(dir.path.clone());
                });
            }
        } else {
            self.expanded.clear();
        }
    }
}

/// Visits every file (leaf) below `entry`, including `entry` itself if it is a file.
pub(super) fn for_each_file(entry: &DirEntry, f: &mut impl FnMut(&DirEntry)) {
    match &entry.children {
        None => f(entry),
        Some(children) => children.iter().for_each(|child| for_each_file(child, f)),
    }
}

/// Visits every directory below `entry`, including `entry` itself.
pub(super) fn for_each_dir(entry: &DirEntry, f: &mut impl FnMut(&DirEntry)) {
    if let Some(children) = &entry.children {
        f(entry);
        for child in children {
            for_each_dir(child, f);
        }
    }
}

/// Adds `key`, or removes it if it is there. e.g., opening or closing a directory.
pub(super) fn toggle(set: &mut HashSet<String>, key: &str) {
    if !set.remove(key) {
        set.insert(key.to_owned());
    }
}

/// `(files below entry, of which checked)`. A directory's checkbox is derived from these.
pub(super) fn count_files(entry: &DirEntry, checked: &HashSet<String>) -> (usize, usize) {
    let (mut total, mut on) = (0, 0);
    for_each_file(entry, &mut |file| {
        total += 1;
        on += usize::from(checked.contains(&file.path));
    });
    (total, on)
}
