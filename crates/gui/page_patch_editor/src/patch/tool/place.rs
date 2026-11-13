//! Which work is open, and where its files are.
//!
//! ```text
//! mod_code      "mymod"                       the work's name: its folder in the cache
//! loaded_for    Some("mymod")                 the work in memory now; None: the start screen
//! cache_root    <patch output>/.d_merge/...   where every work is kept (`workspace::cache_root_of`)
//! output        D:/mods/mymod/                where it is exported to
//! mod_dir       D:/mods/Nemesis mymod/        the mod it was read from; empty: made here
//! template_dir  <skyrim data>/meshes/...      the templates (`.bin` / `.xml`)
//! patch_output  the patch page's output, the cache follows it
//! ```

use std::path::PathBuf;

/// See the module docs.
#[derive(Debug, Default)]
pub(in crate::patch) struct WorkPlace {
    /// The code of the work: typed in, or of the work opened.
    pub(in crate::patch) mod_code: String,
    /// The code of the work in memory (`None`: no work is open).
    pub(in crate::patch) loaded_for: Option<String>,
    /// Where the works are: see [`crate::patch::workspace::cache_root_of`].
    pub(in crate::patch) cache_root: PathBuf,
    /// Where this work is exported to.
    pub(in crate::patch) output: String,
    /// The root of the mod the work was read from. Empty: it was made here.
    pub(in crate::patch) mod_dir: String,
    /// `settings.behavior.template_dir` (`.bin` or `.xml` templates under `meshes/`).
    pub(in crate::patch) template_dir: String,
    /// The patch page's output directory, see [`crate::patch::PatchPaths`].
    pub(in crate::patch) patch_output: String,
}

impl WorkPlace {
    /// Is a work open?
    pub(in crate::patch) const fn is_open(&self) -> bool {
        self.loaded_for.is_some()
    }
}
