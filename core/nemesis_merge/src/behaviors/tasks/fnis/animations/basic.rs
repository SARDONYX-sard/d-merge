// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Pandora Behaviour Engine Contributors
//
// This is based on the logic of Pandora-Behaviour-Engine-Plus.

use std::borrow::Cow;

use json_patch::{json_path, JsonPatch, Op, OpRangeKind, ValueWithPriority};
use simd_json::json_typed;

use crate::behaviors::tasks::{
    fnis::{animations::FNISAnimation, FNISAnimFlags, FNISAnimType},
    patches::types::{OnePatchMap, SeqPatchMap},
};

#[derive(Debug, Clone)]
pub struct BasicAnimation<'a> {
    pub(crate) template_type: FNISAnimType,
    pub(crate) flags: FNISAnimFlags,

    event_id: &'a str,
    animation_file_path: &'a str,

    anim_object_names: &'a [String],
    pub(crate) next_animation: Option<Box<FNISAnimation<'a>>>,
}

impl<'a> BasicAnimation<'a> {
    pub const fn new(
        template_type: FNISAnimType,
        flags: FNISAnimFlags,
        event: &'a str,
        animation_file_path: &'a str,
        anim_object_names: &'a [String],
    ) -> Self {
        Self {
            template_type,
            flags,
            event_id: event,
            animation_file_path,
            anim_object_names,
            next_animation: None,
        }
    }

    // - `state_info_id`: hkbStateMachineStateInfo root class name att r
    fn build_flags(
        &self,
        patches: &mut (OnePatchMap<'a>, SeqPatchMap<'a>),
        priority: usize,
        clip_id: &'a str,
        state_info_id: &'a str,
    ) {
        let (one, seq) = patches;

        if self.flags.has_modifier() {
            // TODO:
        }

        // `mode` to Acyclic or Looping
        {
            let mode = if self.flags.contains(FNISAnimFlags::Acyclic) {
                "MODE_SINGLE_PLAY"
            } else {
                "MODE_LOOPING"
            };
            one.insert(
                json_path![clip_id, "hkbClipGenerator", "mode"],
                ValueWithPriority {
                    patch: JsonPatch {
                        op: OpRangeKind::Pure(Op::Replace),
                        value: json_typed!(borrowed, mode),
                    },
                    priority,
                },
            );
        }

        // AnimObjects の場合
        if self.flags.contains(FNISAnimFlags::AnimObjects) {
            for name in self.anim_object_names {
                let payload = json_typed!(borrowed, { "data": name });

                // enter events (id 393, 394)
                push_notify_event(
                    seq,
                    state_info_id,
                    NotifyEvent::Enter,
                    393,
                    Some(payload.clone()),
                    priority,
                );
                push_notify_event(
                    seq,
                    state_info_id,
                    NotifyEvent::Enter,
                    394,
                    Some(payload),
                    priority,
                );

                // Sticky exit to 165
                if !self.flags.contains(FNISAnimFlags::Sticky) {
                    push_notify_event(seq, state_info_id, NotifyEvent::Exit, 165, None, priority);
                }
            }
        }

        // SequenceStart
        if self.flags.contains(FNISAnimFlags::SequenceStart) {
            if let Some(FNISAnimation::Basic(next)) = self.next_animation.as_deref() {
                // use Nemesis variable.
                let start_event_var = Cow::Owned(format!("$eventID[{}]", next.event_id));
                push_event_name(seq, &start_event_var, priority);
                push_trigger(seq, clip_id, -0.3, &start_event_var, priority);
            }
        }

        // SequenceFinish
        if self.flags.contains(FNISAnimFlags::SequenceFinish) {
            // use Nemesis variable.
            // During hkx conversion, if `hkbBehaviorGraphStringData.eventNames` contains identical strings, they are automatically replaced with the corresponding index.
            // Otherwise, an error occurs.
            {
                let done_event_var = Cow::Owned(format!("$eventID[{}_DONE]", self.event_id));
                push_event_name(seq, &done_event_var, priority);
                push_trigger(seq, clip_id, -0.2, &done_event_var, priority);
            }

            let idle_event_var = Cow::Borrowed("$eventID[IdleForceDefaultState]$"); // `mt_behavior` has this eventName.
            push_trigger(seq, clip_id, -0.05, &idle_event_var, priority);
        }
    }
}

enum NotifyEvent {
    Enter,
    Exit,
}

/// Push seq patch to `hkbStateMachineStateInfo.events`
///
/// - `event`: "enterNotifyEvents" or "exitNotifyEvents"
fn push_notify_event<'a>(
    seq: &SeqPatchMap<'a>,
    state_info_id: &'a str,
    kind: NotifyEvent,
    id: impl serde::Serialize,
    payload: Option<simd_json::BorrowedValue<'a>>,
    priority: usize,
) {
    let kind = match kind {
        NotifyEvent::Enter => "enterNotifyEvents",
        NotifyEvent::Exit => "exitNotifyEvents",
    };

    seq.insert(
        json_path![state_info_id, "hkbStateMachineStateInfo", kind, "events"],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Seq(json_patch::OpRange {
                    op: Op::Add,
                    range: 9999..9999, // FIXME?: intended push to the last.
                }),
                value: json_typed!(borrowed, {
                    "parent": {
                        "id": id,
                        "payload": payload
                    }
                }),
            },
            priority,
        },
    );
}

/// Push seq patch to `hkbBehaviorGraphStringData.eventNames`
fn push_event_name<'a>(seq: &SeqPatchMap<'a>, event_name: &str, priority: usize) {
    let id = "#0000"; // TODO: Find character mt_behavior.xml hkbBehaviorGraphStringData index

    seq.insert(
        json_path![id, "hkbBehaviorGraphStringData", "eventNames"],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Seq(json_patch::OpRange {
                    op: Op::Add,
                    range: 9999..9999, // FIXME?: intended push to the last.
                }),
                // hkbClipTrigger
                value: json_typed!(borrowed, event_name),
            },
            priority,
        },
    );
}

/// Push seq patch to `hkbClipGenerator.triggers`
fn push_trigger<'a>(
    seq: &SeqPatchMap<'a>,
    clip_id: &'a str,
    local_time: f32,
    event_id: &str,
    priority: usize,
) {
    seq.insert(
        json_path![clip_id, "hkbClipGenerator", "triggers"],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Seq(json_patch::OpRange {
                    op: Op::Add,
                    range: 9999..9999, // FIXME?: intended push to the last.
                }),
                // hkbClipTrigger
                value: json_typed!(borrowed, {
                    "localTime": local_time,
                    "relativeToEndOfClip": true,
                    // hkbEventProperty
                    "event": {
                        // hkbEventBase
                        "parent": {
                            "id": event_id
                        }
                    },
                }),
            },
            priority,
        },
    );
}

impl core::fmt::Display for BasicAnimation<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty = self.template_type.as_str();
        write!(f, "PN_{ty}_{}", self.event_id)
    }
}
