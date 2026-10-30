//! One open file: its annotation text, the original data, and per-tab UI state.

use std::{path::Path, sync::Arc};

use d_merge_gui_shared::settings::{convert::OutFormat, hkanno::HkannoTabSettings};
use hkanno_lsp::features::diagnostics::{Diagnostic, diagnostics};
use serde_hkx_for_gui::hkanno::{Hkanno, annotation_tracks_from_text, parse_as_hkanno};

use super::{editor::EditorState, preview::Preview};
use crate::app::pages::convert::to_format;

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
    pub(crate) diagnostics: Vec<Diagnostic>,

    pub(crate) editor: EditorState,
    pub(crate) preview: Preview,
}

impl HkannoTab {
    /// Reads `input_path` (.hkx or .xml) and extracts its annotations.
    ///
    /// # Errors
    /// Unreadable file, or no/multiple `hkaAnimation` in it.
    pub(crate) fn open(input_path: &str) -> Result<Self, String> {
        let bytes = std::fs::read(input_path).map_err(|e| format!("{input_path}: {e}"))?;
        let original = parse_as_hkanno(&bytes, Path::new(input_path))
            .map_err(|e| format!("{input_path}: {e}"))?
            .into_static();

        let format = infer_format(input_path);
        let mut tab = Self {
            settings: HkannoTabSettings {
                input_path: input_path.to_string(),
                output_path: infer_output_path(input_path),
                format,
                cursor: 0,
            },
            text: original.to_string(),
            original,
            bytes: Arc::new(bytes),
            dirty: false,
            diagnostics: Vec::new(),
            editor: EditorState::default(),
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
        self.preview.request();
    }

    /// The edited annotations as data. See [`document`].
    pub(crate) fn to_hkanno(&self) -> Hkanno<'_> {
        document(&self.original, &self.text)
    }

    /// Writes the edited annotations to `output_path` in `format`.
    ///
    /// # Errors
    /// Serialization or file write failure.
    pub(crate) fn save(&mut self) -> Result<(), String> {
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
        std::fs::write(output, bytes).map_err(|e| format!("{}: {e}", output.display()))?;

        self.original = hkanno.into_static();
        self.dirty = false;
        Ok(())
    }

    /// Discards the edits.
    pub(crate) fn revert(&mut self) {
        self.text = self.original.to_string();
        self.dirty = false;
        self.text_changed();
    }

    /// Changes the output format and the output extension with it.
    ///
    /// e.g., `a.modified.hkx` + `Xml` -> `a.modified.xml`
    pub(crate) fn set_format(&mut self, format: OutFormat) {
        self.settings.format = format;
        let path = Path::new(&self.settings.output_path).with_extension(format.extension());
        self.settings.output_path = path.display().to_string();
    }
}

/// `.hkx` -> `Amd64`, anything else -> `Xml`.
fn infer_format(input_path: &str) -> OutFormat {
    let is_hkx =
        Path::new(input_path).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("hkx"));
    if is_hkx { OutFormat::Amd64 } else { OutFormat::Xml }
}

/// Never overwrite the input by default.
///
/// e.g., `C:/a/attack.hkx` -> `C:/a/attack.modified.hkx`
fn infer_output_path(input_path: &str) -> String {
    let path = Path::new(input_path);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let name = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => format!("{stem}.modified.{ext}"),
        None => format!("{stem}.modified"),
    };
    path.with_file_name(name).display().to_string()
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
    fn output_path_keeps_extension() {
        // Compare as paths: the separator differs per OS (`a\attack...` on Windows).
        let output = infer_output_path("a/attack.hkx");
        assert_eq!(Path::new(&output), Path::new("a/attack.modified.hkx"));
    }
}
