use nemesis_merge::{Config, DebugOptions, HackOptions, Status};

use crate::args::PatchArgs;

pub(crate) fn build_config(args: &PatchArgs) -> Config {
    Config {
        resource_dir: args.resource_dir.clone(),
        output_dir: args.output_dir.clone(),
        status_report: (!args.no_status).then(new_color_status_reporter),
        hack_options: Some(HackOptions::enable_all()),
        debug: DebugOptions {
            output_patch_json: args.debug,
            output_merged_json: args.debug,
            output_merged_xml: false,
        },
        output_target: args.output_target.into(),
        skyrim_data_dir_glob: args.skyrim_data_dir_glob.clone(),
        generate_fnis_esp: args.generate_fnis_esp,
    }
}

// =======================
// ANSI Color Constants
// =======================
const CYAN: &str = "\x1b[36m";
const MAGENTA: &str = "\x1b[35m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const GREEN_BOLD: &str = "\x1b[32;1m";
const RED_BOLD: &str = "\x1b[31;1m";
const RESET: &str = "\x1b[0m";
const CLEAR_LINE: &str = "\r\x1b[2K"; // Delete the entire line and move to the beginning of the line.

pub(crate) fn new_color_status_reporter() -> Box<dyn Fn(Status) + Send + Sync> {
    let start = std::time::Instant::now();

    Box::new(move |status| {
        use std::io::{stdout, Write};
        match &status {
            Status::ReadingPatches { .. } => {
                print!("{CLEAR_LINE}{CYAN}{status}{RESET}");
                stdout().flush().ok();
            }
            Status::GeneratingFnisPatches { .. } | Status::ParsingPatches { .. } => {
                print!("{CLEAR_LINE}{MAGENTA}{status}{RESET}");
                stdout().flush().ok();
            }
            Status::ApplyingPatches { .. } => {
                print!("{CLEAR_LINE}{YELLOW}{status}{RESET}");
                stdout().flush().ok();
            }
            Status::GeneratingHkxFiles { .. } => {
                print!("{CLEAR_LINE}{BLUE}{status}{RESET}");
                stdout().flush().ok();
            }
            Status::Done => {
                let elapsed = start.elapsed();
                tracing::info!("Time: {elapsed:.2?}");
                println!("{CLEAR_LINE}{GREEN_BOLD}{status}{RESET} Time: {elapsed:.2?}");
            }
            Status::Error(_) => {
                tracing::info!("Time: {:.2?}", start.elapsed());
                println!("{CLEAR_LINE}{RED_BOLD}{status}{RESET}");
            }
        }
    })
}
