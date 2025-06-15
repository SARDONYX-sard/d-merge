use crate::adsf::{
    ser::{serialize_anim_header, serialize_clip_anim_block, serialize_clip_motion_block},
    AltAdsf, AltAnimData,
};

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
        let name = crate::adsf::to_adsf_key(name.as_ref().into());
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

    #[cfg(feature = "alt_map")]
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
}
