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

#[derive(Debug, Clone, Hash)]
pub struct BasicAnimation<'a> {
    pub(crate) template_type: FNISAnimType,
    pub(crate) flags: FNISAnimFlags,

    event_id: &'a str,
    animation_file_path: &'a str,

    anim_object_names: &'a [String],
    pub(crate) next_animation: Option<Box<FNISAnimation<'a>>>,
}

type Patches<'a> = (OnePatchMap<'a>, SeqPatchMap<'a>);

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
        patches: &Patches<'a>,
        priority: usize,
        state_info_id: &'a str,
        clip_id: &'a str,
    ) {
        let mut ctx = PatchContext::new(patches, priority, clip_id);

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
            ctx.one.insert(
                json_path![ctx.clip_id, "hkbClipGenerator", "mode"],
                ValueWithPriority {
                    patch: JsonPatch {
                        op: OpRangeKind::Pure(Op::Replace),
                        value: json_typed!(borrowed, mode),
                    },
                    priority,
                },
            );
        }

        if self.flags.contains(FNISAnimFlags::AnimObjects) {
            for name in self.anim_object_names {
                // hkbStringEventPayload
                let payload = json_typed!(borrowed, { "data": name });

                // enter events (id 393, 394)
                ctx.push_notify_event(
                    state_info_id,
                    NotifyEvent::Enter,
                    393,
                    Some(payload.clone()),
                );
                ctx.push_notify_event(state_info_id, NotifyEvent::Enter, 394, Some(payload));

                // Sticky exit to 165
                if !self.flags.contains(FNISAnimFlags::Sticky) {
                    ctx.push_notify_event(state_info_id, NotifyEvent::Exit, 165, None);
                }
            }
        }

        // SequenceStart
        if self.flags.contains(FNISAnimFlags::SequenceStart) {
            if let Some(FNISAnimation::Basic(next)) = self.next_animation.as_deref() {
                // use Nemesis variable.
                let start_event_var = Cow::Owned(format!("$eventID[{}]", next.event_id));
                ctx.push_event_name(&start_event_var);
                ctx.push_trigger(-0.3, &start_event_var);
            }
        }

        // SequenceFinish
        if self.flags.contains(FNISAnimFlags::SequenceFinish) {
            // use Nemesis variable.
            // During hkx conversion, if `hkbBehaviorGraphStringData.eventNames` contains identical strings, they are automatically replaced with the corresponding index.
            // Otherwise, an error occurs.
            {
                let done_event_var = Cow::Owned(format!("$eventID[{}_DONE]", self.event_id));
                ctx.push_event_name(&done_event_var);
                ctx.push_trigger(-0.2, &done_event_var);
            }

            let idle_event_var = Cow::Borrowed("$eventID[IdleForceDefaultState]$"); // `mt_behavior` has this eventName.
            ctx.push_trigger(-0.05, &idle_event_var);
        }

        ctx.finish();
    }
}

enum NotifyEvent {
    Enter,
    Exit,
}

/// PatchContext groups OnePatchMap and SeqPatchMap together
/// and manages insertion priority for patches.
///
/// - Provides helper methods for inserting notify events, event names,
///   and triggers.
/// - If `push_trigger` is called at least once, `finish()` must be called
///   to inject a `clip_index` entry into `hkbClipTriggerArray.triggers`.
struct PatchContext<'a: 'b, 'b> {
    one: &'b OnePatchMap<'a>,
    seq: &'b SeqPatchMap<'a>,
    priority: usize,
    clip_id: &'a str,
    trigger_pushed: bool,
}

impl<'a: 'b, 'b> PatchContext<'a, 'b> {
    /// Create a new PatchContext.
    const fn new(patches: &'b Patches<'a>, priority: usize, clip_id: &'a str) -> Self {
        let (one, seq) = patches;
        Self {
            one,
            seq,
            priority,
            clip_id,
            trigger_pushed: false,
        }
    }

    /// Push seq patch to `hkbStateMachineStateInfo.events`.
    ///
    /// - `kind`: "enterNotifyEvents" or "exitNotifyEvents"
    /// - `id`: event ID
    /// - `payload`: optional payload value
    pub fn push_notify_event(
        &self,
        state_info_id: &'a str,
        event_kind: NotifyEvent,
        id: impl serde::Serialize,
        payload: Option<simd_json::BorrowedValue<'a>>,
    ) {
        let kind = match event_kind {
            NotifyEvent::Enter => "enterNotifyEvents",
            NotifyEvent::Exit => "exitNotifyEvents",
        };

        self.seq.insert(
            json_path![state_info_id, "hkbStateMachineStateInfo", kind, "events"],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Seq(json_patch::OpRange {
                        op: Op::Add,
                        range: 9999..9999, // FIXME?: intended push to the last.
                    }),
                    // hkbEventProperty
                    value: json_typed!(borrowed, {
                        "parent": { // hkbEventBase
                            "id": id,
                            "payload": payload
                        }
                    }),
                },
                priority: self.priority,
            },
        );
    }

    /// Push seq patch to `hkbBehaviorGraphStringData.eventNames`.
    pub fn push_event_name(&self, event_name: &str) {
        let id = "#0000"; // TODO: resolve index dynamically

        self.seq.insert(
            json_path![id, "hkbBehaviorGraphStringData", "eventNames"],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Seq(json_patch::OpRange {
                        op: Op::Add,
                        range: 9999..9999, // FIXME?: intended push to the last.
                    }),
                    value: json_typed!(borrowed, event_name),
                },
                priority: self.priority,
            },
        );
    }

    /// Push seq patch to `hkbClipTriggerArray.triggers`.
    pub fn push_trigger(&mut self, local_time: f32, event_id: &str) {
        self.seq.insert(
            json_path![self.clip_id, "hkbClipTriggerArray", "triggers"],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Seq(json_patch::OpRange {
                        op: Op::Add,
                        range: 9999..9999, // FIXME?: intended push to the last.
                    }),
                    value: json_typed!(borrowed, {
                        "localTime": local_time,
                        "relativeToEndOfClip": true,
                        // hkbEventProperty
                        "event": {
                            "parent": { // hkbEventBase
                                "id": event_id // I32<'a>
                            }
                        },
                    }),
                },
                priority: self.priority,
            },
        );
        self.trigger_pushed = true;
    }

    /// Finalize the patch set.
    /// If at least one trigger was pushed, insert a `clip_id` to `triggers` entry.
    pub fn finish(self) {
        if !self.trigger_pushed {
            return;
        }

        // TODO: Find push target!
        self.one.insert(
            json_path!["#0000", "hkbClipGenerator", "triggers"],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Replace), // FIXME: If the name attribute is $mod_Id$, then generate hkbClipGenerator all.
                    value: json_typed!(borrowed, self.clip_id),
                },
                priority: self.priority,
            },
        );
    }
}

impl core::fmt::Display for BasicAnimation<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty = self.template_type.as_str();
        write!(f, "PN_{ty}_{}", self.event_id)
    }
}
