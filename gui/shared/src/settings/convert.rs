//! Convert page (hkx <-> xml/json) settings.
//!
//! # Backward compatibility
//! Added in v2.9. Every field has a default, so older `settings.json` files without
//! the `"convert"` key still load (see `#[serde(default)]` on [`super::Settings::convert`]).

/// How the user picks the inputs to convert.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SelectionType {
    /// Pick individual files.
    Files,
    /// Pick directories; everything inside is converted.
    Dir,
    /// Pick a root directory, then check files in a tree view.
    #[default]
    Tree,
}

/// Output format of the converter.
///
/// `yaml` is intentionally absent: it cannot be converted back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutFormat {
    /// 64-bit hkx (Skyrim SE/AE).
    #[default]
    Amd64,
    /// 32-bit hkx (Skyrim LE).
    Win32,
    /// Havok XML.
    Xml,
    /// JSON (for diffing / scripting).
    Json,
}

impl OutFormat {
    /// All variants in UI order.
    pub const ALL: [Self; 4] = [Self::Amd64, Self::Win32, Self::Xml, Self::Json];

    /// Label shown in the UI. e.g., `Amd64`
    pub const fn label(self) -> &'static str {
        match self {
            Self::Amd64 => "Amd64",
            Self::Win32 => "Win32",
            Self::Xml => "XML",
            Self::Json => "Json",
        }
    }

    /// File extension written for this format. e.g., `hkx`
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Amd64 | Self::Win32 => "hkx",
            Self::Xml => "xml",
            Self::Json => "json",
        }
    }
}

/// Persisted state of the Convert page.
///
/// # JSON key
/// Serialized under the `"convert"` key in `settings.json`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ConvertSettings {
    /// Current selection mode.
    pub selection_type: SelectionType,

    /// Inputs for [`SelectionType::Files`].
    pub selected_files: Vec<String>,

    /// Inputs for [`SelectionType::Dir`].
    pub selected_dirs: Vec<String>,

    /// Root directories shown by [`SelectionType::Tree`].
    ///
    /// The checked items themselves are not persisted: a tree can contain thousands of files.
    pub tree_roots: Vec<String>,

    /// Output directory. Empty = next to each input.
    pub output: String,

    /// Output format.
    pub format: OutFormat,
}
