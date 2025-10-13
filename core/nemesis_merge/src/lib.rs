mod behaviors;
mod config;
pub mod errors;
pub mod global_logger;
mod results;

pub use crate::behaviors::{behavior_gen, create_bin_templates, PatchMaps, PriorityMap};
pub use crate::config::{Config, DebugOptions, HackOptions, OutPutTarget, Status};

#[cfg(test)]
mod tests;
