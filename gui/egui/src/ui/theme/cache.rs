//! Shared, cache-backed theme preset store.
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use d_merge_gui_shared::settings::ui::theme::{Error, ThemePreset, load_preset, save_preset};
use dashmap::DashMap;
use parking_lot::RwLock;

/// Thread-safe, lazily-loaded preset cache.
///
/// Clone the `Arc` to share the same cache across subsystems; all clones
/// observe the same underlying state.
#[derive(Debug)]
pub(crate) struct ThemeCache {
    /// Absolute path to the `themes/` directory.
    pub(crate) dir: PathBuf,

    /// Sorted list of preset names derived from `*.json` filenames.
    ///
    /// `parking_lot::RwLock` so that `names()` (hot path) is a cheap
    /// read-lock while `reload_dir()` (rare) takes a write-lock.
    names: RwLock<Vec<String>>,

    /// Lazily-loaded preset data.
    ///
    /// `DashMap` gives per-key sharding: concurrent reads on different keys
    /// never block each other.
    entries: DashMap<String, ThemePreset>,
}

impl ThemeCache {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Create a new cache rooted at `dir` and eagerly scan the directory for
    /// preset names (but *not* their contents).
    pub(crate) fn new(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        let names = scan_names(&dir);
        Self { dir, names: RwLock::new(names), entries: DashMap::new() }
    }

    // ── Read ──────────────────────────────────────────────────────────────────

    /// Sorted list of all available preset names.
    ///
    /// This is a cheap clone of the in-memory list; no disk I/O.
    pub(crate) fn names(&self) -> Vec<String> {
        self.names.read().clone()
    }

    /// Return a preset by name, loading it from disk on first access.
    ///
    /// # Errors
    /// - [`Error::NotFound`] when the name is not in the directory
    ///   listing (call [`reload_dir`](Self::reload_dir) if the directory
    ///   changed since construction).
    /// - [`Error::Io`] / [`Error::Json`] on disk or parse
    ///   failures.
    pub(crate) fn get(&self, name: &str) -> Result<ThemePreset, Error> {
        // Fast path: already cached.
        if let Some(entry) = self.entries.get(name) {
            return Ok(entry.clone());
        }

        // Verify that the name is known before hitting disk.
        {
            let names = self.names.read();
            if !names.iter().any(|n| n == name) {
                return Err(Error::Io {
                    path: name.into(),
                    source: std::io::Error::new(ErrorKind::NotFound, ""),
                });
            }
        }

        // Slow path: load from disk and insert.
        let path = self.path_for(name);
        let preset = load_preset(&path)?;
        self.entries.insert(name.to_owned(), preset.clone());
        Ok(preset)
    }

    // ── Write ─────────────────────────────────────────────────────────────────

    /// Serialize `preset` to `<dir>/<preset.name>.json` and upsert the cache.
    ///
    /// Creates the themes directory if it does not exist.
    pub(crate) fn save(&self, preset: ThemePreset) -> Result<PathBuf, Error> {
        let name = preset.name.clone();
        let path = self.path_for(&name);

        save_preset(&preset, &path)?;

        // Upsert in-memory state.
        self.entries.insert(name.clone(), preset);
        {
            let mut names = self.names.write();
            if !names.contains(&name) {
                names.push(name);
                names.sort_unstable();
            }
        }

        Ok(path)
    }

    // ── Directory reload ──────────────────────────────────────────────────────

    /// Re_scan the themes directory: clear the name list and evict all cached
    /// presets so that subsequent `get` calls re-read from disk.
    ///
    /// Safe to call from any thread; all `Arc` clones will immediately observe
    /// the refreshed state.
    pub(crate) fn reload_dir(&self) {
        let fresh = scan_names(&self.dir);
        *self.names.write() = fresh;
        self.entries.clear();
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn path_for(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.json"))
    }
}

// ─── Free helpers ─────────────────────────────────────────────────────────────

/// Scan `dir` and return a sorted list of JSON stems.
fn scan_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if path.extension()?.to_str()? == "json" {
                Some(path.file_stem()?.to_str()?.to_owned())
            } else {
                None
            }
        })
        .collect();

    names.sort_unstable();
    names
}
