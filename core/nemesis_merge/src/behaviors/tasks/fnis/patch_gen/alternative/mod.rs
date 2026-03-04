//! # FNIS Alternative
//!
//! ```txt
//! <skyrim data dir>/
//! └── meshes/
//!     └── actors/
//!         └── character/                                      <- defaultmale, defaultfemale humanoid animations
//!             └── animations/
//!                 └── <fnis_mod_namespace>/                   <- this is `animations_mod_dir`
//!                     ├── xpe0_1hm_equip.hkx                  <- HKX animation file.
//!                     └── xpe0_1hm_unequip.HKX                <- HKX animation file.
//! ```
pub(crate) mod gen_old_patch;
#[expect(unused)]
mod generated_group_table;
