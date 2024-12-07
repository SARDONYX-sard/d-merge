mod collect_path;
pub mod error;
mod merger;
mod output_path;

pub use merger::{
    behavior_gen::behavior_gen,
    config::{Config, Status},
};
