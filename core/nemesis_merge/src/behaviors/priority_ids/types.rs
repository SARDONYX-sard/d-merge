use std::collections::HashMap;

/// Path & Priority map
///
/// - key:
///   - Nemesis => path until mod_code(e.g. `<skyrim data dir>/meshes/Nemesis_Engine/mod/slide`)
///   - FNIS => path until namespace(e.g. `<skyrim_data_dir>/meshes/actors/character/animations/FNISMod`)
/// - value: priority
pub type PriorityMap = HashMap<String, usize>;
