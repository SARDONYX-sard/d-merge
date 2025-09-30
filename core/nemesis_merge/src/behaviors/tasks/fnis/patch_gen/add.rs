use std::borrow::Cow;

use json_patch::{json_path, JsonPatch, ValueWithPriority};
use rayon::prelude::*;
use simd_json::json_typed;
use skyrim_anim_parser::adsf::normal::{ClipAnimDataBlock, ClipMotionBlock, Rotation, Translation};

use crate::behaviors::tasks::adsf::{AdsfPatch, PatchKind};
use crate::behaviors::tasks::fnis::{
    collect::owned::OwnedFnisInjection,
    list_parser::{
        combinator::{fnis_animation::FNISAnimation, rotation::RotationData},
        FNISList, SyntaxPattern,
    },
    patch_gen::PUSH_OP,
};
use crate::behaviors::tasks::patches::types::{HkxPatches, OnePatchMap, SeqPatchMap};

pub fn generate_patch<'a>(
    owned_data: &'a OwnedFnisInjection,
    list: FNISList<'a>,
) -> (HkxPatches<'a>, Vec<AdsfPatch<'a>>) {
    let namespace = owned_data.namespace.as_str();
    let priority = owned_data.priority;

    let hkx_patches = (OnePatchMap::new(), SeqPatchMap::new());
    {
        let animations = &owned_data.animation_paths;
        let (json_path, patch) = new_add_anim_seq_patch(animations, priority);
        hkx_patches.1.insert(json_path, patch);
    }

    // push_mod_root_behavior(&hkx_patches, owned_data);

    let mut adsf_patches = vec![];
    for (index, pattern) in list.patterns.into_iter().enumerate() {
        match pattern {
            SyntaxPattern::AltAnim(_alt_animation) => {
                tracing::error!("Unsupported Alternative Animation yet.");
            }
            SyntaxPattern::PairAndKillMove(_paired_and_kill_animation) => {
                tracing::error!("Unsupported PairAndKillMove Animation yet.");
            }
            SyntaxPattern::Chair(_chair_animation) => {
                tracing::error!("Unsupported Chair Animation yet.");
            }
            SyntaxPattern::Furniture(_furniture_animation) => {
                tracing::error!("Unsupported Furniture Animation yet.");
            }
            SyntaxPattern::Sequenced(sequenced_animation) => {
                for fnis_animation in sequenced_animation.animations {
                    let FNISAnimation {
                        anim_event,
                        motions,
                        rotations,
                        ..
                    } = fnis_animation;

                    push_new_adsf_patch(
                        &mut adsf_patches,
                        namespace,
                        index,
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
                    &mut adsf_patches,
                    namespace,
                    index,
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
    patches: &mut Vec<AdsfPatch<'a>>,
    namespace: &'a str,
    index: usize,
    anim_event: &'a str,
    motions: Vec<Translation<'a>>,
    rotations: Vec<RotationData<'a>>,
) {
    // To link them, translation and rotation must always use the same ID.
    let clip_id: Cow<'a, str> = Cow::Owned(format!("FNIS_{namespace}${index}")); // use Nemesis variable

    let anim_block = PatchKind::AddAnim(ClipAnimDataBlock {
        name: Cow::Borrowed(anim_event),
        clip_id: clip_id.clone(),
        play_back_speed: Cow::Borrowed("1"),
        crop_start_local_time: Cow::Borrowed("0"),
        crop_end_local_time: Cow::Borrowed("0"),
        trigger_names_len: 0,
        trigger_names: vec![],
    });

    let motion_block = {
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

        PatchKind::AddMotion(ClipMotionBlock {
            clip_id,
            duration,
            translation_len: motions.len(),
            translations: motions,
            rotation_len: rotations.len(),
            rotations,
        })
    };

    // Movement and rotation patches for humans (`meshes/actors/character`) are equivalent to patches
    // for both DefaultMale and DefaultFemale (since there's only one, the index is 1).
    // TODO: separate Creature adsf
    patches.push(AdsfPatch {
        target: "DefaultMale~1",
        id: namespace,
        patch: anim_block.clone(),
    });
    patches.push(AdsfPatch {
        target: "DefaultMale~1",
        id: namespace,
        patch: motion_block.clone(),
    });

    patches.push(AdsfPatch {
        target: "DefaultFemale~1",
        id: namespace,
        patch: anim_block,
    });
    patches.push(AdsfPatch {
        target: "DefaultFemale~1",
        id: namespace,
        patch: motion_block,
    });
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
pub fn new_add_anim_seq_patch<'a>(
    animations: &[String],
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
