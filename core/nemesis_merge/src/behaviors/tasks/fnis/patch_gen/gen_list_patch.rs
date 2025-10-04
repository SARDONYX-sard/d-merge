use std::borrow::Cow;

use rayon::prelude::*;
use skyrim_anim_parser::adsf::normal::{ClipAnimDataBlock, ClipMotionBlock, Rotation};

use crate::behaviors::tasks::adsf::{AdsfPatch, PatchKind};
use crate::behaviors::tasks::fnis::collect::owned::OwnedFnisInjection;
use crate::behaviors::tasks::fnis::list_parser::{
    combinator::{fnis_animation::FNISAnimation, Trigger},
    patterns::sequenced::SequencedAnimation,
    FNISList, SyntaxPattern,
};

/// Generate from one list file.
pub fn generate_patch<'a>(
    owned_data: &'a OwnedFnisInjection,
    list: FNISList<'a>,
) -> (Vec<String>, Vec<AdsfPatch<'a>>) {
    let namespace = owned_data.namespace.as_str();

    let mut all_adsf_patches = vec![];
    let mut all_anim_files = vec![];

    for pattern in list.patterns {
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
                fn collect_seq_patch<'a>(
                    namespace: &'a str,
                    owned_data: &'a OwnedFnisInjection,
                    sequenced_animation: SequencedAnimation<'a>,
                ) -> (Vec<String>, Vec<Vec<AdsfPatch<'a>>>) {
                    sequenced_animation
                        .animations
                        .into_par_iter()
                        .map(|fnis_animation| {
                            let FNISAnimation { anim_file, .. } = &fnis_animation;

                            (
                                format!("Animations\\{namespace}\\{anim_file}"),
                                new_adsf_patch(owned_data, namespace, fnis_animation),
                            )
                        })
                        .collect()
                }
                fn collect_seq_creature_patch<'a>(
                    namespace: &str,
                    sequenced_animation: SequencedAnimation<'a>,
                ) -> Vec<String> {
                    sequenced_animation
                        .animations
                        .into_par_iter()
                        .map(|fnis_animation| {
                            let FNISAnimation {
                                anim_file,
                                motions,
                                rotations,
                                ..
                            } = fnis_animation;
                            if !motions.is_empty() || !rotations.is_empty() {
                                tracing::error!(
                                    "Unsupported animationdatasinglefile.txt for Creature yet."
                                );
                            }

                            format!("Animations\\{namespace}\\{anim_file}")
                        })
                        .collect()
                }

                let (anim_files, adsf_patches): (Vec<_>, Vec<_>) =
                    if owned_data.behavior_entry.is_humanoid() {
                        collect_seq_patch(namespace, owned_data, sequenced_animation)
                    } else {
                        // TODO: Support creature adsf
                        (
                            collect_seq_creature_patch(namespace, sequenced_animation),
                            vec![],
                        )
                    };
                all_anim_files.par_extend(anim_files);
                all_adsf_patches.par_extend(adsf_patches.into_par_iter().flatten());
            }
            SyntaxPattern::Basic(fnis_animation) => {
                let FNISAnimation { anim_file, .. } = &fnis_animation;
                all_anim_files.push(format!("Animations\\{namespace}\\{anim_file}"));

                let adsf_patches = new_adsf_patch(owned_data, namespace, fnis_animation);
                all_adsf_patches.par_extend(adsf_patches);
            }
        };
    }

    (all_anim_files, all_adsf_patches)
}

fn new_adsf_patch<'a>(
    owned_data: &'a OwnedFnisInjection,
    namespace: &'a str,
    fnis_animation: FNISAnimation<'a>,
) -> Vec<AdsfPatch<'a>> {
    let FNISAnimation {
        flag_set,
        anim_event,
        motions,
        rotations,
        ..
    } = fnis_animation;

    // Since there is no need to output adsf if there are no rotation (RD) or motion (MD) syntaxes,
    // skip it.
    if motions.is_empty() && rotations.is_empty() {
        return vec![];
    };

    // To link them, translation and rotation must always use the same ID.
    // use Nemesis variable
    let clip_id: Cow<'a, str> = Cow::Owned(owned_data.next_adsf_id());

    let anim_block = PatchKind::AddAnim(ClipAnimDataBlock {
        name: Cow::Borrowed(anim_event),
        clip_id: clip_id.clone(),
        play_back_speed: Cow::Borrowed("1"),
        crop_start_local_time: Cow::Borrowed("0"),
        crop_end_local_time: Cow::Borrowed("0"),
        trigger_names_len: flag_set.triggers.len(),
        trigger_names: flag_set
            .triggers
            .into_par_iter()
            .map(|Trigger { event, time }| Cow::Owned(format!("{event}:{time}")))
            .collect(),
    });

    let motion_block = {
        let rotations: Vec<Rotation<'a>> = rotations
            .into_par_iter()
            .map(|rotation| rotation.into_rotation())
            .collect();

        let duration = match (motions.last(), rotations.last()) {
            (None, None) => Cow::Borrowed("0.000000"), // FIXME: Correct?
            (None | Some(_), Some(rd)) => rd.time.clone(),
            (Some(md), None) => md.time.clone(),
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

    // TODO: separate Creature adsf
    // Movement and rotation patches for humans (`meshes/actors/character`) are equivalent to patches
    // for both DefaultMale and DefaultFemale (since there's only one, the index is 1).
    vec![
        AdsfPatch {
            target: "DefaultMale~1",
            id: namespace,
            patch: anim_block.clone(),
        },
        AdsfPatch {
            target: "DefaultMale~1",
            id: namespace,
            patch: motion_block.clone(),
        },
        AdsfPatch {
            target: "DefaultFemale~1",
            id: namespace,
            patch: anim_block,
        },
        AdsfPatch {
            target: "DefaultFemale~1",
            id: namespace,
            patch: motion_block,
        },
    ]
}
