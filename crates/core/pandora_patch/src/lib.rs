//! Pandora Behaviour Engine's patch format: `Pandora_Engine/mod/<code>/patches/<pack file>.xml`.
//!
//! ```xml
//! <?xml version="1.0" encoding="utf-8"?>
//! <amco>
//!   <Insert path="">
//!     <hkobject name="#amco$0" class="hkbExpressionCondition" signature="0x1c3c1045">...</hkobject>
//!   </Insert>
//!   <Replace path="#2773/flags">
//!     <hkparam name="flags">0</hkparam>
//!   </Replace>
//!   <Replace path="#2802/transitions/Element0/initiateInterval/Element0/enterEventId">
//!     <hkparam name="enterEventId">$eventID[MCO_WinOpen]$</hkparam>
//!   </Replace>
//!   <Remove path="#0367/transitions/Element0" />
//!   <Append path="#0106/variableNames">
//!     <hkcstring>bIsPandoraLocked</hkcstring>
//!   </Append>
//! </amco>
//! ```
//!
//! # `path`
//! The object's name, then one part per level: a `hkparam` by its `name`, what has no name
//! (`hkobject`, `hkcstring`) as `Element<its index among those>`.
//!
//! - [`tree`]: reads the XML of an object.
//! - [`diff`]: what an edit of a template object is in this format.
//! - [`doc`]: what a whole file does to the template, and writing an object's part of it anew.
//! - [`path`]: what a `path` leads to, and what is wrong in a file.
//! - [`apply`]: does what Pandora does with a patch, to check the above.
//! - [`write`]: the files.

pub mod apply;
pub mod diff;
pub mod doc;
pub mod path;
pub mod tree;
pub mod write;

pub use self::{
    diff::{ObjectProblem, Op, OpKind, object_ops},
    write::{info_xml, patch_file},
};
