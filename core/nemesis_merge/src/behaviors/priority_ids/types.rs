use std::collections::HashMap;

/// Path & Priority map
///
/// - key:
///   - Nemesis => path until mod_code(e.g. `<skyrim data dir>/meshes/Nemesis_Engine/mod/slide`)
///   - FNIS => path until namespace(e.g. `<skyrim_data_dir>/meshes/actors/character/animations/FNISMod`)
/// - value: priority
pub type PriorityMap = HashMap<String, usize>;

/// Nemesis & FNIS patch path
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PatchMaps {
    /// Nemesis patch path
    /// - key: path until mod_code(e.g. `<skyrim_data_dir>/meshes/Nemesis_Engine/mod/slide`)
    /// - value: priority
    pub nemesis_entries: PriorityMap,
    /// FNIS patch path
    /// - key: namespace (e.g. `FNISFlyer`)
    /// - value: priority
    ///
    /// # Important notes when using FNIS entries:
    /// 1. FNIS collects all files under `animations/<namespace>` across all mods.
    ///    If running in manual mode (outside of VFS) and multiple versions of the same mod exist,
    ///    they share the same namespace, which can lead to unintended conflicts such as
    ///    duplicate animation registrations and other bugs.
    /// 2. When using FNIS entries, ensure that `config.skyrim_data_dir_glob` includes
    ///    all relevant Skyrim data directories, e.g.:
    ///    - `"MO2/mods/*"`
    ///    - `"steamapps/Skyrim Special Edition/Data"`
    ///
    ///    Otherwise, the code may fail with errors.
    pub fnis_entries: PriorityMap,
}