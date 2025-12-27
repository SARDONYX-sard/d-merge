use havok_classes::Classes;
use jwalk::WalkDir;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// -------------------------------
/// Output structures
/// -------------------------------

#[derive(Debug, Serialize)]
struct ClipInfo {
    /// Primary grouping key (e.g. state machine group)
    group_key: String,

    /// meshes template path.
    path: String,

    /// Alternative map key (e.g. animation binding / alias)
    alt_group_file: String,

    /// Resolved clip data
    raw: ClipGeneratorRaw,
}

#[derive(Debug, Serialize)]
struct ClipGeneratorRaw {
    ptr: String,

    #[serde(rename = "variableBindingSet")]
    variable_binding_set: havok_types::Pointer<'static>,
    #[serde(rename = "userData")]
    user_data: havok_types::Ulong,

    name: String,
    #[serde(rename = "animationName")]
    animation_name: String,
    triggers: havok_types::Pointer<'static>,

    #[serde(rename = "cropStartAmountLocalTime")]
    crop_start_amount_local_time: f32,
    #[serde(rename = "cropEndAmountLocalTime")]
    crop_end_amount_local_time: f32,
    #[serde(rename = "startTime")]
    start_time: f32,
    #[serde(rename = "playbackSpeed")]
    playback_speed: f32,
    #[serde(rename = "enforcedDuration")]
    enforced_duration: f32,
    #[serde(rename = "userControlledTimeFraction")]
    user_controlled_time_fraction: f32,

    #[serde(rename = "animationBindingIndex")]
    animation_binding_index: i16,
    mode: havok_classes::PlaybackMode,
    flags: i8,
}

/// file_path -> clips
type Output = BTreeMap<String, Vec<ClipInfo>>;
type FilterTable = std::collections::HashMap<&'static str, &'static str>;

// -------------------------------
// Core logic
// -------------------------------

fn extract_clip_generators(
    xml_path: &Path,
    root: &Path,
    anim_to_group: &FilterTable,
) -> Result<Vec<ClipInfo>, String> {
    let content = std::fs::read_to_string(xml_path)
        .map_err(|e| format!("read error {}: {}", xml_path.display(), e))?;

    let xml_path = xml_path
        .strip_prefix(root)
        .map_err(|e| format!("prefix error {}: {}", xml_path.display(), e))?;

    let class_map: serde_hkx_features::ClassMap = serde_hkx::from_str(&content)
        .map_err(|e| format!("parse error {}: {}", xml_path.display(), e))?;

    let clips = class_map
        .par_iter()
        .filter_map(|(_, class)| {
            if let Classes::hkbClipGenerator(cg) = class {
                let animation_name = cg.m_animationName.to_string();

                let normalized = normalize_anim_path(&animation_name);
                let group_key = anim_to_group.get(normalized.as_str())?;

                Some(ClipInfo {
                    group_key: group_key.to_string(),
                    path: xml_path.to_string_lossy().replace('\\', "/"),
                    alt_group_file: normalized,
                    raw: ClipGeneratorRaw {
                        ptr: cg.__ptr.as_ref()?.to_string(),
                        variable_binding_set: cg
                            .parent
                            .parent
                            .parent
                            .m_variableBindingSet
                            .to_static(),
                        user_data: cg.parent.parent.m_userData,
                        name: cg.parent.parent.m_name.to_string(),
                        triggers: cg.m_triggers.to_static(),
                        animation_name: cg.m_animationName.to_string(),

                        crop_start_amount_local_time: cg.m_cropStartAmountLocalTime,
                        crop_end_amount_local_time: cg.m_cropEndAmountLocalTime,
                        start_time: cg.m_startTime,
                        playback_speed: cg.m_playbackSpeed,
                        enforced_duration: cg.m_enforcedDuration,
                        user_controlled_time_fraction: cg.m_userControlledTimeFraction,
                        mode: cg.m_mode.clone(),

                        animation_binding_index: match &cg.m_animationBindingIndex {
                            havok_types::I16::Number(n) => *n,
                            other => panic!(
                                "animation_binding_index must be a numeric value, but got {:?}",
                                other
                            ),
                        },
                        flags: match &cg.m_flags {
                            havok_types::I8::Number(n) => *n,
                            other => panic!("flags must be a numeric value, but got {:?}", other),
                        },
                    },
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(clips)
}

fn collect_xml_files(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .par_bridge()
        .filter_map(|e| {
            let path = e.ok()?.path();
            if path.extension()?.eq_ignore_ascii_case("xml") {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

fn extract_all(root: &Path) -> Output {
    let xml_files = collect_xml_files(root);

    let filter_set = build_alt_animation_map();

    xml_files
        .par_iter()
        .filter_map(
            |path| match extract_clip_generators(path, root, &filter_set) {
                Ok(clips) if !clips.is_empty() => {
                    let key = path.to_string_lossy().replace('\\', "/");

                    Some((key, clips))
                }
                _ => None,
            },
        )
        .collect()
}

fn normalize_anim_path(s: &str) -> String {
    let s = s.replace('\\', "/").to_lowercase();
    let s = s.strip_prefix("animations/").unwrap_or(&s);
    s.to_string()
}

fn build_alt_animation_map() -> FilterTable {
    use super::generated_group_table::ALT_GROUPS;

    let mut map = FilterTable::new();
    let mut duplicates = Vec::new();

    for (group_key, group) in ALT_GROUPS.entries() {
        for &anim in group.animations {
            if let Some(prev_group) = map.insert(anim, group_key) {
                duplicates.push((anim.to_string(), prev_group, group_key.to_string()));
            }
        }
    }

    // let json = simd_json::to_string_pretty(&map).expect("failed to serialize json");
    // let output_path = Path::new("../../dummy/alt_map.json");
    // std::fs::write(output_path, json).expect("failed to write output file");
    if !duplicates.is_empty() {
        let dup_json =
            simd_json::to_string_pretty(&duplicates).expect("failed to serialize duplicates");

        std::fs::write(Path::new("../../dummy/alt_map_duplicates.json"), dup_json)
            .expect("failed to write alt_map_duplicates.json");
    }

    map
}

fn regroup_by_group_key(input: Output) -> BTreeMap<String, Output> {
    let mut out: BTreeMap<String, Output> = BTreeMap::new();

    for (_xml_path, clips) in input {
        for clip in clips {
            let top_key = if clip.path.contains("_1stperson") {
                "_1stperson"
            } else {
                "character"
            };

            out.entry(top_key.to_string())
                .or_default()
                .entry(clip.group_key.clone())
                .or_default()
                .push(clip);
        }
    }

    out
}

#[test]
fn main() {
    // ===== 設定 =====
    let input_root = Path::new("../../resource/xml/templates");
    let output_path = Path::new("../../dummy/clip_generators.json");
    // =================

    let output = extract_all(input_root);
    let output = regroup_by_group_key(output);

    let json = sonic_rs::to_string_pretty(&output).expect("failed to serialize json");

    std::fs::write(output_path, json).expect("failed to write output file");

    println!(
        "Extracted hkbClipGenerator info written to {}",
        output_path.display()
    );
}
