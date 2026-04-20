use std::path::PathBuf;

use indexmap::IndexMap;
/// - key: full path(For asdsf)
/// - value: adsf patch
#[derive(Debug, Default)]
pub(crate) struct OwnedAsdsfPatchMap(pub IndexMap<PathBuf, (String, usize)>);

impl OwnedAsdsfPatchMap {
    #[inline]
    pub(crate) fn new() -> Self {
        Self(IndexMap::new())
    }

    #[inline]
    pub(crate) fn insert(
        &mut self,
        path: PathBuf,
        patch: (String, usize),
    ) -> Option<(String, usize)> {
        self.0.insert(path, patch)
    }
}
