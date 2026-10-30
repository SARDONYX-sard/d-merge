//! `cargo x gen <what>`: code and assets generated from `resource/` (git submodule) and `xtask/data/`.
//!
//! Only `templates` / `anim-data` write into the submodule (its committed `.bin` assets).
//!
//! A separate binary (`xtask-gen`), built only with the `gen` feature; `cargo x gen` runs it.
//! Separate, so that building it never has to replace the running `xtask` executable
//! (Windows cannot overwrite a running exe).

mod alt_groups;
mod anim_data;
mod behavior_table;
mod behaviors;
mod nemesis_indexes;
mod templates;

use std::{path::PathBuf, process::ExitCode};

/// Result of a generator. Errors are printed as they are.
type Result<T = (), E = Box<dyn std::error::Error>> = core::result::Result<T, E>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result {
    let ws = workspace_root();
    let (what, rest) =
        args.split_first().ok_or("usage: cargo x gen <what> (see `cargo x help gen`)")?;
    match what.as_str() {
        "templates" => templates::run(&ws, rest),
        "anim-data" => anim_data::run(&ws),
        "alt-groups" => alt_groups::run(&ws),
        "behavior-table" => behavior_table::run(&ws),
        "behaviors" => {
            let input = rest
                .first()
                .map_or_else(|| ws.join("dummy/debug/behaviors_table.json"), |path| ws.join(path));
            let output = ws.join("crates/core/fnis_patch_gen/src/patch_gen/generated_behaviors.rs");
            behaviors::run(&input, &output)
        }
        "nemesis-indexes" => nemesis_indexes::run(&ws),
        other => Err(format!("unknown `gen {other}` (see `cargo x help gen`)").into()),
    }
}

/// The repository root (`xtask/..`).
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().map_or_else(|| manifest_dir.clone(), PathBuf::from)
}
