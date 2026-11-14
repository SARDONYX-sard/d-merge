//! The template picker's directory tree.
//!
//! Templates are found as paths, in sorted order (see [`super::workspace::catalog`]):
//! ```text
//! meshes/actors/character/behaviors/0_master.bin
//! meshes/actors/draugr/behaviors/draugrbehavior.bin
//! ```
//! and shown with their directories folded:
//! ```text
//! ▼ meshes
//!   ▼ actors
//!     ▶ character
//!     ▼ draugr
//!       ▼ behaviors
//!         draugrbehavior.bin
//! ```

use fast_hash::HashSet;

use super::workspace::TemplateEntry;

/// Directories shallower than this start open, so the first view is not one closed `meshes` row.
const OPEN_DEPTH: usize = 2;

/// One row of the tree. Borrows from the catalog.
#[derive(Debug, Clone, Copy)]
pub struct Row<'a> {
    pub depth: u16,
    /// The last path component.
    pub name: &'a str,
    pub kind: RowKind<'a>,
}

#[derive(Debug, Clone, Copy)]
pub enum RowKind<'a> {
    /// `path`: the directory's path from the root, the key in `toggled`.
    Dir {
        path: &'a str,
        open: bool,
    },
    File(&'a TemplateEntry),
}

/// The visible rows.
///
/// `catalog` must be sorted by path, which keeps the files of a directory together, so one pass
/// that compares each path with the previous one is enough. `toggled`: the directories whose
/// open state differs from the default ([`OPEN_DEPTH`]).
pub fn rows<'a>(catalog: &'a [TemplateEntry], toggled: &HashSet<String>) -> Vec<Row<'a>> {
    let mut rows = Vec::new();
    let mut previous: Vec<&str> = Vec::new();

    for entry in catalog {
        let path = entry.meshes_path.as_str();
        // Byte offset of the end of each directory, e.g. `meshes`, `meshes/actors`.
        let dir_ends: Vec<usize> = path.match_indices('/').map(|(at, _)| at).collect();
        let dirs: Vec<&str> = path.split('/').take(dir_ends.len()).collect();
        let common = previous.iter().zip(&dirs).take_while(|(a, b)| a == b).count();

        let mut visible = true;
        for (depth, (&name, &end)) in dirs.iter().zip(&dir_ends).enumerate() {
            let dir_path = &path[..end];
            let open = (depth < OPEN_DEPTH) != toggled.contains(dir_path);
            if depth >= common {
                rows.push(Row { depth: row_depth(depth), name, kind: RowKind::Dir { path: dir_path, open } });
            }
            if !open {
                visible = false;
                break;
            }
        }
        if visible {
            let name = path.rsplit('/').next().unwrap_or(path);
            rows.push(Row { depth: row_depth(dirs.len()), name, kind: RowKind::File(entry) });
        }
        previous = dirs;
    }
    rows
}

/// Opens a closed directory or closes an open one. See [`rows`].
pub fn toggle(toggled: &mut HashSet<String>, path: &str) {
    if !toggled.remove(path) {
        toggled.insert(path.to_owned());
    }
}

/// Paths are far shallower than 65535 directories.
#[expect(clippy::cast_possible_truncation, reason = "see above")]
const fn row_depth(depth: usize) -> u16 {
    depth as u16
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn entry(path: &str) -> TemplateEntry {
        TemplateEntry { meshes_path: path.to_owned(), path: PathBuf::from(path), nemesis: true }
    }

    fn names<'a>(rows: &[Row<'a>]) -> Vec<(&'a str, u16)> {
        rows.iter().map(|row| (row.name, row.depth)).collect()
    }

    #[test]
    fn folds_directories() {
        let catalog = [entry("meshes/actors/a/x.bin"), entry("meshes/actors/a/y.bin"), entry("meshes/z.bin")];
        let mut toggled = HashSet::default();

        // `meshes` and `actors` start open, deeper ones closed.
        assert_eq!(names(&rows(&catalog, &toggled)), [("meshes", 0), ("actors", 1), ("a", 2), ("z.bin", 1)]);

        toggle(&mut toggled, "meshes/actors/a");
        assert_eq!(
            names(&rows(&catalog, &toggled)),
            [("meshes", 0), ("actors", 1), ("a", 2), ("x.bin", 3), ("y.bin", 3), ("z.bin", 1)]
        );

        toggle(&mut toggled, "meshes/actors"); // closing a parent hides everything under it
        assert_eq!(names(&rows(&catalog, &toggled)), [("meshes", 0), ("actors", 1), ("z.bin", 1)]);
    }
}
