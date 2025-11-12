use std::borrow::Cow;
use std::collections::HashSet;
use std::path::PathBuf;

use dashmap::DashSet;
use json_patch::{JsonPath, ValueWithPriority};
use rayon::prelude::*;
use skyrim_anim_parser::adsf::normal::{ClipAnimDataBlock, ClipMotionBlock, Rotation};

use crate::behaviors::tasks::adsf::{AdsfPatch, PatchKind};
use crate::behaviors::tasks::fnis::collect::owned::OwnedFnisInjection;
use crate::behaviors::tasks::fnis::list_parser::combinator::flags::FNISAnimFlags;
use crate::behaviors::tasks::fnis::list_parser::patterns::pair_and_kill::{
    FNISPairedAndKillAnimation, FNISPairedType,
};
use crate::behaviors::tasks::fnis::list_parser::{
    combinator::{fnis_animation::FNISAnimation, Trigger},
    patterns::sequenced::SequencedAnimation,
    FNISList, SyntaxPattern,
};
use crate::behaviors::tasks::fnis::patch_gen::anim_var::new_push_anim_vars_patch;
use crate::behaviors::tasks::fnis::patch_gen::furniture::one_group::new_furniture_one_group_patches;
use crate::behaviors::tasks::fnis::patch_gen::{
    kill_move::new_kill_patches, offset_arm::new_offset_arm_patches, pair::new_pair_patches,
};

/// A patch with borrowed references to a single FNIS_*_List.txt file.
#[derive(Debug)]
pub struct OneListPatch<'a> {
    /// `hkbCharacterStringData.animationNames` of each default(behavior) file(e.g. `defaultmale.xml`).
    pub animation_paths: HashSet<String>,
    /// `hkbBehaviorGraphStringData.eventNames` of each master file(e.g. `0_master.xml`).
    pub events: HashSet<Cow<'a, str>>,
    /// `animationdatasinglefile.txt` patch
    ///
    /// That txt file actually has the key first, followed by a sequence of values.
    /// This is an array of those key(target)-value pairs.
    pub adsf_patches: Vec<AdsfPatch<'a>>,
    /// Add/Replace one field/class patches to master file(e.g. `0_master.xml`).
    pub one_master_patches: Vec<(JsonPath<'a>, ValueWithPriority<'a>)>,
    /// Add/Replace/Remove array field patches to master file(e.g. `0_master.xml`).
    pub seq_master_patches: Vec<(JsonPath<'a>, ValueWithPriority<'a>)>,

    /// Add/Replace one field/class patches to master file(e.g. `mt_behavior.xml`).
    pub one_mt_behavior_patches: Vec<(JsonPath<'a>, ValueWithPriority<'a>)>,
    /// Add/Replace/Remove array field patches to master file(e.g. `mt_behavior.xml`).
    pub seq_mt_behavior_patches: Vec<(JsonPath<'a>, ValueWithPriority<'a>)>,

    /// One group of furniture syntax must be pushed to the states of the Furniture root.
    /// Therefore, it is placed here to be pushed when the furniture root is generated.
    pub furniture_group_root_indexes: Vec<String>,
}

