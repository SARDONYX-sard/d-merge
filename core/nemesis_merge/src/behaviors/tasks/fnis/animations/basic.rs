// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Pandora Behaviour Engine Contributors
//
// This is based on the logic of Pandora-Behaviour-Engine-Plus.

use std::borrow::Cow;

use json_patch::{json_path, JsonPatch, Op, OpRangeKind, ValueWithPriority};
use simd_json::json_typed;

use crate::behaviors::tasks::{
    fnis::{
        animations::PUSH_OP,
        list_parser::combinator::{flags::FNISAnimFlags, fnis_animation::FNISAnimation},
    },
    patches::types::{OnePatchMap, SeqPatchMap},
};

/// Context for patch building.
pub struct PatchContext<'a, 'b> {
    pub one: &'b OnePatchMap<'a>,
    pub seq: &'b SeqPatchMap<'a>,
    pub priority: usize,

    /// `hkbStateMachineStateInfo` index e.g. `#0000`
    pub index: &'a str,
    /// Replace target(`hkbClipTriggerArray`) index
    pub clip_id: &'a str,

    pub trigger_pushed: bool,
    /// creation index. e.g. `#{mod_code}${index}`
    pub new_clip_gen_id: &'a str,
}

/// Notify event type for state machine transitions.
enum NotifyEvent {
    Enter,
    Exit,
}
impl NotifyEvent {
    /// to C++ field name
    const fn to_field_name(&self) -> &'static str {
        match self {
            Self::Enter => "enterNotifyEvents",
            Self::Exit => "exitNotifyEvents",
        }
    }
}

/// Build all patches for a sequence of FNIS animations.
///
/// - Inserts clip mode (`MODE_SINGLE_PLAY` or `MODE_LOOPING`)
/// - Adds animation object notify events (393, 394, 165)
/// - Handles sequence start (first animation) and finish (last animation)
///
/// # Parameters
/// - `anims`: slice of FNIS animations forming a sequence
/// - `ctx`: patch context for inserting patches
pub fn build_animation_sequence<'a>(anims: &[FNISAnimation<'a>], ctx: &mut PatchContext<'a, '_>) {
    if anims.is_empty() {
        return;
    }
    let last_index = anims.len() - 1;

    for (i, anim) in anims.iter().enumerate() {
        // Common patch generation for every animation
        build_animation_common(anim, ctx);

        // Sequence start → first animation
        if i == 0 {
            if let Some(next) = anims.get(1) {
                let start_event_var = Cow::Owned(format!("$eventID[{}]", next.anim_event));
                push_event_name(ctx, &start_event_var);
                push_trigger(ctx, -0.3, &start_event_var);
            }
        }

        // Sequence finish → last animation
        if i == last_index {
            {
                let done_event_var = format!("$eventID[{}_DONE]", anim.anim_event);
                push_event_name(ctx, &done_event_var);
                push_trigger(ctx, -0.2, &done_event_var);
            }

            push_trigger(ctx, -0.05, "$eventID[IdleForceDefaultState]$");
        }
    }

    finish(ctx);
}

/// Build common patches for a single FNIS animation.
///
/// - Inserts clip mode (`MODE_SINGLE_PLAY` or `MODE_LOOPING`)
/// - Adds animation object notify events (393, 394, 165)
fn build_animation_common<'a>(anim: &FNISAnimation<'a>, ctx: &PatchContext<'a, '_>) {
    replace_mode(ctx, anim.flag_set.flags);

    if anim.flag_set.flags.contains(FNISAnimFlags::AnimObjects) {
        for name in &anim.anim_objects {
            let payload = json_typed!(borrowed, { "data": name }); // hkbStringEventPayload

            push_notify_event(ctx, NotifyEvent::Enter, 393, Some(&payload));
            push_notify_event(ctx, NotifyEvent::Enter, 394, Some(&payload));

            if !anim.flag_set.flags.contains(FNISAnimFlags::Sticky) {
                push_notify_event(ctx, NotifyEvent::Exit, 165, None);
            }
        }
    }
}

/// Insert or replace the clip mode in `hkbClipGenerator`.
fn replace_mode(ctx: &PatchContext<'_, '_>, flags: FNISAnimFlags) {
    let mode = if flags.contains(FNISAnimFlags::Acyclic) {
        "MODE_SINGLE_PLAY"
    } else {
        "MODE_LOOPING"
    };

    ctx.one.insert(
        json_path![ctx.clip_id, "hkbClipGenerator", "mode"],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Replace),
                value: json_typed!(borrowed, mode),
            },
            priority: ctx.priority,
        },
    );
}

/// Push a notify event into `hkbStateMachineStateInfo`.
fn push_notify_event<'a>(
    ctx: &PatchContext<'a, '_>,
    event: NotifyEvent,
    id: impl serde::Serialize,
    payload: Option<&simd_json::BorrowedValue<'a>>,
) {
    let json_path = json_path![
        ctx.index,
        "hkbStateMachineStateInfo",
        event.to_field_name(),
        "events"
    ];
    ctx.seq.insert(
        json_path,
        ValueWithPriority {
            patch: JsonPatch {
                op: PUSH_OP,
                value: json_typed!(borrowed, {
                    "id": id,
                    "payload": payload
                }),
            },
            priority: ctx.priority,
        },
    );
}

/// Push to `eventNames` in `mt_behavior.xml(from hkxcmd).hkbBehaviorGraphStringData(#0083)`.
fn push_event_name(ctx: &PatchContext<'_, '_>, event_name: &str) {
    ctx.seq.insert(
        json_path!["#0083", "hkbBehaviorGraphStringData", "eventNames"],
        ValueWithPriority {
            patch: JsonPatch {
                op: PUSH_OP,
                value: json_typed!(borrowed, event_name),
            },
            priority: ctx.priority,
        },
    );
}

/// Push a trigger into `hkbClipTriggerArray.triggers`.
fn push_trigger(ctx: &mut PatchContext<'_, '_>, local_time: f32, event_id: &str) {
    ctx.seq.insert(
        json_path![ctx.clip_id, "hkbClipTriggerArray", "triggers"],
        ValueWithPriority {
            patch: JsonPatch {
                op: PUSH_OP,
                value: json_typed!(borrowed, {
                    "localTime": local_time,
                    "relativeToEndOfClip": true,
                    "event": {
                        "id": event_id
                    }
                }),
            },
            priority: ctx.priority,
        },
    );
    ctx.trigger_pushed = true;
}

/// Finalize the patch set.
/// If at least one trigger was pushed, a `clip_id` entry is inserted.
fn finish(ctx: &PatchContext<'_, '_>) {
    if !ctx.trigger_pushed {
        return;
    }
    ctx.one.insert(
        json_path![ctx.new_clip_gen_id, "hkbClipGenerator", "triggers"],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Replace),
                value: json_typed!(borrowed, ctx.clip_id), // hkbClipTriggerArray Pointer
            },
            priority: ctx.priority,
        },
    );
}
