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
    output.push_str(&format!("{}\n", adsf.project_names.len()));
    for name in &adsf.project_names {
        output.push_str(&format!("{name}\n"));
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
    if anim_data.header.has_motion_data {
        for block in &anim_data.clip_motion_blocks {
            output.push_str(&serialize_clip_motion_block(block));
        }
    }

    output
}

/// Serializes the animation data header into a string.
///
/// # Errors
/// Returns an error if serialization fails.
fn serialize_anim_header(header: &AnimDataHeader) -> String {
    let mut output = String::new();

    output.push_str(&format!("{}\n", header.line_range));
    output.push_str(&format!("{}\n", header.lead_int));
    output.push_str(&format!("{}\n", header.project_assets_len));
    for asset in &header.project_assets {
        output.push_str(&format!("{asset}\n"));
    }
    output.push_str(&format!("{}\n", if header.has_motion_data { 1 } else { 0 }));

    output
}

/// Serializes a clip animation data block into a string.
fn serialize_clip_anim_block(block: &ClipAnimDataBlock) -> String {
    let mut output = String::new();

    output.push_str(&format!("{}\n", block.name));
    output.push_str(&format!("{}\n", block.clip_id));
    output.push_str(&format!("{:.}\n", block.play_back_speed));
    output.push_str(&format!("{:.}\n", block.crop_start_local_time));
    output.push_str(&format!("{:.}\n", block.crop_end_local_time));
    output.push_str(&format!("{}\n", block.trigger_names_len));
    for trigger in &block.trigger_names {
        output.push_str(&format!("{trigger}\n"));
    }
    output.push('\n'); // Empty line

    output
}

/// Serializes a clip motion block into a string.
fn serialize_clip_motion_block(block: &ClipMotionBlock) -> String {
    let mut output = String::new();

    output.push_str(&format!("{}\n", block.clip_id));
    output.push_str(&format!("{:.}\n", block.duration));
    output.push_str(&format!("{}\n", block.translation_len));
    for translation in &block.translations {
        output.push_str(&serialize_translation(translation));
    }
    output.push_str(&format!("{}\n", block.rotation_len));
    for rotation in &block.rotations {
        output.push_str(&serialize_rotation(rotation));
    }
    output.push('\n'); // Empty line

    output
}

/// Serializes a translation into a string.
fn serialize_translation(translation: &Translation) -> String {
    format!(
        "{:.} {:.} {:.} {:.}\n",
        translation.time, translation.x, translation.y, translation.z
    )
}

/// Serializes a rotation into a string.
fn serialize_rotation(rotation: &Rotation) -> String {
    format!(
        "{:.} {:.} {:.} {:.} {:.}\n",
        rotation.time, rotation.x, rotation.y, rotation.z, rotation.w
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adsf::de::parse_adsf;

    #[ignore = "Not complete yet"]
    #[test]
    fn test_serialize_adsf() {
        let expected = include_str!(
            "../../../../resource/assets/templates/meshes/animationdatasinglefile.txt"
        );
        let adsf = parse_adsf(expected).unwrap();
        let actual = serialize_adsf(&adsf);

        let diff = serde_hkx_features::diff::diff(&actual, expected, false);
        let res = dbg!(actual != expected);
        if res {
            std::fs::write("../../dummy/diff.txt", diff).unwrap();
            panic!("actual != expected");
        }
        println!("Ok!");
    }
}
