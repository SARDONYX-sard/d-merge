// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Pandora Behaviour Engine Contributors
//
// This is based on the logic of Pandora-Behaviour-Engine-Plus.
//! This module defines FNIS animation types and flags.
//!
//! All animation type (`<AnimType>`) and option (`<option>`) definitions
//! are **based on and quoted from** _Fore's_ **"FNIS for Modders_V6.2.pdf"(© Fore)**,
//! which is part of the FNIS (Fores New Idles in Skyrim) modding documentation.
mod animations;
mod inject;
mod list_parser;

/// Represents a FNIS animation kind, consisting of a base animation type and optional flags.
///
/// Derived from FNIS animation definition syntax:
/// - `<AnimType>` describes the core behavior (e.g., basic, sequenced, furniture, paired).
/// - `<option>` describes modifiers (e.g., acyclic, animated camera, headtracking).
#[derive(Debug, PartialEq)]
pub struct FNISAnimKind {
    /// The main animation type (FNISAnimType).
    pub anim_type: FNISAnimType,
    /// Bitflags representing animation modifiers (FNISAnimFlags).
    pub flags: FNISAnimFlags,
}

impl FNISAnimKind {
    /// Creates a new FNISAnimKind with the given animation type and flags.
    #[inline]
    pub const fn new(animation_type: FNISAnimType, flags: FNISAnimFlags) -> Self {
        Self {
            anim_type: animation_type,
            flags,
        }
    }
}

/// Core FNIS animation types from `<AnimType>` syntax.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FNISAnimType {
    /// **b** – Basic: simple idle animation with one animation file.
    Basic,
    /// **s** – Sequenced Animation (SA): first of at least 2 animations played as a sequence.
    Sequenced,
    /// **so** – Sequenced Optimized: SA with AnimObjects and optimized Equip/UnEquip.
    SequencedOptimized,
    /// **fu** – Furniture Animation: first of at least 3 animations played on a furniture object.
    Furniture,
    /// **fuo** – Furniture Animation Optimized: fu with AnimObjects and optimized Equip/UnEquip.
    FurnitureOptimized,
    /// **+** – Second-to-last animation of a s/so/fu/fuo or ch definition.
    SequencedContinued,
    /// **ofa** – Offset Arm Animation: modifies arm position while other animations play.
    OffsetArm,
    /// **pa** – Paired Animation: contains animation data for two actors in one animation file.
    Paired,
    /// **km** – Killmove: paired animation used for the final blow in combat.
    KillMove,
    /// **aa** – Alternate Animation.
    Alternate,
    /// **ch** – Chair Animation.
    Chair,
}

impl FNISAnimType {
    #[inline]
    const fn as_str(&self) -> &'static str {
        match self {
            Self::Basic => "b",
            Self::Sequenced => "s",
            Self::SequencedOptimized => "so",
            Self::Furniture => "fu",
            Self::FurnitureOptimized => "fuo",
            Self::SequencedContinued => "+",
            Self::OffsetArm => "ofa",
            Self::Paired => "pa",
            Self::KillMove => "km",
            Self::Alternate => "aa",
            Self::Chair => "ch",
        }
    }
}

bitflags::bitflags! {
    /// FNIS animation modifier flags from `<option>` syntax.
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct FNISAnimFlags: u32 {
        /// No special options.
        const NONE = 0;
        /// **a** – Acyclic: plays only once (default is cyclic loop).
        const Acyclic = 1 << 0;
        /// **o** – Animation uses one or more AnimObjects.
        const AnimObjects = 1 << 1;
        /// **ac** – Animated Camera: allows camera control via `Camera3rd [Cam3]` bone.
        const AnimatedCamera = 1 << 2;
        /// **ac1** – Animated Camera Set: enable animated camera at animation start.
        const AnimatedCameraSet = 1 << 3;
        /// **ac0** – Animated Camera Reset: disable animated camera at animation end.
        const AnimatedCameraReset = 1 << 4;
        /// **bsa** – Animation file is part of a BSA archive (excluded from consistency check).
        const BSA = 1 << 5;
        /// **h** – Headtracking ON (default is OFF).
        const HeadTracking = 1 << 6;
        /// **k** – "Known" animation (vanilla or from another mod; excluded from consistency check).
        const Known = 1 << 7;
        /// **md** — Motion is driven by actor AI instead of animation data.
        ///
        /// - "motion driven" = motion from actor's package or player input.
        /// - "animation driven" = motion from animation's motion data.
        /// Most animations default to "animation driven" which disables AI movement.
        /// Use `-md` to keep AI movement active.
        const MotionDriven = 1 << 8;
        /// **st** —  sticky AO. Animation Object will not be unequipped at the end of animation.
        const Sticky = 1 << 9;
        /// **Tn** — Character keeps position after `-a` animation (no IdleForceDefaultState).
        const TransitionNext = 1 << 10;

        /// Special runtime-added flag: sequence start marker (not parsed from text).
        const SequenceStart = 1 << 11;
        /// Special runtime-added flag: sequence end marker (not parsed from text).
        const SequenceFinish = 1 << 12;
    }
}

impl FNISAnimFlags {
    /// Checks whether this animation has runtime modifiers like `HeadTracking` or `MotionDriven`.
    #[inline]
    pub const fn has_modifier(&self) -> bool {
        self.contains(Self::HeadTracking) || self.contains(Self::MotionDriven)
    }
}
