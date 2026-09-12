use std::{borrow::Cow, path::PathBuf};

use fnis_list::{
    FNISList, SyntaxPattern,
    combinator::{Trigger, flags::FNISAnimFlags, fnis_animation::FNISAnimation},
    patterns::{
        pair_and_kill::{FNISPairedAndKillAnimation, FNISPairedType},
        sequenced::SequencedAnimation,
    },
};
use json_patch::{JsonPath, ValueWithPriority};
use rapidhash::fast::RapidHashSet as HashSet;
use rayon::prelude::*;
use skyrim_anim_parser::adsf::normal::{ClipAnimDataBlock, ClipMotionBlock, Rotation};

use crate::behaviors::tasks::{
    adsf::AdsfPatch,
    asdsf::AsdsfPatch,
    fnis::{
        collect::owned::OwnedFnisInjection,
        patch_gen::{
            anim_var::new_push_anim_vars_patch, chair::new_chair_patches,
            furniture::one_group::new_furniture_one_group_patches, io_jobs::AnimIoJob,
            kill_move::new_kill_patches, offset_arm::new_offset_arm_patches,
            pair::new_pair_patches,
        },
    },
};

type DashSet<K> = dashmap::DashSet<K, rapidhash::fast::RandomState>;

/// A patch with borrowed references to a single FNIS_*_List.txt file.
#[derive(Debug)]
pub(super) struct OneListPatch<'a> {
    /// A set of raw animation path entries extracted from FNIS list files.
    ///
    /// Each element represents a relative path (e.g. `"..\\sample.hkx"`)
    ///
    /// # How should this information be used?
    /// referenced by a line such as:
    ///
    /// ```text
    /// b ..\sample.hkx
    /// ```
    ///
    /// in files like `character/FNISSample/animations/FNIS_FNISSample_List.txt`.
    ///
    /// These paths correspond to entries that will later be patched into
    /// `hkbCharacterStringData.animationNames` of each default behavior file
    /// (e.g. `defaultmale.xml`) as fully qualified paths such as
    /// `Animations\FNISSample\..\sample.hkx`.
    pub animation_paths: HashSet<&'a str>,
    /// `hkbBehaviorGraphStringData.eventNames` of each master file(e.g. `0_master.xml`).
    pub events: HashSet<Cow<'a, str>>,

    /// `animationdatasinglefile.txt` patch
    pub adsf_patches: Vec<AdsfPatch<'a>>,
    /// `animationsetdatasinglefile.txt` patch
    pub asdsf_patches: Vec<AsdsfPatch<'a>>,

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

    pub conversion_jobs: Vec<AnimIoJob>,
}

