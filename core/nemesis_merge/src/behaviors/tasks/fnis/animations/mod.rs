mod basic;
mod furniture;
mod offset_arm;

use self::basic::BasicAnimation;
use self::furniture::FurnitureAnimation;
use self::offset_arm::OffsetArmAnimation;
use crate::behaviors::tasks::fnis::{list_parser::combinator::flags::FNISAnimFlags, FNISAnimType};

#[derive(Debug, Clone, Hash)]
pub enum FNISAnimation<'a, 'b> {
    Basic(BasicAnimation<'a, 'b>),
    Furniture(FurnitureAnimation<'a, 'b>),
    OffsetArm(OffsetArmAnimation<'a, 'b>),
}

impl<'a, 'b> FNISAnimation<'a, 'b> {
    #[inline]
    const fn next_mut(&mut self) -> Option<&mut Box<Self>> {
        match self {
            Self::Basic(anim) => anim.next_animation.as_mut(),
            Self::Furniture(anim) => anim.next_animation.as_mut(),
            Self::OffsetArm(anim) => anim.next_animation.as_mut(),
        }
    }

    #[inline]
    const fn flags(&self) -> FNISAnimFlags {
        match self {
            Self::Basic(anim) => anim.flags,
            Self::Furniture(anim) => anim.flags,
            Self::OffsetArm(anim) => anim.flags,
        }
    }

    #[inline]
    const fn flags_mut(&mut self) -> &mut FNISAnimFlags {
        match self {
            Self::Basic(anim) => &mut anim.flags,
            Self::Furniture(anim) => &mut anim.flags,
            Self::OffsetArm(anim) => &mut anim.flags,
        }
    }

    #[inline]
    const fn template_type(&self) -> FNISAnimType {
        match self {
            Self::Basic(anim) => anim.template_type,
            Self::Furniture(anim) => anim.template_type,
            Self::OffsetArm(anim) => anim.template_type,
        }
    }
}

pub struct FNISFactory<'a, 'b>(Vec<FNISAnimation<'a, 'b>>);

impl<'a, 'b> FNISFactory<'a, 'b> {
    pub fn create(
        &mut self,
        template_type: FNISAnimType,
        mut flags: FNISAnimFlags,
        event: &'a str,
        anim_path: &'a str,
        anim_object_names: &'b [&'a str],
    ) -> FNISAnimation<'a, 'b> {
        match template_type {
            FNISAnimType::Furniture | FNISAnimType::FurnitureOptimized => FNISAnimation::Furniture(
                FurnitureAnimation::new(template_type, flags, event, anim_path, anim_object_names),
            ),
            FNISAnimType::SequencedContinued => {
                let Some(mut prev_anim) = self.0.pop() else {
                    return FNISAnimation::Basic(BasicAnimation::new(
                        template_type,
                        flags,
                        event,
                        anim_path,
                        anim_object_names,
                    ));
                };

                let prev_anim_flags = prev_anim.flags();
                let prev_has_acyclic = prev_anim_flags.contains(FNISAnimFlags::Acyclic);

                if prev_has_acyclic && !prev_anim_flags.contains(FNISAnimFlags::SequenceFinish) {
                    *prev_anim.flags_mut() |= FNISAnimFlags::SequenceStart;
                } else if prev_has_acyclic {
                    flags |= FNISAnimFlags::SequenceFinish;
                };

                let animation = self.create(
                    prev_anim.template_type(),
                    flags,
                    event,
                    anim_path,
                    anim_object_names,
                );

                if let Some(next_anim) = prev_anim.next_mut() {
                    *next_anim = Box::new(animation.clone()); // TODO: use Arc?
                }

                animation
            }
            FNISAnimType::OffsetArm => FNISAnimation::OffsetArm(OffsetArmAnimation::new(
                template_type,
                flags,
                event,
                anim_path,
                anim_object_names,
            )),

            FNISAnimType::Basic
            | FNISAnimType::Sequenced
            | FNISAnimType::SequencedOptimized
            | FNISAnimType::Paired
            | FNISAnimType::KillMove
            | FNISAnimType::Alternate
            | FNISAnimType::Chair => FNISAnimation::Basic(BasicAnimation::new(
                template_type,
                flags,
                event,
                anim_path,
                anim_object_names,
            )),
        }
    }
}
