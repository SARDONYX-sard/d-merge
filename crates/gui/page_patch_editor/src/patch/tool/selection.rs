//! What is shown: the behavior (`active`) and the object of it (`selected`). The side bar picks
//! them, the editor shows them, the tabs remember them.
//!
//! ```text
//! active    0                 index in `Workspace::behaviors`, e.g. 0_master
//! selected  Some("#0106")     the object open in the editor; `None`: none is
//! ```

use std::rc::Rc;

/// See the module docs.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(in crate::patch) struct Selection {
    pub(in crate::patch) active: usize,
    pub(in crate::patch) selected: Option<Rc<str>>,
}
