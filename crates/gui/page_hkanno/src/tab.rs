//! One open file: its annotation text, the original data, and per-tab UI state.

use std::{path::Path, sync::Arc};

use egui_code_editor::CodeEditorState;
use gui_shared::settings::pages::{convert::OutputFormat, hkanno::HkannoTabSettings};
use hkanno_lsp::features::diagnostics::{Diagnostic, diagnostics};
use serde_hkx_for_gui::hkanno::{Format, Hkanno, annotation_tracks_from_text, parse_as_hkanno};

use crate::{language::to_editor_diagnostics, preview::Preview};

pub(crate) struct HkannoTab {
    /// Paths/format/cursor; this part is persisted in `settings.json`.
    pub(crate) settings: HkannoTabSettings,

    /// Editable annotation text (hkanno v2 format).
    pub(crate) text: String,

    /// Annotations as last loaded/saved. "Revert" restores the text from this.
    pub(crate) original: Hkanno<'static>,

    /// Raw bytes of the input file. Saving and the preview rewrite these bytes,
    /// so the file is read only once. `Arc`: shared with the preview worker thread.
    pub(crate) bytes: Arc<Vec<u8>>,

    /// `true` after an edit, `false` after load/save/revert.
    pub(crate) dirty: bool,

    /// Diagnostics of `text`; refreshed by [`Self::text_changed`].
    /// Kept in `hkanno_lsp` form for quick fixes (code actions).
    pub(crate) diagnostics: Vec<Diagnostic>,

    /// The same diagnostics converted for the editor's underlines.
    pub(crate) editor_diagnostics: Vec<egui_code_editor::Diagnostic>,

    pub(crate) editor: CodeEditorState,

    /// The saved cursor still has to be applied to the editor (once, on first show).
    pub(crate) restore_cursor: bool,
    pub(crate) preview: Preview,
}

impl HkannoTab {
    /// Reads `input_path` (.hkx or .xml) and extracts its annotations.
    ///
    /// # Errors
    /// Unreadable file, or no/multiple `hkaAnimation` in it.
    pub(crate) fn open(input_path: &str) -> Result<Self, String> {
        let bytes = std::fs::read(input_path).map_err(|e| format!("{input_path}: {e}"))?;
        let original =
            parse_as_hkanno(&bytes, Path::new(input_path)).map_err(|e| format!("{input_path}: {e}"))?.into_static();

        let format = infer_format(input_path);
        let mut tab = Self {
            settings: HkannoTabSettings {
                input_path: input_path.to_string(),
                // Saved over the input: kept first if asked (see `back_up`).
                output_path: input_path.to_owned(),
                format,
                cursor: 0,
            },
            text: original.to_string(),
            original,
            bytes: Arc::new(bytes),
            dirty: false,
            diagnostics: Vec::new(),
            editor_diagnostics: Vec::new(),
            editor: CodeEditorState::default(),
            restore_cursor: true,
            preview: Preview::default(),
        };
        tab.text_changed();
        Ok(tab)
    }

    /// Re-opens a tab saved in `settings.json`, keeping its output path, format and cursor.
    ///
    /// # Errors
    /// See [`Self::open`].
    pub(crate) fn restore(saved: &HkannoTabSettings) -> Result<Self, String> {
        let mut tab = Self::open(&saved.input_path)?;
        tab.settings = saved.clone();
        Ok(tab)
    }

    /// File name shown on the tab. e.g., `attack.hkx`
    pub(crate) fn title(&self) -> &str {
        Path::new(&self.settings.input_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&self.settings.input_path)
    }

    /// Must be called after every edit of `text`.
    pub(crate) fn text_changed(&mut self) {
        self.diagnostics = diagnostics(&self.text);
        self.editor_diagnostics = to_editor_diagnostics(&self.diagnostics);
        self.preview.request();
    }

