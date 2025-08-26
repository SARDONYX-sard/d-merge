use rayon::prelude::*;

/// Represents a single mod entry.
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ModItem {
    /// Whether the mod is enabled.
    pub enabled: bool,
    /// - vfs mode => id. e.g. `aaaa`
    /// - others => `/path/to/mod/<id>`
    pub id: String,
    /// Display name of the mod.
    pub name: String,
    /// Mod source site.
    pub site: String,
    /// Load priority.
    pub priority: usize,
}

/// Columns that can be used for sorting mods.
#[derive(PartialEq, Eq, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortColumn {
    Id,
    Name,
    Site,
    Priority,
}

/// Convert `ModInfo` into `ModItem`.
pub fn from_mod_infos(mod_infos: Vec<mod_info::ModInfo>) -> Vec<ModItem> {
    mod_infos
        .into_par_iter()
        .enumerate()
        .map(|(i, mi)| ModItem {
            enabled: true,
            id: mi.id,
            name: mi.name,
            site: mi.site,
            priority: i,
        })
        .collect()
}
