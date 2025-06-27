use crate::{Config, DebugOptions, HackOptions, OutPutTarget, Status};

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
    Box::new(|status| {
        use std::io::{stdout, Write};
        match &status {
            Status::ReadingPatches { .. } => {
                print!("{CLEAR_LINE}{CYAN}{status}{RESET}");
                stdout().flush().ok();
            }
            Status::ParsingPatches { .. } => {
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
                println!("{CLEAR_LINE}{GREEN_BOLD}{status}{RESET}");
            }
            Status::Error(_) => {
                println!("{CLEAR_LINE}{RED_BOLD}{status}{RESET}");
            }
        }
    })
}

#[allow(unused)]
pub(crate) fn fastest_config() -> Config {
    Config {
        resource_dir: "../../resource/assets/templates".into(),
        output_dir: "../../dummy/behavior_gen/output".into(),
        status_report: None,
        hack_options: Some(HackOptions::enable_all()),
        debug: DebugOptions {
            output_patch_json: false,
            output_merged_json: false,
            output_merged_xml: false,
        },
        output_target: OutPutTarget::SkyrimSe,
    }
}

#[allow(unused)]
pub(crate) fn slow_debug_config() -> Config {
    Config {
        resource_dir: "../../resource/assets/templates".into(),
        output_dir: "../../dummy/behavior_gen/output".into(),
        status_report: Some(new_color_status_reporter()), // +2s
        // status_report: None,
        hack_options: Some(HackOptions::enable_all()),
        debug: DebugOptions {
            output_patch_json: true,
            output_merged_json: true,
            output_merged_xml: false,
        },
        output_target: OutPutTarget::SkyrimSe,
    }
}
