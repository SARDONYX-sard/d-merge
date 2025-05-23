use super::class_table::FieldInfo;
use json_patch::Op;
use simd_json::BorrowedValue;
use std::{borrow::Cow, mem, ops::Range};

type Patches<'xml> = Vec<CurrentJsonPatch<'xml>>;

#[derive(Debug, Clone, Default)]
pub struct CurrentState<'xml> {
    /// current parsing filed type info.
    pub field_info: Option<&'static FieldInfo>,
    /// When present, this signals the start of a differential change
    pub mode_code: Option<&'xml str>,

    /// Is passed `<!-- ORIGINAL --!>` xml comment?
    is_passed_original: bool,

    /// If the addition or deletion is `<! -- CLOSE --!>` since it is impossible to determine whether something
    /// is added or deleted until a comment comes in, this is a place to temporarily save them.
    pub patches: Patches<'xml>,

    /// Indicates the current json position during one patch file.
    ///
    /// e.g. `["#2521", "BSRagdollContactListenerModifier"]`
    pub path: Vec<Cow<'xml, str>>,

    /// Indicates the extent of differential change of elements inside the Array.
    ///
    /// # Note
    /// - Used only when changing Array.
    /// - If start and end of range are the same, index mode
    pub seq_range: Option<Range<usize>>,

    /// Array range patch
    ///
    /// # Note
    /// - Used only when changing Array.
    /// - If start and end of range are the same, index mode
    pub seq_values: Vec<BorrowedValue<'xml>>,
}

impl<'de> CurrentState<'de> {
    pub const fn new() -> Self {
        Self {
            field_info: None,
            mode_code: None,
            patches: vec![],
            is_passed_original: false,
            path: vec![],
            seq_range: None,
            seq_values: vec![],
        }
    }

    /// The following is an additional element, so push.
    /// - `<!-- MOD_CODE ~<id>~ --!>` after it is found.
    /// - `<!-- ORIGINAL --!> is not found yet.
    pub fn push_current_patch(&mut self, value: BorrowedValue<'de>) {
        if self.mode_code.is_some() && !self.is_passed_original {
            if self.seq_range.is_some() {
                self.seq_values.push(value);
            } else {
                let path = self.path.clone();
                self.patches.push(CurrentJsonPatch { path, value });
            }
        }
    }

    /// - `<!-- ORIGINAL --!> is found.
    #[inline]
    pub const fn set_is_passed_original(&mut self) {
        self.is_passed_original = true;
    }

    #[inline]
    pub fn judge_operation(&self) -> Op {
        self.mode_code.map_or(Op::Remove, |_| {
            if self.is_passed_original {
                if self.patches.is_empty() && self.seq_values.is_empty() {
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
    pub const fn clear_flags(&mut self) {
        self.mode_code = None;
        self.is_passed_original = false;
    }

    /// Judge Op + clear flags + take patches
    #[inline]
    pub fn take_patches(&mut self) -> (Op, Patches<'de>) {
        let op = self.judge_operation();
        self.clear_flags();
        (op, mem::take(&mut self.patches))
    }

    pub const fn increment_range(&mut self) {
        if let Some(ref mut range) = self.seq_range {
            range.end += 1;
        }
    }
}

/// The reason this is necessary is because
/// `<!-- ORIGINAL -->` or `<! -- CLOSE -->` is read, the operation type cannot be determined.
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentJsonPatch<'xml> {
    /// $(root), index, className, fieldName
    /// - e.g. "$.4514.hkbStateMachineStateInfo.generator",
    pub path: Vec<Cow<'xml, str>>,
    /// patch target json value
    pub value: BorrowedValue<'xml>,
}
