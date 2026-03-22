mod examples;

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use nemesis_merge::OutPutTarget;

#[derive(Debug, Parser)]
#[command(author, version, about, after_long_help = examples::PATCH_EXAMPLES )]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Log level: error | warn | info | debug | trace
    #[clap(global = true, long, display_order = 100)]
    #[arg(long, value_name = "LEVEL", default_value = "info")]
    pub log_level: tracing::Level,

    /// Write logs to this file (default: stderr only)
    #[clap(global = true, long, display_order = 101)]
    #[arg(long, value_name = "FILE")]
    pub log_file: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Generate behavior patches from Nemesis and/or FNIS ini files
    Patch(PatchArgs),
    /// Query mod/Skyrim installation information
    Info(InfoArgs),
}

#[derive(Debug, Args)]
pub(crate) struct InfoArgs {
    #[command(subcommand)]
    pub command: InfoCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum InfoCommand {
    /// Print the Skyrim data directory path
    #[command(after_long_help = examples::SKYRIM_DIR_EXAMPLES)]
    SkyrimDir(SkyrimDirArgs),

    /// List all Nemesis/FNIS mods found under a glob pattern (JSON output)
    #[command(after_long_help = examples::MODS_EXAMPLES)]
    Mods(ModsArgs),
}

#[derive(Debug, Args)]
pub(crate) struct SkyrimDirArgs {
    /// Target runtime
    #[arg(long, value_name = "RUNTIME", default_value = "SkyrimSE")]
    pub runtime: Runtime,
}

#[derive(Debug, Args)]
pub(crate) struct ModsArgs {
    /// Skyrim data dir glob pattern to search (e.g. "D:/GAME/ModOrganizer Skyrim SE/mods/*", "D:\STEAM\steamapps\common\Skyrim Special Edition\Data")
    #[arg(long, value_name = "GLOB")]
    pub glob: String,

    /// Output json file path. If not provided, stdout.
    #[arg(long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Treat paths as virtual filesystem (MO2 VFS)
    #[arg(long, default_value_t = false)]
    pub vfs: bool,
}

#[derive(Debug, Args)]
#[command(after_long_help = examples::PATCH_EXAMPLES)]
pub(crate) struct PatchArgs {
    /// Nemesis ids.ini — one mod ID per line, `;` for comments
    #[arg(long, value_name = "FILE")]
    pub nemesis_ini: Option<PathBuf>,

    /// FNIS ids.ini — same format as Nemesis ini
    #[arg(long, value_name = "FILE")]
    pub fnis_ini: Option<PathBuf>,

    /// Glob pattern to locate Skyrim mod directories (required when --fnis-ini is set)
    ///
    /// Example: "D:/GAME/ModOrganizer Skyrim SE/mods/*"
    #[arg(long, value_name = "GLOB")]
    pub skyrim_data_dir_glob: Option<String>,

    /// Target Skyrim runtime for output format
    #[arg(long, value_name = "TARGET", default_value = "SkyrimSE")]
    pub output_target: Runtime,

    /// Resource/template directory
    #[arg(long, value_name = "DIR", default_value = "./assets/templates")]
    pub resource_dir: PathBuf,

    /// Output directory
    #[arg(long, value_name = "DIR", default_value = "./output")]
    pub output_dir: PathBuf,

    /// Enable all debug output (patch JSON, merged JSON)
    #[arg(long, default_value_t = false)]
    pub debug: bool,

    /// Disable the color status reporter
    #[arg(long, default_value_t = false)]
    pub no_status: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Runtime {
    Se,
    Le,
}

impl From<Runtime> for OutPutTarget {
    fn from(r: Runtime) -> Self {
        match r {
            Runtime::Se => Self::SkyrimSe,
            Runtime::Le => Self::SkyrimLe,
        }
    }
}

impl core::str::FromStr for Runtime {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            _ if s.eq_ignore_ascii_case("SkyrimSE") => Self::Se,
            _ if s.eq_ignore_ascii_case("SkyrimLE") => Self::Le,
            invalid => {
                return Err(format!(
                    "Expected `SkyrimSE` or `SkyrimLE`. But got {invalid}"
                ))
            }
        })
    }
}
