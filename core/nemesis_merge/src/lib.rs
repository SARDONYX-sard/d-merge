mod behaviors;
pub mod cache_remover;
mod config;
pub mod errors;
mod results;

pub use crate::{
    behaviors::{PatchMaps, PriorityMap, behavior_gen, create_bin_templates},
    config::{Config, DebugOptions, HackOptions, OutPutTarget, Status},
};

#[cfg(test)]
mod tests;
