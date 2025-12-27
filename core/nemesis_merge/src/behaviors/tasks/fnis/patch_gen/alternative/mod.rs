//! # FNIS Alternative
//!
//! ```txt
//! <skyrim data dir>/
//! └── meshes/
//!     └── actors/
//!         └── character/                                      <- defaultmale, defaultfemale humanoid animations
//!             └── animations/
//!                 └── <fnis_mod_namespace>/                   <- this is `animations_mod_dir`
//!                     ├── FNIS_<namespace>_toOAR.json         <- FNIS alt anim to OAR override config file.(optional)
//!                     ├── xpe0_1hm_equip.hkx                  <- HKX animation file.
//!                     └── xpe0_1hm_unequip.HKX                <- HKX animation file.
//! ```
pub(crate) mod gen_old_patch;
mod generated_group_table;
pub(crate) mod to_oar;
