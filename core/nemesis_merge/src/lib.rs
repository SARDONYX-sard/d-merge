mod behaviors;
mod config;
pub mod errors;
mod results;

pub use crate::behaviors::{behavior_gen, create_bin_templates};
pub use crate::config::{Config, DebugOptions, HackOptions, OutPutTarget, Status};

#[cfg(test)]
mod tests;
