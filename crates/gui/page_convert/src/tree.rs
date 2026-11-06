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

use fast_hash::{HashMap, HashSet};
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

    /// Directory path -> `(files below, of which checked)`, for its checkbox. Counting walks all
    /// the files below, too slow for every row of every frame, so this only runs after a change.
    pub(super) counts: Option<HashMap<String, (u32, u32)>>,
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
        self.counts = None;
        self.expanded = self.roots.iter().map(|root| root.path.clone()).collect();
        Ok(())
    }

    /// The checks changed: the directory counts are out of date.
    pub(super) fn invalidate_counts(&mut self) {
        self.counts = None;
    }

    /// See [`Self::counts`].
    pub(super) fn counts(&mut self) -> &HashMap<String, (u32, u32)> {
        let Self { roots, checked, counts, .. } = self;
        counts.get_or_insert_with(|| {
            let mut counts = HashMap::default();
            for root in roots.iter() {
                add_dir_counts(root, checked, &mut counts);
            }
            counts
        })
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
        self.counts = None;
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

/// The rows to show: `entry` and, if it is expanded, what is below it, as `(entry, depth)`.
pub(super) fn visible_rows<'a>(roots: &'a [DirEntry], expanded: &HashSet<String>) -> Vec<(&'a DirEntry, usize)> {
    fn push<'a>(entry: &'a DirEntry, depth: usize, expanded: &HashSet<String>, rows: &mut Vec<(&'a DirEntry, usize)>) {
        rows.push((entry, depth));
        if expanded.contains(&entry.path) {
            for child in entry.children.iter().flatten() {
                push(child, depth + 1, expanded, rows);
            }
        }
    }
    let mut rows = Vec::new();
    for root in roots {
        push(root, 0, expanded, &mut rows);
    }
    rows
}

/// Adds `key`, or removes it if it is there. e.g., opening or closing a directory.
pub(super) fn toggle(set: &mut HashSet<String>, key: &str) {
    if !set.remove(key) {
        set.insert(key.to_owned());
    }
}

/// Records `(files, checked)` of `entry` and every directory below it, and returns those of `entry`:
/// each directory adds up its children, so the whole tree is one pass.
fn add_dir_counts(entry: &DirEntry, checked: &HashSet<String>, counts: &mut HashMap<String, (u32, u32)>) -> (u32, u32) {
    let Some(children) = &entry.children else {
        return (1, u32::from(checked.contains(&entry.path)));
    };
    let (mut total, mut on) = (0, 0);
    for child in children {
        let (child_total, child_on) = add_dir_counts(child, checked, counts);
        total += child_total;
        on += child_on;
    }
    counts.insert(entry.path.clone(), (total, on));
    (total, on)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str) -> DirEntry {
        DirEntry { name: path.to_owned(), path: path.to_owned(), children: None }
    }

    fn dir(path: &str, children: Vec<DirEntry>) -> DirEntry {
        DirEntry { name: path.to_owned(), path: path.to_owned(), children: Some(children) }
    }

    fn sample() -> TreeView {
        let roots = vec![dir("root", vec![file("a"), dir("sub", vec![file("b"), file("c")])])];
        TreeView { roots, ..TreeView::default() }
    }

    #[test]
    fn directories_add_up_their_files() {
        let mut tree = sample();
        tree.checked.insert("b".to_owned());
        assert_eq!(tree.counts().get("root"), Some(&(3, 1)));
        assert_eq!(tree.counts().get("sub"), Some(&(2, 1)));

        // Checking a file is seen after the invalidation.
        tree.checked.insert("a".to_owned());
        assert_eq!(tree.counts().get("root"), Some(&(3, 1)));
        tree.invalidate_counts();
        assert_eq!(tree.counts().get("root"), Some(&(3, 2)));
    }

    #[test]
    fn only_expanded_directories_show_their_rows() {
        let tree = sample();
        let names = |expanded: &[&str]| -> Vec<(String, usize)> {
            let expanded: HashSet<String> = expanded.iter().map(|path| (*path).to_owned()).collect();
            visible_rows(&tree.roots, &expanded).into_iter().map(|(entry, depth)| (entry.path.clone(), depth)).collect()
        };
        assert_eq!(names(&[]), [("root".to_owned(), 0)]);
        assert_eq!(names(&["root"]), [("root".to_owned(), 0), ("a".to_owned(), 1), ("sub".to_owned(), 1)]);
    }

    #[test]
    fn select_all_toggles_and_lists_in_tree_order() {
        let mut tree = sample();
        tree.toggle_select_all();
        assert_eq!(tree.checked_files(), ["a", "b", "c"]);
        tree.toggle_select_all();
        assert!(tree.checked_files().is_empty());
    }
}
