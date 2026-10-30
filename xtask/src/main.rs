//! Repository tasks, run with `cargo x <task>` (alias in `.cargo/config.toml`).
//!
//! `cargo x help` lists the tasks; `cargo x help <task>` shows the options of one.
//!
//! # Adding a task
//! Add a module with `run(args)`, then a row to [`TASKS`].

// The gen dependencies belong to the xtask-gen binary.
#![cfg_attr(feature = "gen", allow(unused_crate_dependencies))]

mod cmd;
mod release;
mod tag;
mod version;

use std::{path::PathBuf, process::ExitCode};

/// Result of a task. Errors are printed as they are, so they are plain messages.
type Result<T = (), E = Box<dyn std::error::Error>> = core::result::Result<T, E>;

struct Task {
    name: &'static str,
    usage: &'static str,
    help: &'static str,
    run: fn(&[String]) -> Result,
}

const TASKS: &[Task] = &[
    Task { name: "release", usage: release::USAGE, help: release::HELP, run: release::run },
    Task { name: "tag", usage: tag::USAGE, help: tag::HELP, run: tag::run },
    Task { name: "gen", usage: GEN_USAGE, help: GEN_HELP, run: run_gen },
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (name, args) = match args.split_first() {
        Some((name, args)) => (name.as_str(), args),
        None => ("help", &[][..]),
    };

    if matches!(name, "help" | "-h" | "--help") {
        return match help(args.first().map(String::as_str)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => fail(&err.to_string()),
        };
    }

    let Some(task) = TASKS.iter().find(|task| task.name == name) else {
        return fail(&format!("unknown task `{name}` (see `cargo x help`)"));
    };
    match (task.run)(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => fail(&err.to_string()),
    }
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("error: {msg}");
    ExitCode::FAILURE
}

fn help(name: Option<&str>) -> Result {
    let tasks: Vec<&Task> = match name {
        Some(name) => vec![
            TASKS
                .iter()
                .find(|task| task.name == name)
                .ok_or_else(|| format!("unknown task `{name}`"))?,
        ],
        None => TASKS.iter().collect(),
    };
    println!("usage: cargo x <task> [options]\n");
    for task in tasks {
        println!("{} {}\n  {}\n", task.name, task.usage, task.help.replace('\n', "\n  "));
    }
    Ok(())
}

/// The repository root (the parent of `xtask/`).
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().map_or_else(|| manifest_dir.clone(), PathBuf::from)
}

// ── gen ──────────────────────────────────────────────────────────────────────

const GEN_USAGE: &str = "<what> [options]";
const GEN_HELP: &str = "\
Generates code or assets from the resources (builds the heavy crates on first use).
  templates [<dir>..] [-o <dir>]  template xml -> msgpack .bin
                                  (default: resource/xml/templates -> resource/assets/templates)
  anim-data                       animation(set)datasinglefile.txt -> .bin (+ debug json in dummy/debug)
  alt-groups                      xtask/data/Alternate_AnimationGroups.json -> fnis_patch_gen generated_group_table.rs
  behavior-table                  dummy/debug/FNIS_output/fnis_table.json + templates -> dummy/behaviors_table.json
  behaviors [<json>]              behaviors table json -> fnis_patch_gen generated_behaviors.rs
                                  (default: dummy/debug/behaviors_table.json)
  nemesis-indexes                 3rd person template indexes -> dummy/extracted_indexes.json";

/// `gen` needs heavy crates (`behavior_merge`, ...), so it is a separate binary built with the
/// `gen` feature: `cargo x release` stays fast and `cargo x gen` still just works.
fn run_gen(args: &[String]) -> Result {
    let mut cargo_args =
        vec!["run", "-q", "-p", "xtask", "--bin", "xtask-gen", "--features", "gen", "--"];
    cargo_args.extend(args.iter().map(String::as_str));
    cmd::run("cargo", &cargo_args)
}