/// Generate from one list file.
pub(super) fn generate_patch<'a>(
    owned_data: &'a OwnedFnisInjection,
    list: FNISList<'a>,
    config: &crate::Config,
) -> Result<OneListPatch<'a>, FnisPatchGenerationError> {
    // TODO: Support AsciiCaseIgnore
    let mut all_anim_files = HashSet::default();
    // NOTE: Currently, during the creation of the event/variable map immediately before hkx conversion in serde_hkx, duplicates are removed using ASCII ignore.
    let mut all_events = HashSet::default();

    let mut all_adsf_patches = vec![];
    let mut all_asdsf_patches = vec![];
    let mut one_master_patches = vec![];
    let mut seq_master_patches = vec![];
    let mut one_mt_behavior_patches = vec![];
    let mut seq_mt_behavior_patches = vec![];
    let mut furniture_group_root_indexes = vec![];
    let mut conversion_jobs = vec![];

    for pattern in list.patterns {
        match pattern {
            SyntaxPattern::AnimVar(anim_var) => {
                seq_master_patches.par_extend(new_push_anim_vars_patch(&[anim_var], owned_data));
            }
            SyntaxPattern::AltAnim(alt_animation) => {
                let (jobs, errs) =
                    super::alternate::alt_anim_to_oar(owned_data, alt_animation, config);
                if !errs.is_empty() {
                    return Err(FnisPatchGenerationError::FailedToConvertAltAnimToOAR {
                        errors: errs,
                    });
                }
                conversion_jobs.par_extend(jobs);
            }
            SyntaxPattern::PairAndKillMove(paired_and_kill_anim) => {
                // NOTE: It seems FNIS doesn't support `_1stperson` kill moves.
                if owned_data.behavior_entry.behavior_object != "character" {
                    return Err(FnisPatchGenerationError::UnsupportedPairAndKillMoveForCreature {
                        path: owned_data.to_list_path(),
                    });
                }

                let FNISPairedAndKillAnimation { kind, flag_set, anim_file, anim_event, .. } =
                    &paired_and_kill_anim;

                if !flag_set.flags.contains(FNISAnimFlags::Known) {
                    all_anim_files.insert(*anim_file);
                }
                all_events.extend(
                    flag_set
                        .triggers
                        .iter()
                        .chain(flag_set.triggers2.iter())
                        .map(|trigger| Cow::Borrowed(trigger.event)),
                );
                all_asdsf_patches.extend(new_asdsf_patch(owned_data, anim_event, anim_file)?);

                let (one, seq) = match kind {
                    FNISPairedType::KilMove => new_kill_patches(paired_and_kill_anim, owned_data),
                    FNISPairedType::Paired => new_pair_patches(paired_and_kill_anim, owned_data),
                };
                one_master_patches.par_extend(one);
                seq_master_patches.par_extend(seq);
            }
            SyntaxPattern::Chair(chair_animation) => {
                all_anim_files.insert(chair_animation.start.anim_file);
                all_anim_files.extend(chair_animation.sequenced.as_slice());

                let (one, seq) = new_chair_patches(&chair_animation, owned_data);
                one_mt_behavior_patches.par_extend(one);
                seq_mt_behavior_patches.par_extend(seq);
            }
            SyntaxPattern::Furniture(furniture_animation) => {
                if !owned_data.behavior_entry.is_3rd_person_character() {
                    return Err(
                        FnisPatchGenerationError::UnsupportedFurnitureAnimationToCreature {
                            path: owned_data.to_list_path(),
                        },
                    );
                }

                all_anim_files.par_extend(
                    furniture_animation.animations.par_iter().map(|fnis_anim| fnis_anim.anim_file),
                );

                // NOTE: Based on the temporal_log, it appears Furniture does not need to register its animation with behaviors like `defaultmale.xml`.
                let (one, seq, group_root_index) =
                    new_furniture_one_group_patches(&furniture_animation, owned_data);
                one_mt_behavior_patches.par_extend(one);
                seq_mt_behavior_patches.par_extend(seq);
                furniture_group_root_indexes.push(group_root_index);
            }
            SyntaxPattern::Sequenced(sequenced_animation) => {
                let (anim_files, events, adsf_patches) =
                    collect_seq_patch(owned_data, sequenced_animation);

                all_anim_files.extend(anim_files);
                all_events.extend(events);
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

                let FNISAnimation { flag_set, anim_file, .. } = &fnis_animation;

                if !flag_set.flags.contains(FNISAnimFlags::Known) {
                    all_anim_files.insert(anim_file);
                }

                let (one, seq) = new_offset_arm_patches(&fnis_animation, owned_data);
                one_mt_behavior_patches.par_extend(one);
                seq_mt_behavior_patches.par_extend(seq);
                all_adsf_patches.par_extend(new_adsf_patch(owned_data, fnis_animation));
            }
            SyntaxPattern::Basic(fnis_animation) | SyntaxPattern::AnimObject(fnis_animation) => {
                let FNISAnimation { flag_set, anim_event, anim_file, .. } = &fnis_animation;

                if !flag_set.flags.contains(FNISAnimFlags::Known) {
                    all_anim_files.insert(*anim_file);
                }
                // NOTE: According to the log, FNIS does not register events in `Basic`/`Sequenced`.
                all_events.insert(anim_event_hack(&owned_data.namespace, anim_event));

                all_adsf_patches.extend(new_adsf_patch(owned_data, fnis_animation));
            }
        };
    }

    Ok(OneListPatch {
        animation_paths: all_anim_files,
        events: all_events,
        adsf_patches: all_adsf_patches,
        asdsf_patches: all_asdsf_patches,
        one_master_patches,
        seq_master_patches,
        one_mt_behavior_patches,
        seq_mt_behavior_patches,
        furniture_group_root_indexes,
        conversion_jobs,
    })
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, snafu::Snafu)]
pub enum FnisPatchGenerationError {
    /// The addition of pairs and kill moves animation applies only to 3rd person humanoids; creatures are not supported.
    #[snafu(display("The addition of pairs and kill moves animation applies only to 3rd person humanoids; creatures are not supported.: {}", path.display()))]
    UnsupportedPairAndKillMoveForCreature { path: PathBuf },