    /// The edited annotations as data. See [`document`].
    pub(crate) fn to_hkanno(&self) -> Hkanno<'_> {
        document(&self.original, &self.text)
    }

    /// Writes the edited annotations to `output_path` in `format`. `backup`: the file there is kept
    /// first (see [`back_up`]).
    ///
    /// # Errors
    /// Serialization or file write failure, or the backup's.
    pub(crate) fn save(&mut self, backup: bool) -> Result<(), String> {
        let input = Path::new(&self.settings.input_path);
        let hkanno = self.to_hkanno();
        let bytes = hkanno
            .clone()
            .update_hkx_bytes(&self.bytes, to_format(self.settings.format), input)
            .map_err(|e| e.to_string())?;

        let output = Path::new(&self.settings.output_path);
        if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        if backup {
            back_up(output)?;
        }
        std::fs::write(output, bytes).map_err(|e| format!("{}: {e}", output.display()))?;

        self.original = hkanno.into_static();
        self.dirty = false;
        Ok(())
    }

    /// Replaces the annotation text with the contents of a `.txt` file (hkanno text format).
    /// The header values still come from the opened file (see [`document`]).
    ///
    /// # Errors
    /// The file cannot be read.
    pub(crate) fn import_txt(&mut self, path: &Path) -> Result<(), String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        self.text = txt_to_text(&bytes);
        self.dirty = true;
        self.text_changed();
        Ok(())
    }

    /// Writes the annotation text to a `.txt` file as is.
    ///
    /// # Errors
    /// The file cannot be written.
    pub(crate) fn export_txt(&self, path: &Path) -> Result<(), String> {
        std::fs::write(path, &self.text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Where "Export .txt" starts: next to the input. e.g., `C:/a/attack.hkx` -> `C:/a/attack.txt`
    pub(crate) fn default_txt_path(&self) -> std::path::PathBuf {
        Path::new(&self.settings.input_path).with_extension("txt")
    }

    /// Discards the edits.
    pub(crate) fn revert(&mut self) {
        self.text = self.original.to_string();
        self.dirty = false;
        self.text_changed();
    }

    /// Changes the output format and the output extension with it.
    ///
    /// e.g., `a.hkx` + `Xml` -> `a.xml`
    pub(crate) fn set_format(&mut self, format: OutputFormat) {
        self.settings.format = format;
        let path = Path::new(&self.settings.output_path).with_extension(format.extension());
        self.settings.output_path = path.display().to_string();
    }

    /// The `TextEdit` id of this tab's editor (e.g., to move its cursor from the problems list).
    pub(crate) fn editor_id(&self) -> egui::Id {
        egui_code_editor::editor_id(self.settings.input_path.as_str())
    }
}

/// Settings enum -> serializer enum.
const fn to_format(format: OutputFormat) -> Format {
    match format {
        OutputFormat::Amd64 => Format::Amd64,
        OutputFormat::Win32 => Format::Win32,
        OutputFormat::Xml => Format::Xml,
        OutputFormat::Json => Format::Json,
    }
}

/// `.hkx` -> `Amd64`, anything else -> `Xml`.
fn infer_format(input_path: &str) -> OutputFormat {
    let is_hkx = Path::new(input_path).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("hkx"));
    if is_hkx { OutputFormat::Amd64 } else { OutputFormat::Xml }
}

/// Where the file at `output` is kept before it is overwritten.
///
/// e.g., `C:/a/attack.hkx` -> `C:/a/attack.hkx.bak`
fn backup_path(output: &Path) -> std::path::PathBuf {
    let mut name = output.as_os_str().to_owned();
    name.push(".bak");
    name.into()
}

/// Keeps the file at `output` (if there is one) as [`backup_path`], before it is overwritten. Once:
/// a backup there already is the file as it was before the first save, and stays.
///
/// # Errors
/// The copy failed.
fn back_up(output: &Path) -> Result<(), String> {
    let backup = backup_path(output);
    if !output.exists() || backup.exists() {
        return Ok(());
    }
    std::fs::copy(output, &backup).map(drop).map_err(|e| format!("{}: {e}", backup.display()))
}

/// `.txt` bytes -> editor text: UTF-8 (lossy) without a BOM, `\r\n` -> `\n` (the editor's line end).
fn txt_to_text(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.strip_prefix('\u{feff}').unwrap_or(&text).replace("\r\n", "\n")
}

/// Builds the annotation data for `text`: tracks from the text, header values
/// (`ptr`, `numOriginalFrames`, `duration`) from the `original` file (they are not editable).
///
/// A free function (not a method) so it can be called while other tab fields are borrowed mutably.
pub(crate) fn document<'a>(original: &'a Hkanno<'static>, text: &'a str) -> Hkanno<'a> {
    Hkanno {
        ptr: std::borrow::Cow::Borrowed(&original.ptr),
        num_original_frames: original.num_original_frames,
        duration: original.duration,
        annotation_tracks: annotation_tracks_from_text(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_backup_is_beside_the_file() {
        assert_eq!(backup_path(Path::new("a/attack.hkx")), Path::new("a/attack.hkx.bak"));
    }

    /// The first save keeps the file; the next ones do not overwrite that.
    #[test]
    fn the_file_is_backed_up_once() {
        let dir = std::env::temp_dir().join(format!("hkanno_backup_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let output = dir.join("attack.hkx");
        std::fs::write(&output, "original").unwrap();

        back_up(&output).unwrap();
        std::fs::write(&output, "first save").unwrap();
        back_up(&output).unwrap();
        let kept = std::fs::read_to_string(backup_path(&output)).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(kept, "original");
    }

    #[test]
    fn txt_is_normalized() {
        assert_eq!(txt_to_text(b"\xEF\xBB\xBF0.1 a\r\n0.2 b"), "0.1 a\n0.2 b");
    }
}

/// Replaces the chars `columns` of line `line` (a quick fix).
///
/// e.g., line 1, columns `0..3`, `new`: `a\nold b` -> `a\nnew b`
pub(crate) fn replace_columns(text: &mut String, line: usize, columns: std::ops::Range<usize>, new_text: &str) {
    use egui_code_editor::text_pos::{LinePos, char_to_byte};
    let at = |column| LinePos { line, column }.to_char_index(text);
    let (start, end) = (char_to_byte(text, at(columns.start)), char_to_byte(text, at(columns.end)));
    text.replace_range(start..end, new_text);
}

#[cfg(test)]
mod replace_tests {
    use super::*;

    #[test]
    fn replaces_columns_of_a_line() {
        let mut text = "first\nold b".to_owned();
        replace_columns(&mut text, 1, 0..3, "new");
        assert_eq!(text, "first\nnew b");
    }
}
