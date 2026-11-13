//! The two animation data templates, `animationdatasinglefile` and `animationsetdatasinglefile`.
//!
//! - [`character`]: adding the mod's hkx to the `animationNames` of its actor's characters
//! - [`data`]: making clips and sets, and reading the template's names
//! - [`lines`]: what each line of a file means, for hints, hovers and completion
//! - [`patch_file`]: what a patch file is by its path, and d-merge's parser check of it

pub(in crate::patch) mod character;
pub(in crate::patch) mod data;
pub(in crate::patch) mod lines;
pub(in crate::patch) mod patch_file;
