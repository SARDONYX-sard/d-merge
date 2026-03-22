use std::path::PathBuf;

use indexmap::IndexMap;
/// - key: full path(For asdsf)
/// - value: adsf patch
#[derive(Debug, Default)]
pub struct OwnedAsdsfPatchMap(pub IndexMap<PathBuf, (String, usize)>);

impl OwnedAsdsfPatchMap {
    #[inline]
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    #[inline]
    pub fn insert(&mut self, path: PathBuf, patch: (String, usize)) -> Option<(String, usize)> {
        self.0.insert(path, patch)
    }
}