    #[snafu(display(
        "Failed to convert alternate animation to OAR: {}", errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")
    ))]
    FailedToConvertAltAnimToOAR { errors: Vec<crate::errors::Error> },

    /// The addition of furniture animation applies only to 3rd person character; `_1stperson`, creatures are not supported.
    #[snafu(display("The addition of furniture(fu, fuo) animation applies only to 3rd person `character`; `_1stperson`, creatures are not supported.: {}", path.display()))]
    UnsupportedFurnitureAnimationToCreature { path: PathBuf },

    /// The addition of OffsetArm animation applies only to 3rd person character; `_1stperson`, creatures are not supported.
    #[snafu(display("The addition of OffsetArm(ofa) animation applies only to 3rd person `character`; `_1stperson`, creatures are not supported.: {}", path.display()))]
    UnsupportedOffsetArmAnimationToCreature { path: PathBuf },

    // AnimSet Patch error ----------
    /// The generated animation path unexpectedly has no parent directory.
    #[snafu(display("Failed to determine the parent directory of the animation path: {}", path.display()))]
    MissingAnimationParent { path: PathBuf },

    /// The parent directory of the generated animation path is not valid UTF-8.
    #[snafu(display("The parent directory of the animation path is not valid UTF-8: {}", path.display()))]
    InvalidAnimationParent { path: PathBuf },

    /// The generated animation path has no valid file stem.
    #[snafu(display("Failed to determine the file stem of the animation path: {}", path.display()))]
    MissingAnimationFileStem { path: PathBuf },
}

fn collect_seq_patch<'a>(
    owned_data: &'a OwnedFnisInjection,
    sequenced_animation: SequencedAnimation<'a>,
) -> (DashSet<&'a str>, DashSet<Cow<'a, str>>, Vec<AdsfPatch<'a>>) {
    let files = DashSet::default();
    let events = DashSet::default();

    let adsf_patches: Vec<AdsfPatch<'a>> = sequenced_animation
        .animations
        .into_iter()
        .flat_map(|fnis_animation| {
            let FNISAnimation { flag_set, anim_file, anim_event, .. } = &fnis_animation;

            if !flag_set.flags.contains(FNISAnimFlags::Known) {
                files.insert(*anim_file);
            }
            events.insert(Cow::Borrowed(*anim_event));

            new_adsf_patch(owned_data, fnis_animation)
        })
        .collect();

    (files, events, adsf_patches)
}

/// HACK:
/// Animation event name registered in behavior graph:
///
/// | Correct                       | Wrong               |
/// |-------------------------------|---------------------|
/// | `FNISFlyer_FNISfl_BackUp2_fm` | `FNISfl_BackUp2_fm` |
///
/// - FNIS_*_List.txt entry: `+ FNISfl_BackUp2 FNISfl_BackUp2.hkx`
/// - At least for FNISFlyer, FNIS prepends `<namespace>_` at behavior
///   generation time (confirmed from FNIS-generated behavior output)
///   Whether this applies to other mods is not yet verified.
fn anim_event_hack<'a>(namespace: &str, anim_event: &'a str) -> Cow<'a, str> {
    if namespace == "FNISFlyer" {
        Cow::Owned(format!("{namespace}_{anim_event}"))
    } else {
        Cow::Borrowed(anim_event)
    }
}

