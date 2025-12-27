//@ts-check
import fs from 'fs';

const input = JSON.parse(fs.readFileSync('./dummy/clip_generators.json', 'utf8'));
/**
 * @typedef {Object} ClipInfo
 *
 * @property {string} group_key
 *   FNIS AA group key (e.g. "_mtidle")
 *
 * @property {string} path
 *   XML template path (relative, normalized)
 *
 * @property {string} alt_group_file
 *   Normalized animation path (e.g. "male/mt_idle.hkx")
 *
 * @property {ClipGeneratorRaw} raw
 */
/**
 * @typedef {Object} ClipGeneratorRaw
 *
 * @property {string} ptr
 *   Havok object pointer (e.g. "#0585")
 *
 * @property {string} variableBindingSet
 *   Havok pointer string (e.g. "#0000")
 *
 * @property {number} userData
 *
 * @property {string} name
 *   Clip generator name
 *
 * @property {string} animationName
 *   Full animation path used by Havok
 *
 * @property {string} triggers
 *   Havok pointer or "#0000"
 *
 * @property {number} cropStartAmountLocalTime
 * @property {number} cropEndAmountLocalTime
 * @property {number} startTime
 * @property {number} playbackSpeed
 * @property {number} enforcedDuration
 * @property {number} userControlledTimeFraction
 *
 * @property {number} animationBindingIndex
 * @property {string} mode
 *   havok_classes::PlaybackMode variant name
 *
 * @property {number} flags
 */

/**
 *
 * @param {string} s
 * @returns
 */
function rustStr(s) {
  return `"${s.replace(/\\/g, '/').replace(/"/g, '\\"')}"`;
}

/**
 * Force Rust f32 literal
 * @param {number} n
 * @returns {string}
 */
function rustF32(n) {
  if (!Number.isFinite(n)) return '0.0';
  return Number.isInteger(n) ? `${n}.0` : `${n}`;
}

/**
 *
 * @param {ClipInfo} clip
 * @returns
 */
function emitClipInfo(clip) {
  const r = clip.raw;

  return `
        ClipInfo {
            group_key: ${rustStr(clip.group_key)},
            path: ${rustStr(clip.path.replace('.xml', '.bin'))},
            alt_group_file: ${rustStr(clip.alt_group_file)},
            raw: ClipGeneratorRaw {
                ptr: ${rustStr(r.ptr)},
                variable_binding_set: "${r.variableBindingSet.replace('null', '#0000')}",
                user_data: ${r.userData},
                name: "${r.name.replaceAll('\\', '\\\\')}",
                animation_name: "${r.animationName.replaceAll('\\', '\\\\')}",
                triggers: "${r.triggers.replace('null', '#0000')}",
                crop_start_amount_local_time: ${rustF32(r.cropStartAmountLocalTime)},
                crop_end_amount_local_time: ${rustF32(r.cropEndAmountLocalTime)},
                start_time: ${rustF32(r.startTime)},
                playback_speed: ${rustF32(r.playbackSpeed)},
                enforced_duration: ${rustF32(r.enforcedDuration)},
                user_controlled_time_fraction: ${rustF32(r.userControlledTimeFraction)},
                animation_binding_index: ${r.animationBindingIndex},
                mode: "${r.mode}",
                flags: ${r.flags},
            },
        }`;
}

/**
 *
 * @param {string} name
 * @param {Record<string, ClipInfo[]>} groups
 * @returns
 */
function emitPhfMap(name, groups) {
  let out = `
pub static ${name}: phf::Map<&'static str, &'static [ClipInfo]> = phf::phf_map! {
`;

  for (const [groupKey, clips] of Object.entries(groups)) {
    out += `    ${rustStr(groupKey)} => &[\n`;
    for (const clip of clips) {
      out += emitClipInfo(clip) + ',\n';
    }
    out += '    ],\n';
  }

  out += '};\n';
  return out;
}

// -------------------------------
// Generate
// -------------------------------

let rs = `
#[derive(Debug)]
pub struct ClipInfo {
    /// FNIS AA group key (e.g. "_jump")
    pub group_key: &'static str,

    /// xml template path from \`meshes\`
    pub path: &'static str,

    /// normalized animation path (alt key)
    pub alt_group_file: &'static str,

    /// resolved clip generator data
    pub raw: ClipGeneratorRaw,
}

#[derive(Debug)]
pub struct ClipGeneratorRaw {
    /// hkobject ptr (e.g. "#0585")
    pub ptr: &'static str,

    /// ptr (e.g. "#0585")
    pub variable_binding_set: &'static str,
    pub user_data: u64,

    pub name: &'static str,
    pub animation_name: &'static str,
    /// ptr (e.g. "#0585")
    pub triggers: &'static str,

    pub crop_start_amount_local_time: f32,
    pub crop_end_amount_local_time: f32,
    pub start_time: f32,
    pub playback_speed: f32,
    pub enforced_duration: f32,
    pub user_controlled_time_fraction: f32,

    pub animation_binding_index: i16,
    /// Enum string
    pub mode: &'static str,
    pub flags: i8,
}
`;

rs += emitPhfMap('CHARACTER_CLIPS', input.character);
rs += emitPhfMap('FIRSTPERSON_CLIPS', input['_1stperson']);

fs.writeFileSync('core/nemesis_merge/src/behaviors/tasks/fnis/patch_gen/alternative/generated_group_table.rs', rs);
