//! Instead of generating a Nemesis patch, use the BehaviorDataInjector (BDI) configuration file
//! to register variables that serve as an index for FNIS general animations.
//!
//! docs: https://github.com/max-su-2019/BehaviorDataInjector/blob/master/doc/How%20to%20create%20BDI%20config%20files.md
//!
//!
//! # Example
//!
//! ```txt
//! DGV FNISaa_1hmeqp i
//! ```
use std::path::Path;

use serde::Serialize;

use crate::errors::Error;

/// A single BDI graph variable entry.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BdiEntry {
    /// Relative to `Data/Meshes/`. e.g. `actors/character`
    project_path: &'static str,
    /// Always `kInt` for FNIS alt anim variables
    #[serde(rename = "type")]
    ty: &'static str,
    /// e.g. `FNISaa_mtidle` or `FNISaa_mtidle_crc`
    name: String,
    /// Initial value — always 0
    value: i32,
}

/// Generates the BDI config JSON for a given behavior entry.
///
/// Injects `FNISaa_*` and `FNISaa_*_crc` int variables into the behavior graph
/// so that FNIS alternate animations can drive OAR conditions at runtime.
///
/// # Output path
/// `SKSE/Plugins/BehaviorDataInjector/FNIS_AA_to_OAR_BDI.json`
pub fn generate_bdi_config(output_dir: &Path) -> Result<(), crate::errors::Error> {
    let mut entries: Vec<BdiEntry> = super::generated_group_table::ALT_GROUPS
        .keys()
        .flat_map(|group| {
            [
                BdiEntry {
                    project_path: "actors\\Character",
                    ty: "kInt",
                    name: format!("FNISaa{group}"),
                    value: 0,
                },
                BdiEntry {
                    project_path: "actors\\Character", // Important: back slash path sep
                    ty: "kInt",
                    name: format!("FNISaa{group}_crc"),
                    value: 0,
                },
            ]
        })
        .collect();
    entries.push(BdiEntry {
        project_path: "actors\\Character", // Important: back slash path sep
        ty: "kInt",
        name: "FNISaa_crc".to_string(),
        value: 0,
    });

    let path = output_dir.join("SKSE/Plugins/BehaviorDataInjector/FNIS_AA_to_OAR_BDI.json");
    let json = sonic_rs::to_string_pretty(&entries).map_err(|e| Error::JsonError {
        path: path.clone(),
        source: e,
    })?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::FailedIo {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    std::fs::write(&path, &json).map_err(|e| Error::FailedIo {
        path: path.clone(),
        source: e,
    })?;
    Ok(())
}

#[test]
fn write_bdi_config() {
    let output_dir = Path::new("../../dummy/debug");
    generate_bdi_config(output_dir).unwrap();
}
