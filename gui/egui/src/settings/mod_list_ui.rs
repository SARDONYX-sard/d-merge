//! Mod-list UI state (filter and sort).
//!
//! These fields are persisted so that the user's filter and sort preferences
//! survive application restarts.  They are entirely presentational — no
//! business logic reads them outside the UI layer.

use serde::{Deserialize, Serialize};

use crate::mod_item::SortColumn;

/// Filter and sort state for the mod-list table.
///
/// # JSON key
/// Serialized under the `"mod_list_ui"` key in `settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ModListUiSettings {
    /// Current text in the mod-list search box.
    ///
    /// Matched case-insensitively against the column selected by
    /// [`filter_column`].  An empty string disables filtering and enables
    /// drag-and-drop reordering.
    ///
    /// [`filter_column`]: ModListUiSettings::filter_column
    #[serde(skip_serializing_if = "String::is_empty")]
    pub filter_text: String,

    /// Which column the filter text is matched against.
    ///
    /// `None` matches all text-bearing columns (id, name, site).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_column: Option<SortColumn>,

    /// Primary sort key for the mod-list table.
    ///
    /// Drag-and-drop reordering is only active when this is
    /// [`SortColumn::Priority`] and [`sort_asc`] is `true`.
    ///
    /// [`sort_asc`]: ModListUiSettings::sort_asc
    pub sort_column: SortColumn,

    /// Sort direction: `true` = ascending, `false` = descending.
    pub sort_asc: bool,
}

impl Default for ModListUiSettings {
    fn default() -> Self {
        Self {
            filter_text: String::new(),
            filter_column: None,
            sort_column: SortColumn::Priority,
            sort_asc: true,
        }
    }
}
