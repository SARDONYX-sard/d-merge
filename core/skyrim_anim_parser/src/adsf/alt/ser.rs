use crate::{
    adsf::{
        alt::{to_adsf_key, AltAdsf, AltAnimData},
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
) -> Result<String, crate::diff_line::error::Error> {
    let mut output = String::new();

    let (mut project_names, anim_list): (Vec<_>, Vec<_>) = alt_adsf
        .0
        .into_par_iter()
        .map(|(k, v)| (to_adsf_key(k), v))
        .unzip();
    patches.into_apply(&mut project_names)?;

    // Serialize project names
    output.push_str(&format!("{}\r\n", project_names.len()));
    for name in project_names {
        let name = to_adsf_key(name.as_ref().into());
        output.push_str(name.as_ref());
        output.push_str("\r\n");
    }

    // Serialize animation data
    for anim_data in &anim_list {
        output.push_str(&serialize_anim_data(anim_data));
    }

    Ok(output)
}

/// Serializes to `animationdatasinglefile.txt` string.
///
/// # Errors
/// Returns an error if serialization fails.
pub fn serialize_alt_adsf(alt_adsf: &AltAdsf) -> String {
    let mut output = String::new();
    let project_names = alt_adsf.0.keys();
    let anim_list = alt_adsf.0.values();

    // Serialize project names
    output.push_str(&format!("{}\r\n", project_names.len()));
    for name in project_names {
        let name = to_adsf_key(name.as_ref().into());
        output.push_str(name.as_ref());
        output.push_str("\r\n");
    }

    // Serialize animation data
    for anim_data in anim_list {
        output.push_str(&serialize_anim_data(anim_data));
    }

    output
}

/// Serializes animation data into a string.
///
/// # Errors
/// Returns an error if serialization fails.
fn serialize_anim_data(anim_data: &AltAnimData) -> String {
    let mut output = String::new();

    // Serialize header
    output.push_str(&serialize_anim_header(
        &anim_data.header,
        anim_data.to_line_range(),
    ));

    // Serialize clip animation blocks
    // TODO: clip id unique check
    // Hints:
    // - It did not crash even if the number of `anim_data` and `motion_data` did not match.

    let mut clip_id_manager = crate::adsf::clip_id_manager::ClipIdManager::new();
    let mut clip_id_map = std::collections::HashMap::new();
    for block in &anim_data.add_clip_anim_blocks {
        let new_id = clip_id_manager.next_id();
        if let Some(new_id) = new_id {
            clip_id_map.insert(&block.clip_id, new_id);
        } else {
            #[cfg(feature = "tracing")]
            tracing::error!("clip_id allocation has reached its limit");
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
            if let Some(&new_id) = clip_id_map.get(&block.clip_id) {
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

    output
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
        let actual = serialize_alt_adsf(&alt_adsf);

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

        if serialize_alt_adsf(&alt_adsf) == input {
            panic!("alt_adsf != input");
        }

        std::fs::create_dir_all("../../dummy/debug/").unwrap();
        let json = serde_json::to_string_pretty(&alt_adsf).unwrap_or_else(|err| {
            panic!("Failed to serialize adsf to JSON:\n{err}");
        });
        std::fs::write("../../dummy/debug/animationdatasinglefile.json", json).unwrap();

        let bin = rmp_serde::to_vec(&alt_adsf).unwrap();
        std::fs::write("../../dummy/debug/animationdatasinglefile.bin", bin).unwrap();
    }
}
