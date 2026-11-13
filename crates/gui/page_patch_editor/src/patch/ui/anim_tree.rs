//! The rows of the anim data list: per actor as its files are, then per kind of file.
//!
//! ```text
//! ▼ DefaultMale~1 (3) ●                  <- animationdatasinglefile/<actor>~1/...
//!     ▶ Header (1)
//!     ▼ Clips (1) ●                      <- `Idle~mymod$0.txt`
//!         ● Idle~mymod$0.txt
//!     ▶ Motions (1) ●                    <- `mymod$0.txt`
//! ▶ DraugrProject~1 (4)
//! ```
//! `animationsetdatasinglefile` has a folder per actor too, e.g. `DefaultMaleData~DefaultMale` with
//! its 374 anim sets, so a flat list would hide what belongs together.
//!
//! What is open is the user's: a row opens and closes with a click. A newly selected file opens the way
//! to it once ([`reveal`]), and can then be closed like any other. A filter shows all that matches.

use fast_hash::HashSet;

use crate::patch::{
    anim::patch_file::{AdsfFile, AsdsfFile},
    tree::Node,
};

/// What is open of the tree, kept between frames.
#[derive(Debug, Default)]
pub(in crate::patch) struct AnimTreeState {
    /// The actors and groups open, e.g. `DefaultMale~1`, `DefaultMale~1/Clip`.
    pub(in crate::patch) open: HashSet<String>,
    /// `Expand all` was clicked: every folder opens on the next frame.
    pub(in crate::patch) expand_all: bool,
    /// The row the keyboard is on (`j` / `k` ...), while the explorer has it. See [`apply_key`].
    pub(in crate::patch) cursor: usize,
}
/// A kind of file, in the order they are shown. See [`AdsfFile`], [`AsdsfFile`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::patch) enum FileKind {
    Header,
    Clip,
    Motion,
    /// An anim set of the template.
    Set,
    /// An anim set the mod adds.
    NewSet,
}

impl FileKind {
    const ALL: [Self; 5] = [Self::Header, Self::Clip, Self::Motion, Self::Set, Self::NewSet];

    /// `is_adsf`: which of the two templates the file `id` is of.
    pub(super) fn of(is_adsf: bool, id: &str) -> Self {
        if is_adsf {
            match AdsfFile::of(id) {
                AdsfFile::ProjectNamesHeader | AdsfFile::AnimHeader => Self::Header,
                AdsfFile::AddAnim | AdsfFile::IndexedAnim => Self::Clip,
                AdsfFile::AddMotion | AdsfFile::IndexedMotion => Self::Motion,
            }
        } else {
            match AsdsfFile::of(id) {
                AsdsfFile::ProjectsHeader | AsdsfFile::SetsHeader => Self::Header,
                AsdsfFile::EditSet => Self::Set,
                AsdsfFile::AddSet => Self::NewSet,
            }
        }
    }

    /// What identifies its row in the set of open rows, e.g. `DefaultMale~1/Clip`.
    fn key(self, folder: &str) -> String {
        format!("{folder}/{self:?}")
    }
}

/// A file of the list.
#[derive(Debug, Clone, Copy)]
pub(super) struct AnimFile<'a> {
    pub(super) node: Node,
    /// `folder/file`
    pub(super) id: &'a str,
    pub(super) changed: bool,
}

/// One row of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AnimRow<'a> {
    Folder {
        name: &'a str,
        files: usize,
        /// A file in it has changes.
        changed: bool,
        open: bool,
    },
    Group {
        kind: FileKind,
        /// What opens and closes it.
        key: String,
        files: usize,
        changed: bool,
        open: bool,
    },
    File {
        node: Node,
        /// Without the folder, e.g. `Idle~mymod$0.txt`.
        name: &'a str,
    },
}

impl AnimRow<'_> {
    /// What the keys need of it: a folder at depth 0, a group of files 1, a file 2.
    pub(super) const fn nav(&self) -> egui_tree::NavRow {
        let (depth, open) = match self {
            Self::Folder { open, .. } => (0, Some(*open)),
            Self::Group { open, .. } => (1, Some(*open)),
            Self::File { .. } => (2, None),
        };
        egui_tree::NavRow { depth, open }
    }

    /// What opens and closes it: a folder's name, a group's key; a file: `""`.
    pub(super) fn open_key(&self) -> &str {
        match self {
            Self::Folder { name, .. } => name,
            Self::Group { key, .. } => key,
            Self::File { .. } => "",
        }
    }
}

/// `DefaultMale~1/Idle~m$0.txt` -> `DefaultMale~1`, `Idle~m$0.txt`
fn split_id(id: &str) -> (&str, &str) {
    id.split_once('/').unwrap_or(("", id))
}

/// The rows to show. `files`: sorted, so that a folder's files are together.
///
/// `filtering`: everything is open, to show what matches.
pub(super) fn rows<'a>(
    files: &[AnimFile<'a>],
    open: &HashSet<String>,
    is_adsf: bool,
    filtering: bool,
) -> Vec<AnimRow<'a>> {
    let mut rows = Vec::new();
    for group in files.chunk_by(|a, b| split_id(a.id).0 == split_id(b.id).0) {
        let name = split_id(group[0].id).0;
        let is_open = filtering || open.contains(name);
        let changed = group.iter().any(|file| file.changed);
        rows.push(AnimRow::Folder { name, files: group.len(), changed, open: is_open });
        if is_open {
            push_groups(&mut rows, name, group, open, (is_adsf, filtering));
        }
    }
    rows
}

