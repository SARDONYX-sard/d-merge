use std::borrow::Cow;
use std::collections::HashSet;
use std::path::PathBuf;

use json_patch::{JsonPath, ValueWithPriority};
use rayon::prelude::*;
use skyrim_anim_parser::adsf::normal::{ClipAnimDataBlock, ClipMotionBlock, Rotation};

use crate::behaviors::tasks::adsf::{AdsfPatch, PatchKind};
use crate::behaviors::tasks::fnis::collect::owned::OwnedFnisInjection;
use crate::behaviors::tasks::fnis::list_parser::patterns::pair_and_kill::{
    FNISPairedAndKillAnimation, FNISPairedType,
};
use crate::behaviors::tasks::fnis::list_parser::{
    combinator::{fnis_animation::FNISAnimation, Trigger},
    patterns::sequenced::SequencedAnimation,
    FNISList, SyntaxPattern,
};
use crate::behaviors::tasks::fnis::patch_gen::kill_move::new_kill_patches;
use crate::behaviors::tasks::fnis::patch_gen::pair::new_pair_patches;

#[derive(Debug)]
pub struct OneListPatch<'a> {
    /// `vec!["Animations\\<namespace>\\anim_file.hkx"]`
    pub animation_paths: HashSet<String>,
    pub events: HashSet<Cow<'a, str>>,
    pub adsf_patches: Vec<AdsfPatch<'a>>,

    /// replace one field, Add one class patches to `0_master.xml`(or each creature master xml)
    pub one_master_patches: Vec<(JsonPath<'a>, ValueWithPriority<'a>)>,
    pub seq_master_patches: Vec<(JsonPath<'a>, ValueWithPriority<'a>)>,
}

#[derive(Debug, snafu::Snafu)]
pub enum FnisPatchGenerationError {
    /// The addition of pairs and kill moves animation applies only to 3rd person humanoids; creatures are not supported.
    #[snafu(display("The addition of pairs and kill moves animation applies only to 3rd person humanoids; creatures are not supported.: {}", path.display()))]
    UnsupportedPairAndKillMoveForCreature { path: PathBuf },
}

/// Generate from one list file.
pub fn generate_patch<'a>(
    owned_data: &'a OwnedFnisInjection,
    list: FNISList<'a>,
) -> Result<OneListPatch<'a>, FnisPatchGenerationError> {
    let namespace = owned_data.namespace.as_str();

    let mut all_adsf_patches = vec![];
    let mut all_anim_files = HashSet::new();
    let mut all_events = HashSet::new();
    let mut one_master_patches = vec![];
    let mut seq_master_patches = vec![];

    for pattern in list.patterns {
        match pattern {
            SyntaxPattern::AltAnim(_alt_animation) => {
                tracing::error!("Unsupported Alternative Animation yet.");
            }
            SyntaxPattern::PairAndKillMove(paired_and_kill_animation) => {
                let FNISPairedAndKillAnimation {
                    kind,
                    flag_set,
                    anim_file,
                    anim_event,
                    ..
                } = &paired_and_kill_animation;
                all_anim_files.insert(format!("Animations\\{namespace}\\{anim_file}"));
                all_events.extend([
                    Cow::Borrowed(*anim_event),
                    Cow::Owned(format!("pa_{anim_event}")),
                ]);
                all_events.par_extend(
                    flag_set
                        .triggers
                        .par_iter()
                        .map(|trigger| Cow::Borrowed(trigger.event)),
                );
                all_events.par_extend(
                    flag_set
                        .triggers2
                        .par_iter()
                        .map(|trigger| Cow::Borrowed(trigger.event)),
                );

                // TODO: It seems FNIS doesn't support `_1stperson` kill moves.
                if owned_data.behavior_entry.behavior_object != "character" {
                    return Err(
                        FnisPatchGenerationError::UnsupportedPairAndKillMoveForCreature {
                            path: PathBuf::from(&owned_data.to_list_path()),
                        },
                    );
                }

                let (one, seq) = match kind {
                    FNISPairedType::KilMove => {
                        new_kill_patches(paired_and_kill_animation, owned_data)
                    }
                    FNISPairedType::Paired => {
                        new_pair_patches(paired_and_kill_animation, owned_data)
                    }
                };
                one_master_patches.par_extend(one);
                seq_master_patches.par_extend(seq);
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
                ) -> (Vec<String>, Vec<Cow<'a, str>>, Vec<Vec<AdsfPatch<'a>>>) {
                    sequenced_animation
                        .animations
                        .into_iter()
                        .map(|fnis_animation| {
                            let FNISAnimation {
                                anim_file,
                                anim_event,
                                ..
                            } = &fnis_animation;

                            (
                                format!("Animations\\{namespace}\\{anim_file}"),
                                Cow::Borrowed(*anim_event),
                                new_adsf_patch(owned_data, namespace, fnis_animation),
                            )
                        })
                        .collect()
                }
                fn collect_seq_creature_patch<'a>(
                    namespace: &str,
                    sequenced_animation: SequencedAnimation<'a>,
                ) -> (Vec<String>, Vec<Cow<'a, str>>) {
                    sequenced_animation
                        .animations
                        .into_par_iter()
                        .map(|fnis_animation| {
                            let FNISAnimation {
                                anim_file,
                                anim_event,
                                motions,
                                rotations,
                                ..
                            } = fnis_animation;
                            if !motions.is_empty() || !rotations.is_empty() {
                                tracing::error!(
                                    "Unsupported animationdatasinglefile.txt for Creature yet."
                                );
                            }

                            (
                                format!("Animations\\{namespace}\\{anim_file}"),
                                Cow::Borrowed(anim_event),
                            )
                        })
                        .collect()
                }

                let (anim_files, events, adsf_patches): (Vec<_>, Vec<_>, Vec<_>) =
                    if owned_data.behavior_entry.is_humanoid() {
                        collect_seq_patch(namespace, owned_data, sequenced_animation)
                    } else {
                        // TODO: Support creature adsf
                        let (anims, events) =
                            collect_seq_creature_patch(namespace, sequenced_animation);
                        (anims, events, vec![])
                    };
                all_anim_files.par_extend(anim_files);
                all_events.par_extend(events);
                all_adsf_patches.par_extend(adsf_patches.into_par_iter().flatten());
            }
            SyntaxPattern::Basic(fnis_animation) => {
                let FNISAnimation { anim_file, .. } = &fnis_animation;
                all_anim_files.insert(format!("Animations\\{namespace}\\{anim_file}"));

                let adsf_patches = new_adsf_patch(owned_data, namespace, fnis_animation);
                all_adsf_patches.par_extend(adsf_patches);
            }
        };
    }

    Ok(OneListPatch {
        animation_paths: all_anim_files,
        events: all_events,
        adsf_patches: all_adsf_patches,
        one_master_patches,
        seq_master_patches,
    })
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