/// Generate from one list file.
pub fn generate_patch<'a>(
    owned_data: &'a OwnedFnisInjection,
    list: FNISList<'a>,
) -> Result<OneListPatch<'a>, FnisPatchGenerationError> {
    // TODO: Support AsciiCaseIgnore
    let mut all_anim_files = HashSet::new();
    // NOTE: Currently, during the creation of the event/variable map immediately before hkx conversion in serde_hkx, duplicates are removed using ASCII ignore.
    let mut all_events = HashSet::new();

    let mut all_adsf_patches = vec![];
    let mut one_master_patches = vec![];
    let mut seq_master_patches = vec![];
    let mut one_mt_behavior_patches = vec![];
    let mut seq_mt_behavior_patches = vec![];
    let mut furniture_group_root_indexes = vec![];

    let namespace = owned_data.namespace.as_str();
    for pattern in list.patterns {
        match pattern {
            SyntaxPattern::AnimVar(anim_var) => {
                seq_master_patches.par_extend(new_push_anim_vars_patch(&[anim_var], owned_data));
            }
            SyntaxPattern::AltAnim(_alt_animation) => {
                return Err(FnisPatchGenerationError::UnsupportedAltAnimation {
                    path: owned_data.to_list_path(),
                });
            }
            SyntaxPattern::PairAndKillMove(paired_and_kill_anim) => {
                // NOTE: It seems FNIS doesn't support `_1stperson` kill moves.
                if owned_data.behavior_entry.behavior_object != "character" {
                    return Err(
                        FnisPatchGenerationError::UnsupportedPairAndKillMoveForCreature {
                            path: owned_data.to_list_path(),
                        },
                    );
                }

                let FNISPairedAndKillAnimation {
                    kind,
                    flag_set,
                    anim_file,
                    ..
                } = &paired_and_kill_anim;

                if !flag_set.flags.contains(FNISAnimFlags::Known) {
                    all_anim_files.insert(format!("Animations\\{namespace}\\{anim_file}"));
                }
                all_events.par_extend(
                    flag_set
                        .triggers
                        .par_iter()
                        .chain(flag_set.triggers2.par_iter())
                        .map(|trigger| Cow::Borrowed(trigger.event)),
                );

                let (one, seq) = match kind {
                    FNISPairedType::KilMove => new_kill_patches(paired_and_kill_anim, owned_data),
                    FNISPairedType::Paired => new_pair_patches(paired_and_kill_anim, owned_data),
                };
                one_master_patches.par_extend(one);
                seq_master_patches.par_extend(seq);
            }
            SyntaxPattern::Chair(_chair_animation) => {
                return Err(FnisPatchGenerationError::UnsupportedChairAnimation {
                    path: owned_data.to_list_path(),
                });
            }
            SyntaxPattern::Furniture(furniture_animation) => {
                if !owned_data.behavior_entry.is_3rd_person_character() {
                    return Err(
                        FnisPatchGenerationError::UnsupportedFurnitureAnimationToCreature {
                            path: owned_data.to_list_path(),
                        },
                    );
                }

                let (one, seq, group_root_index) =
                    new_furniture_one_group_patches(&furniture_animation, owned_data);
                one_mt_behavior_patches.par_extend(one);
                seq_mt_behavior_patches.par_extend(seq);
                furniture_group_root_indexes.push(group_root_index);
            }
            SyntaxPattern::Sequenced(sequenced_animation) => {
                let (anim_files, events, adsf_patches) =
                    collect_seq_patch(owned_data, sequenced_animation);

                all_anim_files.par_extend(anim_files);
                all_events.par_extend(events);
                all_adsf_patches.par_extend(adsf_patches);
            }
            SyntaxPattern::OffsetArm(fnis_animation) => {
                if !owned_data.behavior_entry.is_3rd_person_character() {
                    return Err(
                        FnisPatchGenerationError::UnsupportedOffsetArmAnimationToCreature {
                            path: owned_data.to_list_path(),
                        },
                    );
                }

                let FNISAnimation {
                    flag_set,
                    anim_file,
                    ..
                } = &fnis_animation;

                if !flag_set.flags.contains(FNISAnimFlags::Known) {
                    all_anim_files.insert(format!("Animations\\{namespace}\\{anim_file}"));
                }

                let (one, seq) = new_offset_arm_patches(&fnis_animation, owned_data);
                one_mt_behavior_patches.par_extend(one);
                seq_mt_behavior_patches.par_extend(seq);
                all_adsf_patches.par_extend(new_adsf_patch(owned_data, fnis_animation));
            }
            SyntaxPattern::Basic(fnis_animation) | SyntaxPattern::AnimObject(fnis_animation) => {
                let FNISAnimation {
                    flag_set,
                    anim_event,
                    anim_file,
                    ..
                } = &fnis_animation;

                if !flag_set.flags.contains(FNISAnimFlags::Known) {
                    all_anim_files.insert(format!("Animations\\{namespace}\\{anim_file}"));
                }
                // NOTE: According to the log, FNIS does not register events in `Basic`/`Sequenced`.
                all_events.insert(Cow::Borrowed(anim_event));
                all_adsf_patches.par_extend(new_adsf_patch(owned_data, fnis_animation));
            }
        };
    }

    Ok(OneListPatch {
        animation_paths: all_anim_files,
        events: all_events,
        adsf_patches: all_adsf_patches,
        one_master_patches,
        seq_master_patches,
        one_mt_behavior_patches,
        seq_mt_behavior_patches,
        furniture_group_root_indexes,
    })
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, snafu::Snafu)]
pub enum FnisPatchGenerationError {
    /// The addition of pairs and kill moves animation applies only to 3rd person humanoids; creatures are not supported.
    #[snafu(display("The addition of pairs and kill moves animation applies only to 3rd person humanoids; creatures are not supported.: {}", path.display()))]
    UnsupportedPairAndKillMoveForCreature { path: PathBuf },

