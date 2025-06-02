use std::borrow::Cow;

use super::{
    Adsf, AnimData, AnimDataHeader, ClipAnimDataBlock, ClipMotionBlock, Rotation, Translation,
};

/// Serializes the animation data structure into a string.
///
/// # Errors
/// Returns an error if serialization fails.
pub fn serialize_adsf(adsf: &Adsf) -> String {
    let mut output = String::new();

    // Serialize project names
    output.push_str(&format!("{}\r\n", adsf.project_names.len()));
    for name in &adsf.project_names {
        output.push_str(name.as_ref());
        output.push_str(".txt\r\n");
    }

    // Serialize animation data
    for anim_data in &adsf.anim_list {
        output.push_str(&serialize_anim_data(anim_data));
    }

    output
}

#[cfg(feature = "alt_map")]
/// Serializes to `animationdatasinglefile.txt` string.
///
/// # Errors
/// Returns an error if serialization fails.
pub fn serialize_alt_adsf(alt_adsf: &super::AltAdsf) -> String {
    let mut output = String::new();
    let project_names = alt_adsf.0.keys();
    let anim_list = alt_adsf.0.values();

    // Serialize project names
    output.push_str(&format!("{}\r\n", project_names.len()));
    for name in project_names {
        let name = super::to_adsf_key(name.as_ref().into());
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
fn serialize_anim_data(anim_data: &AnimData) -> String {
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

    let mut clip_id_manager = super::clip_id_manager::ClipIdManager::new();
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

    for block in &anim_data.clip_anim_blocks {
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

        for block in &anim_data.clip_motion_blocks {
            output.push_str(&serialize_clip_motion_block(block, None));
        }
    }

    output
}

/// Serializes the animation data header into a string.
///
/// # Errors
/// Returns an error if serialization fails.
fn serialize_anim_header(header: &AnimDataHeader, line_range: usize) -> String {
    let mut output = String::new();

    output.push_str(&line_range.to_string());
    output.push_str("\r\n");
    output.push_str(&header.lead_int.to_string());
    output.push_str("\r\n");
    output.push_str(&header.project_assets.len().to_string());
    output.push_str("\r\n");

    for asset in &header.project_assets {
        output.push_str(asset.as_ref());
        output.push_str("\r\n");
    }
    output.push_str(&format!(
        "{}\r\n",
        if header.has_motion_data { 1 } else { 0 }
    ));

    output
}

/// Serializes a clip animation data block into a string.
fn serialize_clip_anim_block(
    block: &ClipAnimDataBlock,
    replace_clip_id: Option<Cow<'_, str>>,
) -> String {
    let mut output = String::new();

    let ClipAnimDataBlock {
        name,
        clip_id,
        play_back_speed,
        crop_start_local_time,
        crop_end_local_time,
        trigger_names_len,
        trigger_names,
    } = block;

    output.push_str(name.as_ref());
    output.push_str("\r\n");

    match replace_clip_id {
        Some(new_clip_id) => output.push_str(new_clip_id.as_ref()),
        None => output.push_str(clip_id.as_ref()),
    };
    output.push_str("\r\n");

    output.push_str(play_back_speed.as_ref());
    output.push_str("\r\n");

    output.push_str(crop_start_local_time.as_ref());
    output.push_str("\r\n");

    output.push_str(crop_end_local_time.as_ref());
    output.push_str("\r\n");

    output.push_str(&format!("{trigger_names_len}\r\n"));
    for trigger in trigger_names {
        output.push_str(trigger.as_ref());
        output.push_str("\r\n");
    }
    output.push_str("\r\n"); // Empty line

    output
}

/// Serializes a clip motion block into a string.
fn serialize_clip_motion_block(
    block: &ClipMotionBlock,
    replace_clip_id: Option<Cow<'_, str>>,
) -> String {
    let mut output = String::new();

    let ClipMotionBlock {
        clip_id,
        duration,
        translations,
        rotations,
        ..
    } = block;

    match replace_clip_id {
        Some(new_clip_id) => output.push_str(new_clip_id.as_ref()),
        None => output.push_str(clip_id.as_ref()),
    };
    output.push_str("\r\n");

    output.push_str(duration.as_ref());
    output.push_str("\r\n");

    output.push_str(&format!("{}\r\n", translations.len()));
    for translation in translations {
        serialize_translation(&mut output, translation);
    }

    output.push_str(&format!("{}\r\n", rotations.len()));
    for rotation in rotations {
        serialize_rotation(&mut output, rotation);
    }

    output.push_str("\r\n"); // Empty line

    output
}

/// Serializes a translation into a string.
fn serialize_translation(ser: &mut String, translation: &Translation) {
    let Translation { time, x, y, z } = translation;

    ser.push_str(time.as_ref());
    ser.push(' ');
    ser.push_str(x.as_ref());
    ser.push(' ');
    ser.push_str(y.as_ref());
    ser.push(' ');
    ser.push_str(z.as_ref());
    ser.push_str("\r\n");
}

/// Serializes a rotation into a string.
fn serialize_rotation(ser: &mut String, rotation: &Rotation) {
    let Rotation { time, x, y, z, w } = rotation;

    ser.push_str(time.as_ref());
    ser.push(' ');
    ser.push_str(x.as_ref());
    ser.push(' ');
    ser.push_str(y.as_ref());
    ser.push(' ');
    ser.push_str(z.as_ref());
    ser.push(' ');
    ser.push_str(w.as_ref());
    ser.push_str("\r\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adsf::de::parse_adsf;

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
            "../../../../resource/assets/templates/meshes/animationdatasinglefile.bin"
        );
        let alt_adsf = rmp_serde::from_slice(alt_adsf_bytes).unwrap();
        let actual = serialize_alt_adsf(&alt_adsf);

        let expected = normalize_to_crlf(include_str!(
            "../../../../resource/xml/templates/meshes/animationdatasinglefile.txt"
        ));
        let res = dbg!(actual == expected);
        if !res {
            let diff = serde_hkx_features::diff::diff(&actual, &expected, false);
            std::fs::write("../../dummy/diff.txt", diff).unwrap();
            panic!("actual != expected");
        }
        assert!(res);
    }

    #[test]
    fn test_serialize_adsf() {
        let expected = normalize_to_crlf(include_str!(
            "../../../../resource/xml/templates/meshes/animationdatasinglefile.txt"
        ));
        let adsf = parse_adsf(&expected).unwrap_or_else(|e| panic!("{e}"));
        // std::fs::write("../../dummy/debug/adsf_debug.txt", format!("{:#?}", adsf)).unwrap();
        let actual = serialize_adsf(&adsf);

        // std::fs::create_dir_all("../../dummy").unwrap();
        // std::fs::write("../../dummy/adsf.txt", &actual).unwrap();

        let res = dbg!(actual == expected);
        if !res {
            // let diff = serde_hkx_features::diff::diff(&actual, &expected, false);
            // std::fs::write("../../dummy/diff.txt", diff).unwrap();
            panic!("actual != expected");
        }
        assert!(res);
    }
}
