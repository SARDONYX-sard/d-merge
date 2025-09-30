use std::borrow::Cow;

use json_patch::{json_path, JsonPatch, ValueWithPriority};
use rayon::prelude::*;
use simd_json::json_typed;
use skyrim_anim_parser::adsf::normal::{ClipAnimDataBlock, ClipMotionBlock, Rotation, Translation};

use crate::behaviors::{
    tasks::{
        fnis::{
            animations::PUSH_OP,
            collect::owned::OwnedFnisInjection,
            list_parser::{
                combinator::{fnis_animation::FNISAnimation, rotation::RotationData},
                FNISList, SyntaxPattern,
            },
        },
        patches::types::{HkxPatches, OnePatchMap, SeqPatchMap},
    },
    PatchKind,
};

pub fn generate_patch<'a>(
    owned_data: &OwnedFnisInjection,
    list: FNISList<'a>,
) -> (HkxPatches<'a>, Vec<PatchKind<'a>>) {
    let namespace = owned_data.namespace.as_str();

    let mut hkx_patches = (OnePatchMap::new(), SeqPatchMap::new());
    let mut adsf_patches = vec![];
    for (index, pattern) in list.patterns.into_iter().enumerate() {
        match pattern {
            SyntaxPattern::AltAnim(alt_animation) => todo!(),
            SyntaxPattern::PairAndKillMove(paired_and_kill_animation) => todo!(),
            SyntaxPattern::Chair(chair_animation) => todo!(),
            SyntaxPattern::Furniture(furniture_animation) => todo!(),
            SyntaxPattern::Sequenced(sequenced_animation) => {
                for fnis_animation in sequenced_animation.animations {
                    let FNISAnimation {
                        anim_event,
                        motions,
                        rotations,
                        ..
                    } = fnis_animation;

                    push_new_adsf_patch(
                        namespace,
                        index,
                        &mut adsf_patches,
                        anim_event,
                        motions,
                        rotations,
                    );
                }
            }
            SyntaxPattern::Basic(fnis_animation) => {
                let FNISAnimation {
                    anim_event,
                    motions,
                    rotations,
                    ..
                } = fnis_animation;

                push_new_adsf_patch(
                    namespace,
                    index,
                    &mut adsf_patches,
                    anim_event,
                    motions,
                    rotations,
                );
            }
        };
    }

    (hkx_patches, adsf_patches)
}

fn push_new_adsf_patch<'a>(
    namespace: &str,
    index: usize,

    patches: &mut Vec<PatchKind<'a>>,
    anim_event: &'a str,
    motions: Vec<Translation<'a>>,
    rotations: Vec<RotationData<'a>>,
) {
    let rotations: Vec<Rotation<'a>> = rotations
        .into_par_iter()
        .map(|rotation| rotation.into_rotation())
        .collect();

    let duration = match (motions.is_empty(), rotations.is_empty()) {
        (true, true) => Cow::Borrowed("0.0"),
        (true, false) => motions[0].time.clone(),
        (false, true) => rotations[0].time.clone(),
        (false, false) if motions[0].time == rotations[0].time => motions[0].time.clone(),
        (false, false) => Cow::Borrowed("1.5"), // FIXME: Correct?
    };

    // To link them, translation and rotation must always use the same ID.
    let clip_id: Cow<'a, str> = Cow::Owned(format!("FNIS_{namespace}${index}")); // use Nemesis variable

    patches.push(PatchKind::AddAnim(ClipAnimDataBlock {
        name: Cow::Borrowed(anim_event),
        clip_id: clip_id.clone(),
        play_back_speed: Cow::Borrowed("1"),
        crop_start_local_time: Cow::Borrowed("0"),
        crop_end_local_time: Cow::Borrowed("0"),
        trigger_names_len: 0,
        trigger_names: vec![],
    }));

    patches.push(PatchKind::AddMotion(ClipMotionBlock {
        clip_id,
        duration,
        translation_len: motions.len(),
        translations: motions,
        rotation_len: rotations.len(),
        rotations,
    }));
}

/// Create an additional patch for the animations for one of the following template files.
/// - `meshes/actors/character/_1stperson/firstperson.xml`
/// - `meshes/actors/character/default_female/defaultfemale.xml`
/// - `meshes/actors/character/defaultmale/defaultmale.xml`
///
/// # Note
/// `animations`: Windows path to the Animations dir containing files within `meshes/actors/character/animations/<FNIS one mod namespace>/*.hkx`.
///
/// - sample animations
/// ```txt
/// [
///     "Animations\<FNIS one mod namespace>\sample.hkx",
///     "Animations\<FNIS one mod namespace>\sample1.hkx"
/// ]
/// ```
pub fn new_additional_animations_patch<'a>(
    animations: &[&'a str],
    priority: usize,
) -> (json_path::JsonPath<'static>, ValueWithPriority<'a>) {
    (
        // INFO: The destination for additions to the target template file is either coincidental or unknown,
        //       but all three share the exact same hkxcmd path.
        json_path!["#0029", "hkbCharacterStringData", "animationNames"],
        ValueWithPriority {
            patch: JsonPatch {
                op: PUSH_OP,
                value: json_typed!(borrowed, animations),
            },
            priority,
        },
    )
}