    /// Alternative animation is not supported yet
    #[snafu(display("Alternative Animation is not supported yet: {}", path.display()))]
    UnsupportedAltAnimation { path: PathBuf },

    /// Chair animation is not supported yet
    #[snafu(display("Chair Animation is not supported yet: {}", path.display()))]
    UnsupportedChairAnimation { path: PathBuf },

    /// The addition of furniture animation applies only to 3rd person character; `_1stperson`, creatures are not supported.
    #[snafu(display("The addition of furniture(fu, fuo) animation applies only to 3rd person `character`; `_1stperson`, creatures are not supported.: {}", path.display()))]
    UnsupportedFurnitureAnimationToCreature { path: PathBuf },

    /// The addition of OffsetArm animation applies only to 3rd person character; `_1stperson`, creatures are not supported.
    #[snafu(display("The addition of OffsetArm(ofa) animation applies only to 3rd person `character`; `_1stperson`, creatures are not supported.: {}", path.display()))]
    UnsupportedOffsetArmAnimationToCreature { path: PathBuf },
}

fn collect_seq_patch<'a>(
    owned_data: &'a OwnedFnisInjection,
    sequenced_animation: SequencedAnimation<'a>,
) -> (DashSet<String>, DashSet<Cow<'a, str>>, Vec<AdsfPatch<'a>>) {
    let files = DashSet::new();
    let events = DashSet::new();

    let adsf_patches: Vec<AdsfPatch<'a>> = sequenced_animation
        .animations
        .into_par_iter()
        .flat_map(|fnis_animation| {
            let namespace = &owned_data.namespace;
            let FNISAnimation {
                flag_set,
                anim_file,
                anim_event,
                ..
            } = &fnis_animation;

            if !flag_set.flags.contains(FNISAnimFlags::Known) {
                files.insert(format!("Animations\\{namespace}\\{anim_file}"));
            }
            events.insert(Cow::Borrowed(*anim_event));

            new_adsf_patch(owned_data, fnis_animation)
        })
        .collect();

    (files, events, adsf_patches)
}

fn new_adsf_patch<'a>(
    owned_data: &'a OwnedFnisInjection,
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

    // To link them, `translation` and `rotation` must always use the same ID.
    // use Nemesis variable(`ALltAdsf` is implemented to automatically assign IDs during serialization, so it's fine.)
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
            (None, None) => Cow::Borrowed("0.000000"), // NOTE: Unreachable. The empty check has already been done above.
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

    let namespace = &owned_data.namespace;
    let anim_data_target = owned_data.behavior_entry.anim_data_key;
    if owned_data.behavior_entry.is_3rd_person_character() {
        vec![
            AdsfPatch {
                target: anim_data_target,
                id: namespace,
                patch: anim_block.clone(),
            },
            AdsfPatch {
                target: anim_data_target,
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
    } else if owned_data.behavior_entry.is_draugr() {
        // The draugr synchronizes its skeleton and animation.
        // It also synchronizes events and anim data. (It's unclear if this is actually correct)
        vec![
            AdsfPatch {
                target: anim_data_target,
                id: namespace,
                patch: anim_block.clone(),
            },
            AdsfPatch {
                target: anim_data_target,
                id: namespace,
                patch: motion_block.clone(),
            },
            AdsfPatch {
                target: "DraugrSkeletonProject~1",
                id: namespace,
                patch: anim_block,
            },
            AdsfPatch {
                target: "DraugrSkeletonProject~1",
                id: namespace,
                patch: motion_block,
            },
        ]
    } else {
        vec![
            AdsfPatch {
                target: anim_data_target,
                id: namespace,
                patch: anim_block,
            },
            AdsfPatch {
                target: anim_data_target,
                id: namespace,
                patch: motion_block,
            },
        ]
    }
}
