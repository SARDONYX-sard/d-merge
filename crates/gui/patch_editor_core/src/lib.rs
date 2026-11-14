//! What the Patch Editor edits, without its drawing: the page (`page_patch_editor`) draws it.
//!
//! ```text
//! workspace      the work: the behaviors, their edits and new objects, the history; read, saved, exported
//! tree           the objects of a behavior as a tree from its root, and the state of a tree shown
//! editor         what the editor of an object needs: names, diagnostics, the preview, the merge
//! anim           animation data (`animationdatasinglefile`, `animationsetdatasinglefile`)
//! template_tree  the template files as folders
//! vars_edit      the events and variables a mod added
//! repeat         many objects from one
//! new_work       the form of a new work
//! ```

pub mod anim;
pub mod editor;
pub mod new_work;
pub mod repeat;
pub mod template_tree;
pub mod tree;
pub mod vars_edit;
pub mod workspace;
