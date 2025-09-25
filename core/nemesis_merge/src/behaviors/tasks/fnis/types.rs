use indexmap::IndexMap;
use std::path::PathBuf;
/// - key: full fnis path
/// - value: (patch)
#[derive(Debug, Default)]
pub struct OwnedFnisPatchMap(pub IndexMap<PathBuf, (String, usize)>);

impl OwnedFnisPatchMap {
    #[inline]
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    #[inline]
    pub fn insert(&mut self, path: PathBuf, patch: (String, usize)) -> Option<(String, usize)> {
        self.0.insert(path, patch)
    }
}
