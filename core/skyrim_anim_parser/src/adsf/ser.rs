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
        output.push_str(&format!("{name}\r\n"));
    }

    // Serialize animation data
    for anim_data in &adsf.anim_list {
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
    output.push_str(&serialize_anim_header(&anim_data.header));

    // Serialize clip animation blocks
    for block in &anim_data.clip_anim_blocks {
        output.push_str(&serialize_clip_anim_block(block));
    }

    // Serialize clip motion blocks if present
    let clip_motion_blocks_len = count_clip_motion_lines(&anim_data.clip_motion_blocks);
    if clip_motion_blocks_len > 0 {
        output.push_str(&format!("{clip_motion_blocks_len}\r\n"));
    };
    if anim_data.header.has_motion_data {
        for block in &anim_data.clip_motion_blocks {
            output.push_str(&serialize_clip_motion_block(block));
        }
    }

    output
}

fn count_clip_motion_lines(blocks: &[ClipMotionBlock]) -> usize {
    blocks
        .iter()
        .map(|block| {
            // empty line: 1(clip_id) + 1(duration) + 1(translation_len) + 1(rotation_len) + 1(empty_line) = 5
            let base_lines = 5;
            let translation_lines = block.translations.len();
            let rotation_lines = block.rotations.len();
            base_lines + translation_lines + rotation_lines
        })
        .sum()
}

/// Serializes the animation data header into a string.
///
/// # Errors
/// Returns an error if serialization fails.
fn serialize_anim_header(header: &AnimDataHeader) -> String {
    let mut output = String::new();

    output.push_str(&header.line_range.to_string());
    output.push_str("\r\n");
    output.push_str(&header.lead_int.to_string());
    output.push_str("\r\n");
    output.push_str(&header.project_assets_len.to_string());
    output.push_str("\r\n");

    for asset in &header.project_assets {
        output.push_str(&format!("{asset}\r\n"));
    }
    output.push_str(&format!(
        "{}\r\n",
        if header.has_motion_data { 1 } else { 0 }
    ));

    output
}

/// Serializes a clip animation data block into a string.
fn serialize_clip_anim_block(block: &ClipAnimDataBlock) -> String {
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

    output.push_str(clip_id.as_ref());
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
fn serialize_clip_motion_block(block: &ClipMotionBlock) -> String {
    let mut output = String::new();

    let ClipMotionBlock {
        clip_id,
        duration,
        translation_len,
        translations,
        rotation_len,
        rotations,
    } = block;

    output.push_str(clip_id.as_ref());
    output.push_str("\r\n");

    output.push_str(duration.as_ref());
    output.push_str("\r\n");

    output.push_str(&format!("{translation_len}\r\n"));
    for translation in translations {
        serialize_translation(&mut output, translation);
    }

    output.push_str(&format!("{rotation_len}\r\n"));
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

    #[test]
    fn test_serialize_adsf() {
        let expected = include_str!(
            "../../../../resource/assets/templates/meshes/animationdatasinglefile.txt"
        )
        .replace('\n', "\r\n");
        let adsf = parse_adsf(&expected).unwrap();
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
