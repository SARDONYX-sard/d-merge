use std::collections::HashMap;

use crate::{
    adsf::{
        alt::{to_adsf_key, AltAdsf, AltAnimData},
        clip_id_manager::ClipIdManager,
        normal::ser::{
            serialize_anim_header, serialize_clip_anim_block, serialize_clip_motion_block,
        },
    },
    diff_line::DiffLines,
};
use rayon::prelude::*;

/// Serializes to `animationdatasinglefile.txt` string.
///
/// # Errors
/// Returns an error if serialization fails.
pub fn serialize_alt_adsf_with_patches(
    alt_adsf: AltAdsf,
    patches: DiffLines,
) -> Result<String, SerializeError> {
    let mut output = String::new();

    let (mut project_names, anim_list): (Vec<_>, Vec<_>) = alt_adsf
        .0
        .into_par_iter()
        .map(|(k, v)| (to_adsf_key(k), v))
        .unzip();
    patches.into_apply(&mut project_names)?;

    let mut clip_id_manager = crate::adsf::clip_id_manager::ClipIdManager::new();
    // Serialize clip animation blocks
    // TODO: clip id unique check
    // Hints:
    // - It did not crash even if the number of `anim_data` and `motion_data` did not match.
    let mut clip_id_map = HashMap::new();

    // Serialize project names
    output.push_str(&format!("{}\r\n", project_names.len()));
    for name in project_names {
        let name = to_adsf_key(name.as_ref().into());
        output.push_str(name.as_ref());
        output.push_str("\r\n");
    }

    // Serialize animation data
    for anim_data in &anim_list {
        output.push_str(&serialize_anim_data(
            anim_data,
            &mut clip_id_manager,
            &mut clip_id_map,
        )?);
    }

    Ok(output)
}

/// Serializes to `animationdatasinglefile.txt` string.
///
/// # Errors
/// Returns an error if serialization fails.
pub fn serialize_alt_adsf(alt_adsf: &AltAdsf) -> Result<String, SerializeError> {
    let mut output = String::new();
    let project_names = alt_adsf.0.keys();
    let anim_list = alt_adsf.0.values();

    let mut clip_id_manager = crate::adsf::clip_id_manager::ClipIdManager::new();
    // Serialize clip animation blocks
    // TODO: clip id unique check
    // Hints:
    // - It did not crash even if the number of `anim_data` and `motion_data` did not match.
    let mut clip_id_map = HashMap::new();

    // Serialize project names
    output.push_str(&format!("{}\r\n", project_names.len()));
    for name in project_names {
        let name = to_adsf_key(name.as_ref().into());
        output.push_str(name.as_ref());
        output.push_str("\r\n");
    }

    // Serialize animation data
    for anim_data in anim_list {
        output.push_str(&serialize_anim_data(
            anim_data,
            &mut clip_id_manager,
            &mut clip_id_map,
        )?);
    }

    Ok(output)
}

/// Serializes animation data into a string.
///
/// # Errors
/// Returns an error if serialization fails.
fn serialize_anim_data<'a>(
    anim_data: &'a AltAnimData<'a>,
    clip_id_manager: &mut ClipIdManager,
    clip_id_map: &mut HashMap<&'a str, usize>,
) -> Result<String, SerializeError> {
    let mut output = String::new();

    // Serialize header
    output.push_str(&serialize_anim_header(
        &anim_data.header,
        anim_data.to_line_range(),
    ));

    let base_len = anim_data.clip_anim_blocks.len();
    let add_clip_len = anim_data.add_clip_anim_blocks.len();

    for block in &anim_data.add_clip_anim_blocks {
        let new_id = clip_id_manager.next_id();
        if let Some(new_id) = new_id {
            clip_id_map.insert(&block.clip_id, new_id);
        } else {
            return Err(SerializeError::ClipIdLimitReached {
                base_len,
                needed: add_clip_len,
            });
        }
        output.push_str(&serialize_clip_anim_block(
            block,
            new_id.map(|id| id.to_string().into()),
        ));
    }

    for (_, block) in &anim_data.clip_anim_blocks {
        output.push_str(&serialize_clip_anim_block(block, None));
    }

    let clip_motion_blocks_line_len = anim_data.clip_motion_blocks_line_len();
    if clip_motion_blocks_line_len > 0 {
        output.push_str(&format!("{clip_motion_blocks_line_len}\r\n"));
    };
    if anim_data.header.has_motion_data {
        for block in &anim_data.add_clip_motion_blocks {
            if let Some(&new_id) = clip_id_map.get(block.clip_id.as_ref()) {
                output.push_str(&serialize_clip_motion_block(
                    block,
                    Some(new_id.to_string().into()),
                ));
            } else {
                let new_id = clip_id_manager.next_id();
                output.push_str(&serialize_clip_motion_block(
                    block,
                    new_id.map(|id| id.to_string().into()),
                ));
            }
        }

        for (_, block) in &anim_data.clip_motion_blocks {
            output.push_str(&serialize_clip_motion_block(block, None));
        }
    }

    Ok(output)
}

#[derive(Debug, snafu::Snafu)]
pub enum SerializeError {
    #[snafu(display(
        "Clip ID allocation failed: base({base_len})) {needed} IDs needed but maximum i16 value is 32767. \
         Consider reducing the number of clips or splitting animations.",
    ))]
    ClipIdLimitReached { base_len: usize, needed: usize },

    #[snafu(transparent)]
    DiffLine {
        source: crate::diff_line::error::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalize_to_crlf(input: &str) -> std::borrow::Cow<'_, str> {
        if input.contains("\r\n") {
            input.into()
        } else {
            input.replace("\r", "").replace("\n", "\r\n").into()
        }
    }

    #[test]
    fn test_serialize_alt_adsf() {
        let alt_adsf_bytes = include_bytes!(
            "../../../../../resource/assets/templates/meshes/animationdatasinglefile.bin"
        );
        let alt_adsf = rmp_serde::from_slice(alt_adsf_bytes).unwrap();
        let actual = serialize_alt_adsf(&alt_adsf).unwrap();

        let expected = normalize_to_crlf(include_str!(
            "../../../../../resource/xml/templates/meshes/animationdatasinglefile.txt"
        ));
        let res = dbg!(actual == expected);
        if !res {
            let diff = ::diff::diff(&actual, &expected);
            std::fs::write("../../dummy/debug/animationdatasinglefile.diff", diff).unwrap();
            panic!("actual != expected");
        }
        assert!(res);
    }

    #[test]
    fn should_write_alt_adsf_json() {
        use crate::adsf::alt::AltAdsf;

        let input = include_str!(
            "../../../../../resource/xml/templates/meshes/animationdatasinglefile.txt"
        );
        let adsf = crate::adsf::normal::de::parse_adsf(input).unwrap_or_else(|err| {
            panic!("Failed to parse adsf:\n{err}");
        });
        let alt_adsf: AltAdsf = adsf.into();

        std::fs::create_dir_all("../../dummy/debug/").unwrap();

        std::fs::write(
            "../../dummy/debug/animationdatasinglefile_keys.log",
            format!("{:#?}", alt_adsf.0.keys()),
        )
        .unwrap();

        let json = serde_json::to_string_pretty(&alt_adsf).unwrap_or_else(|err| {
            panic!("Failed to serialize adsf to JSON:\n{err}");
        });
        std::fs::write("../../dummy/debug/animationdatasinglefile.json", json).unwrap();

        let bin = rmp_serde::to_vec(&alt_adsf).unwrap();
        std::fs::write("../../dummy/debug/animationdatasinglefile.bin", bin).unwrap();
    }
}
