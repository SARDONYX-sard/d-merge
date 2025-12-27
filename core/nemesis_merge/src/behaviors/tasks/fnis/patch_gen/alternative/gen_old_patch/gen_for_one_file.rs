//! NOTE: To learn the additional method, "FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\0_master_TEMPLATE.txt"
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;

use json_patch::{Action, JsonPatch, Op, ValueWithPriority};
use rayon::prelude::*;
use simd_json::json_typed;

use crate::behaviors::tasks::fnis::collect::owned::OwnedFnisInjection;
use crate::behaviors::tasks::fnis::list_parser::{
    combinator::Trigger, patterns::alt_anim::AlternativeAnimation,
};
use crate::behaviors::tasks::fnis::patch_gen::alternative::generated_group_table::{
    ClipInfo, CHARACTER_CLIPS,
};
use crate::errors::Error;

pub struct ClipBuildResult<'a> {
    /// - It is necessary to register with `hkbManualSelectorGenerator.generators`.
    pub clip_generator_indexes: Vec<String>,

    /// `hkbClipGenerator`/`hkbClipTriggerArray` patches
    pub one_patches: Vec<(Vec<Cow<'a, str>>, ValueWithPriority<'a>)>,

    pub owned_data: &'a OwnedFnisInjection,

    pub clip_info: &'static ClipInfo,
}

///Generate patches for FNIS Alternate Animation:
/// - `hkbClipGenerator`
/// - `hkbTrigger`
///
/// At this stage, the indexes to be placed in `Selector.generators` are not yet determined.
/// Therefore, Selector generation and generator registration occur immediately before
/// integrating the FNIS patches into the Nemesis patch.
pub fn new_alt_patches<'a>(
    alt_animation: &'a AlternativeAnimation<'a>,
    owned_data: &'a OwnedFnisInjection,
    priority: usize,
) -> (HashMap<&'a str, ClipBuildResult<'a>>, Vec<Error>) {
    let prefix = alt_animation.prefix;
    let mut errors = vec![];

    // anim_name -> triggers
    let trigger_map: HashMap<&str, &Vec<Trigger>> = alt_animation
        .trigger
        .iter()
        .map(|t| (t.anim_name, &t.triggers))
        .collect();

    // key = alt_group_file (anim_name)
    let mut map: HashMap<&str, ClipBuildResult> = HashMap::new();

    for set in &alt_animation.set {
        let slots = set.slots;
        let group_name = set.group;

        let Some(&group) = CHARACTER_CLIPS.get(group_name) else {
            errors.push(Error::Custom {
                msg: format!("There is no such FNIS Alt group.: {group_name}"),
            });
            continue;
        };

        for slot in 0..slots {
            for clip_info in group {
                let animation = clip_info.alt_group_file;
                let animation_path = Path::new(animation);

                let animation_path = if let (Some(parent), Some(file_name)) = (
                    animation_path.parent(),
                    animation_path.file_name().and_then(|s| s.to_str()),
                ) {
                    format!("{}\\{prefix}{slot}_{file_name}", parent.display())
                } else {
                    format!("{prefix}{slot}_{animation}")
                };

                // FIXME?: Unnecessary this condition?
                if !owned_data.animations_mod_dir.join(&animation_path).exists() {
                    continue;
                }

                let entry = map.entry(animation).or_insert_with(|| ClipBuildResult {
                    clip_generator_indexes: Vec::new(),
                    one_patches: Vec::new(),
                    owned_data,
                    clip_info,
                });

                // ---- TriggerArray ----
                let trigger_array_ptr = if let Some(triggers) = trigger_map.get(animation) {
                    let trigger_ptr = owned_data.next_class_name_attribute();
                    entry.one_patches.push(new_clip_trigger_array(
                        &trigger_ptr,
                        priority,
                        &triggers.iter().collect::<Vec<_>>(),
                    ));
                    Some(trigger_ptr)
                } else {
                    None
                };

                // ---- ClipGenerator ----
                let clip_generator_index = owned_data.next_class_name_attribute();

                entry.one_patches.push(make_alt_clip_generator_patch(
                    &clip_generator_index,
                    &animation_path,
                    trigger_array_ptr.as_deref(),
                    priority,
                    clip_info,
                ));

                entry.clip_generator_indexes.push(clip_generator_index);
            }
        }
    }

    (map, errors)
}

#[must_use]
pub fn make_alt_clip_generator_patch<'a>(
    class_index: &str,
    animation_name: &str,
    trigger_array: Option<&str>, // Some("#xxxx") or None
    priority: usize,
    clip_info: &'static ClipInfo,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    (
        vec![
            Cow::Owned(class_index.to_string()),
            Cow::Borrowed("hkbClipGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_index,
                    "variableBindingSet": clip_info.raw.variable_binding_set,
                    "userData": 0,
                    "name": clip_info.raw.variable_binding_set,
                    "animationName": animation_name, // Alternate anim path e.g., `Animations\\fff_3_Jump.hkx`
                    "triggers": trigger_array.unwrap_or("#0000"),
                    "cropStartAmountLocalTime": clip_info.raw.crop_start_amount_local_time,
                    "cropEndAmountLocalTime": clip_info.raw.crop_end_amount_local_time,
                    "startTime": clip_info.raw.start_time,
                    "playbackSpeed": clip_info.raw.playback_speed,
                    "enforcedDuration": clip_info.raw.enforced_duration,
                    "userControlledTimeFraction": clip_info.raw.user_controlled_time_fraction,
                    "animationBindingIndex": clip_info.raw.animation_binding_index,
                    "mode": clip_info.raw.mode,
                    "flags": clip_info.raw.flags
                }),
            },
            priority,
        },
    )
}

#[must_use]
fn new_clip_trigger_array<'a>(
    new_index: &str,
    priority: usize,
    triggers: &[&Trigger<'a>],
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    let triggers_last_index = triggers.len() - 1;
    let triggers: Vec<_> = triggers
        .par_iter()
        .enumerate()
        .map(|(index, Trigger { event, time })| {
            json_typed!(borrowed, {
                "localTime": time, // $&aaTt.<group_name>.<number>$
                "event": {
                    "id": format!("$eventID[{event}]$"), // use Nemesis eventID variable. instead of $&aaTe.<group_name>.<number>$
                    "payload": "#0000"
                },
                "relativeToEndOfClip": triggers_last_index == index, // FIXME?: $&aaTt-.<group_name>.<number>$
                "acyclic": false,
                "isAnnotation": false
            })
        })
        .collect();

    (
        vec![
            Cow::Owned(new_index.to_string()),
            Cow::Borrowed("hkbClipTriggerArray"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                    "__ptr": new_index,
                    "triggers": triggers,
                }),
            },
            priority,
        },
    )
}
