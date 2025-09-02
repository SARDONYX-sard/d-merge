// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Pandora Behaviour Engine Contributors
//
// This is based on the logic of Pandora-Behaviour-Engine-Plus.

use crate::behaviors::tasks::fnis::{animations::FNISAnimation, FNISAnimFlags, FNISAnimType};

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
}

impl core::fmt::Display for BasicAnimation<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty = self.template_type.as_str();
        write!(f, "PN_{ty}_{}", self.event_id)
    }
}