/// The kinds of files of an open folder, and the files of the open ones.
fn push_groups<'a>(
    rows: &mut Vec<AnimRow<'a>>,
    folder: &str,
    files: &[AnimFile<'a>],
    open: &HashSet<String>,
    (is_adsf, filtering): (bool, bool),
) {
    for kind in FileKind::ALL {
        let of_kind = || files.iter().filter(|file| FileKind::of(is_adsf, file.id) == kind);
        let count = of_kind().count();
        if count == 0 {
            continue;
        }
        let key = kind.key(folder);
        let is_open = filtering || open.contains(&key);
        rows.push(AnimRow::Group {
            kind,
            key,
            files: count,
            changed: of_kind().any(|file| file.changed),
            open: is_open,
        });
        if is_open {
            rows.extend(of_kind().map(|file| AnimRow::File { node: file.node, name: split_id(file.id).1 }));
        }
    }
}

/// Opens the way to the file `id`, which was just selected.
pub(super) fn reveal(open: &mut HashSet<String>, is_adsf: bool, id: &str) {
    let folder = split_id(id).0;
    open.insert(FileKind::of(is_adsf, id).key(folder));
    open.insert(folder.to_owned());
}

/// Opens a folder and each kind of file in it.
pub(super) fn expand(open: &mut HashSet<String>, folder: &str) {
    open.insert(folder.to_owned());
    open.extend(FileKind::ALL.map(|kind| kind.key(folder)));
}

/// Closes a folder and each kind of file in it, so that it opens as it starts.
pub(super) fn collapse(open: &mut HashSet<String>, folder: &str) {
    open.remove(folder);
    for kind in FileKind::ALL {
        open.remove(&kind.key(folder));
    }
}

/// `Expand all` above the list: opens every folder there is.
pub(super) fn expand_all(open: &mut HashSet<String>, files: &[AnimFile<'_>]) {
    for group in files.chunk_by(|a, b| split_id(a.id).0 == split_id(b.id).0) {
        expand(open, split_id(group[0].id).0);
    }
}

/// A click on a folder or a group.
pub(super) fn toggle(open: &mut HashSet<String>, key: &str) {
    if !open.remove(key) {
        open.insert(key.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILES: [AnimFile<'_>; 4] = [
        AnimFile { node: 0, id: "A~1/$header$.txt", changed: false },
        AnimFile { node: 1, id: "A~1/Idle~m$0.txt", changed: true },
        AnimFile { node: 2, id: "A~1/m$0.txt", changed: false },
        AnimFile { node: 3, id: "B~1/z~348.txt", changed: false },
    ];

    fn count_files(rows: &[AnimRow<'_>]) -> usize {
        rows.iter().filter(|row| matches!(row, AnimRow::File { .. })).count()
    }

    #[test]
    fn everything_starts_closed() {
        let rows = rows(&FILES, &HashSet::default(), true, false);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], AnimRow::Folder { name: "A~1", files: 3, changed: true, open: false });
    }

    #[test]
    fn files_are_split_by_kind() {
        let open = HashSet::from_iter(["A~1".to_owned()]);
        let list = rows(&FILES, &open, true, false);
        let kinds: Vec<FileKind> = list
            .iter()
            .filter_map(|row| if let AnimRow::Group { kind, .. } = row { Some(*kind) } else { None })
            .collect();
        assert_eq!(kinds, [FileKind::Header, FileKind::Clip, FileKind::Motion]);
        assert_eq!(count_files(&list), 0, "the groups are closed");

        let open = HashSet::from_iter(["A~1".to_owned(), "A~1/Clip".to_owned()]);
        let list = rows(&FILES, &open, true, false);
        assert!(list.contains(&AnimRow::File { node: 1, name: "Idle~m$0.txt" }));
        assert_eq!(count_files(&list), 1);
    }

    #[test]
    fn anim_sets_have_their_own_kinds() {
        assert_eq!(FileKind::of(false, "D~D/$header$.txt"), FileKind::Header);
        assert_eq!(FileKind::of(false, "D~D/_MTSolo.txt"), FileKind::Set);
        assert_eq!(FileKind::of(false, "D~D/tkuc$New.txt"), FileKind::NewSet);
    }

    /// The selected file opens the way once. Closing it afterwards must stay closed, as nothing here
    /// forces it open: this is what the list did wrong before.
    #[test]
    fn a_folder_with_the_selected_file_can_be_closed() {
        let mut open = HashSet::default();
        reveal(&mut open, true, "A~1/Idle~m$0.txt");
        assert_eq!(count_files(&rows(&FILES, &open, true, false)), 1);

        toggle(&mut open, "A~1");
        assert_eq!(count_files(&rows(&FILES, &open, true, false)), 0);
        toggle(&mut open, "A~1");
        assert_eq!(count_files(&rows(&FILES, &open, true, false)), 1);
    }

    #[test]
    fn a_folder_opens_and_closes_with_all_in_it() {
        let mut open = HashSet::default();
        expand(&mut open, "A~1");
        assert_eq!(count_files(&rows(&FILES, &open, true, false)), 3);
        assert!(
            !rows(&FILES, &open, true, false).iter().any(|row| matches!(row, AnimRow::File { name: "z~348.txt", .. }))
        );

        expand_all(&mut open, &FILES);
        assert_eq!(count_files(&rows(&FILES, &open, true, false)), 4);

        // Opened again, it is not as it was left: all of it was closed.
        collapse(&mut open, "A~1");
        assert_eq!(count_files(&rows(&FILES, &open, true, false)), 1);
        toggle(&mut open, "A~1");
        assert_eq!(count_files(&rows(&FILES, &open, true, false)), 1, "its kinds are closed");

        // `Collapse all`.
        open.clear();
        assert_eq!(rows(&FILES, &open, true, false).len(), 2);
    }

    #[test]
    fn a_filter_shows_all_that_matches() {
        assert_eq!(count_files(&rows(&FILES, &HashSet::default(), true, true)), 4);
    }
}
