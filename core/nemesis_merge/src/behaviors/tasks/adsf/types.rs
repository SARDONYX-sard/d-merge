use indexmap::IndexMap;
use std::path::PathBuf;
/// - key: full path(For adsf)
/// - value: adsf patch
#[derive(Debug, Default)]
pub struct OwnedAdsfPatchMap(pub IndexMap<PathBuf, (String, usize)>);

impl OwnedAdsfPatchMap {
    #[inline]
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    #[inline]
    pub fn insert(&mut self, path: PathBuf, patch: (String, usize)) -> Option<(String, usize)> {
        self.0.insert(path, patch)
    }
}
