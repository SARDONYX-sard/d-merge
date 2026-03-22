mod args;
mod config;
mod ini_parser;
mod mod_info_cmd;

use crate::{
    args::{Cli, Command, InfoCommand},
    config::build_config,
    ini_parser::parse_ids_ini,
};
use clap::Parser;
use nemesis_merge::{behavior_gen, PatchMaps};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        tracing::error!("{e}");
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if let Some((Some(parent), Some(file_name))) = cli
        .log_file
        .as_deref()
        .map(|path| (path.parent(), path.file_name().and_then(|f| f.to_str())))
    {
        tracing_rotation::init(parent, file_name)?;
        tracing_rotation::change_level(cli.log_level.as_str())?;
    };

    match cli.command {
        Command::Patch(ref args) => {
            // --fnis-ini requires --skyrim-data-dir-glob
            if args.fnis_ini.is_some() && args.skyrim_data_dir_glob.is_none() {
                return Err("--fnis-ini requires --skyrim-data-dir-glob to be set".into());
            }

            let nemesis_entries = match &args.nemesis_ini {
                Some(path) => parse_ids_ini(path, 0)
                    .map_err(|e| format!("failed to read --nemesis-ini {path:?}: {e}"))?,
                None => Default::default(),
            };

            let fnis_entries = match &args.fnis_ini {
                Some(path) => parse_ids_ini(path, nemesis_entries.len())
                    .map_err(|e| format!("failed to read --fnis-ini {path:?}: {e}"))?,
                None => Default::default(),
            };

            let patches = PatchMaps {
                nemesis_entries,
                fnis_entries,
            };
            let config = build_config(args);

            behavior_gen(patches, config)
                .await
                .map_err(|e| format!("behavior_gen failed: {e}"))?;
        }
        Command::Info(ref info) => match &info.command {
            InfoCommand::SkyrimDir(args) => {
                mod_info_cmd::run_skyrim_dir(args.runtime)?;
            }
            InfoCommand::Mods(args) => {
                mod_info_cmd::run_mods(&args.glob, args.vfs, args.output.as_deref())?;
            }
        },
    }

    Ok(())
}
