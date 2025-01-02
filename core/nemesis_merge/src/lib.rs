mod aliases;
mod behavior;
mod config;
pub mod errors;
mod hkx;
mod patches;
mod paths;
mod results;
mod tables;
mod templates;

pub use crate::config::{Config, Status};
pub use behavior::generate::behavior_gen;
pub use patches::generate::generate_patches;