fn new_adsf_patch<'a>(
    owned_data: &'a OwnedFnisInjection,
    fnis_animation: FNISAnimation<'a>,
) -> Vec<AdsfPatch<'a>> {
    use crate::behaviors::tasks::adsf::PatchKind;

    let FNISAnimation { flag_set, anim_event, motions, rotations, .. } = fnis_animation;

    // Since there is no need to output adsf if there are no rotation (RD) or motion (MD) syntaxes,
    // skip it.
    if motions.is_empty() && rotations.is_empty() {
        return vec![];
    };

    let namespace = &owned_data.namespace;
    let anim_event = anim_event_hack(namespace, anim_event);

    // To link them, `translation` and `rotation` must always use the same ID.
    // use Nemesis variable(`ALltAdsf` is implemented to automatically assign IDs during serialization, so it's fine.)
    let clip_id: Cow<'a, str> = Cow::Owned(owned_data.next_adsf_id());

    let anim_block = PatchKind::AddAnim(ClipAnimDataBlock {
        name: anim_event,
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
        let rotations: Vec<Rotation<'a>> =
            rotations.into_par_iter().map(|rotation| rotation.into_rotation()).collect();

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

    let anim_data_target = owned_data.behavior_entry.anim_data_key;
    if owned_data.behavior_entry.is_3rd_person_character() {
        vec![
            AdsfPatch { target: anim_data_target, id: namespace, patch: anim_block.clone() },
            AdsfPatch { target: anim_data_target, id: namespace, patch: motion_block.clone() },
            AdsfPatch { target: "DefaultFemale~1", id: namespace, patch: anim_block },
            AdsfPatch { target: "DefaultFemale~1", id: namespace, patch: motion_block },
        ]
    } else if owned_data.behavior_entry.is_draugr() {
        // The draugr synchronizes its skeleton and animation.
        // It also synchronizes events and anim data. (It's unclear if this is actually correct)
        vec![
            AdsfPatch { target: anim_data_target, id: namespace, patch: anim_block.clone() },
            AdsfPatch { target: anim_data_target, id: namespace, patch: motion_block.clone() },
            AdsfPatch { target: "DraugrSkeletonProject~1", id: namespace, patch: anim_block },
            AdsfPatch { target: "DraugrSkeletonProject~1", id: namespace, patch: motion_block },
        ]
    } else {
        vec![
            AdsfPatch { target: anim_data_target, id: namespace, patch: anim_block },
            AdsfPatch { target: anim_data_target, id: namespace, patch: motion_block },
        ]
    }
}

/// By examining the FNIS output using `git diff`, I discovered that executing
/// `PairedAndKillMove` via `AIProcess::PlayIdle` requires applying a patch to
/// `AnimSet`.
///
/// The following is the patch resulting from that debugging.
///
/// NPC uses the original animation event, while Player uses the `pa_`-prefixed
/// event generated by FNIS.
fn new_asdsf_patch<'a>(
    owned_data: &'a OwnedFnisInjection,
    anim_event: &'a str,
    anim_file: &str,
) -> Result<Vec<AsdsfPatch<'a>>, FnisPatchGenerationError> {
    use std::path::PathBuf;

    use crate::behaviors::tasks::asdsf::PatchKind;

    let priority = owned_data.priority;
    let namespace = &owned_data.namespace;

    // NOTE: need lowercase
    let lower_anim_file = PathBuf::from(
        format!("meshes\\actors\\character\\animations\\{namespace}\\{anim_file}").to_lowercase(),
    );

    // Intended:
    // `meshes\actors\character\animations\foo\bar.hkx`
    // -> `meshes\actors\character\animations\foo`
    let parent = lower_anim_file.parent().and_then(|dir| dir.to_str()).ok_or_else(|| {
        FnisPatchGenerationError::MissingAnimationParent { path: lower_anim_file.clone() }
    })?;

    let file_stem =
        lower_anim_file.file_stem().and_then(|path| path.to_str()).ok_or_else(|| {
            FnisPatchGenerationError::MissingAnimationFileStem { path: lower_anim_file.clone() }
        })?;

    let anim_info = skyrim_anim_parser::asdsf::normal::AnimInfo {
        hashed_path: skyrim_crc::calc_crc32(parent).to_string().into(),
        hashed_file_name: skyrim_crc::calc_crc32(file_stem).to_string().into(),
        ascii_extension: Cow::Borrowed("7891816"),
    };

    // It seems there's no problem with using the same ID for both players and NPCs.
    let id = owned_data.next_asdsf_id();

    let make_patch = |anim_event: &'a str, player: bool| -> PatchKind<'a> {
        let event = if player {
            Cow::Owned(format!("pa_{anim_event}"))
        } else if namespace == "FNISFlyer" {
            Cow::Owned(format!("{namespace}_{anim_event}"))
        } else {
            Cow::Borrowed(anim_event)
        };

        PatchKind::AddAnimSet {
            patch: skyrim_anim_parser::asdsf::normal::AnimSetData {
                version: Cow::Borrowed("V3"),
                triggers_len: 1,
                triggers: vec![event],
                conditions_len: 0,
                conditions: vec![],
                attacks_len: 0,
                attacks: vec![],
                anim_infos_len: 1,
                anim_infos: vec![anim_info.clone()],
            },
            priority,
            file_name: {
                // TODO: The validity of this increment rule remains unclear.
                let prefix = if player { "Player" } else { "NPC" };
                Cow::Owned(format!("{prefix}FNIS{id}Start.txt"))
            },
        }
    };

    let npc_patch = make_patch(anim_event, false);
    let player_patch = make_patch(anim_event, true);

    Ok(vec![
        AsdsfPatch {
            target: "DefaultFemaleData~DefaultFemale",
            id: namespace,
            patch: npc_patch.clone(),
        },
        AsdsfPatch {
            target: "DefaultFemaleData~DefaultFemale",
            id: namespace,
            patch: player_patch.clone(),
        },
        AsdsfPatch { target: "DefaultMaleData~DefaultMale", id: namespace, patch: npc_patch },
        AsdsfPatch { target: "DefaultMaleData~DefaultMale", id: namespace, patch: player_patch },
    ])
}
