//! `cargo x gen anim-data`: `animationdatasinglefile.txt` / `animationsetdatasinglefile.txt`
//! -> the msgpack `.bin` templates `behavior_merge` patches.
//!
//! Also writes json dumps to `dummy/debug/` for reviewing the data.

use std::{collections::BTreeMap, path::Path};

use skyrim_anim_data::{adsf::alt::AltAdsf, asdsf::alt::AltAsdsf};

use crate::Result;

pub(crate) fn run(ws: &Path) -> Result {
    let txt_dir = ws.join("resource/xml/templates/meshes");
    let bin_dir = ws.join("resource/assets/templates/meshes");
    let debug_dir = ws.join("dummy/debug");
    std::fs::create_dir_all(&debug_dir)?;

    // ── animationdatasinglefile ──────────────────────────────────────────────
    let txt = std::fs::read_to_string(txt_dir.join("animationdatasinglefile.txt"))?;
    let adsf = skyrim_anim_data::adsf::normal::de::parse_adsf(&txt)
        .map_err(|err| format!("Failed to parse adsf:\n{err}"))?;
    let alt_adsf: AltAdsf = adsf.into();

    let mut keys: Vec<_> = alt_adsf.0.keys().collect();
    keys.sort_unstable();
    write(
        &debug_dir.join("animationdatasinglefile_keys.json"),
        &serde_json::to_string_pretty(&keys)?,
    )?;
    write(
        &debug_dir.join("animationdatasinglefile.json"),
        &serde_json::to_string_pretty(&alt_adsf)?,
    )?;
    write(&bin_dir.join("animationdatasinglefile.bin"), &rmp_serde::to_vec(&alt_adsf)?)?;

    // ── animationsetdatasinglefile ───────────────────────────────────────────
    let txt = std::fs::read_to_string(txt_dir.join("animationsetdatasinglefile.txt"))?;
    let asdsf = skyrim_anim_data::asdsf::normal::de::parse_asdsf(&txt)
        .map_err(|err| format!("Failed to parse asdsf:\n{err}"))?;
    let alt_asdsf: AltAsdsf = asdsf.try_into()?;

    // e.g., { "TxtProjectHeader": ["SubTxtHeader1", "SubTxtHeader2"] } (sorted by BTreeMap)
    let structure: BTreeMap<String, Vec<String>> = alt_asdsf
        .txt_projects
        .0
        .iter()
        .map(|(header, sets)| {
            (header.to_string(), sets.0.keys().map(ToString::to_string).collect())
        })
        .collect();
    write(
        &debug_dir.join("animationsetdatasinglefile_structure.json"),
        &serde_json::to_string_pretty(&structure)?,
    )?;
    write(
        &debug_dir.join("animationsetdatasinglefile.json"),
        &serde_json::to_string_pretty(&alt_asdsf)?,
    )?;
    write(&bin_dir.join("animationsetdatasinglefile.bin"), &rmp_serde::to_vec(&alt_asdsf)?)?;
    Ok(())
}

fn write(path: &Path, contents: impl AsRef<[u8]>) -> Result {
    std::fs::write(path, contents)?;
    println!("generated: {}", path.display());
    Ok(())
}
