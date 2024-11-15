use super::class_table::FieldInfo;
use json_patch::merger::Op;
use simd_json::BorrowedValue;
use std::{borrow::Cow, mem};

type Patches<'xml> = Vec<CurrentPatchJson<'xml>>;

#[derive(Debug, Clone, Default)]
pub struct CurrentState<'xml> {
    pub field_info: Option<&'static FieldInfo>,
    pub mode_code: Option<&'xml str>,
    is_passed_original: bool,
    pub patches: Patches<'xml>,

    /// Indicates the current json position during one patch file.
    pub path: Vec<Cow<'xml, str>>,
}

impl<'de> CurrentState<'de> {
    pub const fn new() -> Self {
        Self {
            field_info: None,
            mode_code: None,
            patches: vec![],
            is_passed_original: false,
            path: vec![],
        }
    }

    /// The following is an additional element, so push.
    /// - `<!-- MOD_CODE ~<id>~ --!>` after it is found.
    /// - `<!-- ORIGINAL --!> is not found yet.
    #[inline]
    pub fn push_current_patch(&mut self, value: BorrowedValue<'de>) {
        if self.mode_code.is_some() && !self.is_passed_original {
            self.patches.push(CurrentPatchJson {
                path: self.path.clone(),
                value,
            });
        }
    }

    #[inline]
    pub fn set_is_passed_original(&mut self) {
        self.is_passed_original = true;
    }

    #[inline]
    fn judge_operation(&mut self) -> Op {
        let op = if self.mode_code.is_some() {
            match self.is_passed_original {
                true => Op::Replace,
                false => {
                    if self.patches.is_empty() {
                        Op::Remove
                    } else {
                        Op::Add
                    }
                }
            }
        } else {
            Op::Remove
        };

        self.mode_code = None;
        self.is_passed_original = false;
        op
    }

    #[inline]
    pub fn take_patches(&mut self) -> (Op, Patches<'de>) {
        (self.judge_operation(), mem::take(&mut self.patches))
    }
}

/// The reason this is necessary is because
/// `<!-- ORIGINAL -->` or `<! -- CLOSE -->` is read, the operation type cannot be determined.
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentPatchJson<'xml> {
    /// $(root), index, className, fieldName
    /// - e.g. "$.4514.hkbStateMachineStateInfo.generator",
    pub path: Vec<Cow<'xml, str>>,
    /// patch target json value
    pub value: BorrowedValue<'xml>,
}
