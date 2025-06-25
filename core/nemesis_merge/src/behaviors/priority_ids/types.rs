use std::collections::HashMap;

/// - key: path until mod_code(e.g. `Nemesis_Engine/mod/slide`)
/// - value: priority
pub type PriorityMap<'a> = HashMap<&'a str, usize>;
