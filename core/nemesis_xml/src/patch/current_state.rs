use super::class_table::FieldInfo;
use json_patch::Op;
use simd_json::BorrowedValue;
use std::{borrow::Cow, mem, ops::Range};

type Patches<'xml> = Vec<CurrentPatchJson<'xml>>;

#[derive(Debug, Clone, Default)]
pub struct CurrentState<'xml> {
    pub field_info: Option<&'static FieldInfo>,
    /// When present, this signals the start of a differential change
    pub mode_code: Option<&'xml str>,
    is_passed_original: bool,
    pub patches: Patches<'xml>,

    /// Indicates the current json position during one patch file.
    pub path: Vec<Cow<'xml, str>>,

    /// Current index of the element inside the Array.
    /// # Note
    /// This also indicates non-differential ranges.
    pub seq_index: Option<usize>,
    /// Indicates the extent of differential change of elements inside the Array
    pub seq_range: Option<Range<usize>>,
}

impl<'de> CurrentState<'de> {
    pub const fn new() -> Self {
        Self {
            field_info: None,
            mode_code: None,
            patches: vec![],
            is_passed_original: false,
            path: vec![],
            seq_index: None,
            seq_range: None,
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

    /// - `<!-- ORIGINAL --!> is found.
    #[inline]
    pub fn set_is_passed_original(&mut self) {
        self.is_passed_original = true;
    }

    #[inline]
    pub fn judge_operation(&self) -> Op {
        self.mode_code.map_or(Op::Remove, |_| {
            if self.is_passed_original {
                if self.patches.is_empty() {
                    Op::Remove
                } else {
                    Op::Replace
                }
            } else {
                Op::Add
            }
        })
    }

    #[inline]
    fn clear_flags(&mut self) {
        self.mode_code = None;
        self.is_passed_original = false;
    }

    #[inline]
    pub fn take_patches(&mut self) -> (Op, Patches<'de>) {
        let op = self.judge_operation();
        self.clear_flags();
        (op, mem::take(&mut self.patches))
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
