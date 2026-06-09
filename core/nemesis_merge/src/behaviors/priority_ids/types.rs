use rapidhash::fast::RapidHashMap as HashMap;

/// Path & Priority map
///
/// - key:
///   - Nemesis => path until mod_code(e.g. `<skyrim data dir>/meshes/Nemesis_Engine/mod/slide`)
///   - FNIS => FNIS list path(`.../meshes/actors/<behavior_object>[/_1stperson]/animations/<namespace>/FNIS_*_List.txt`)
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
    /// - key: `.../meshes/actors/<behavior_object>[/_1stperson]/animations/<namespace>/FNIS_*_List.txt`
    /// - value: priority
    pub fnis_entries: PriorityMap,
}
