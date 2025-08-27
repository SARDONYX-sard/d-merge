mod current_state;
pub mod deserializer;
pub mod error;

use self::error::Error;
use json_patch::apply_seq_array_directly;
use json_patch::ValueWithPriority;
use rayon::prelude::*;
use simd_json::borrowed::Value;
use simd_json::serde::from_borrowed_value;
use std::borrow::Cow;

/// Reusable code for analyzing line difference patches in projects txt and anim header files
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct DiffLines<'a>(
    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "Vec<ValueWithPriority<'a>>: serde::Deserialize<'de>")
        )
    )]
    pub Vec<ValueWithPriority<'a>>,
);

impl<'a> DiffLines<'a> {
    pub const DEFAULT: Self = Self(vec![]);

    /// Returns `true` if the vector contains no elements.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn merge(&mut self, other: Self) {
        if !other.0.is_empty() {
            self.0.par_extend(other.0);
        }
    }

    /// Apply the patches to the given lines.
    ///
    /// # Errors
    /// If the patches cannot be applied due to a mismatch in types or other issues.
    pub fn into_apply(mut self, target_lines: &mut Vec<Cow<'a, str>>) -> Result<(), Error> {
        // take & change condition to json -> marge
        if !self.0.is_empty() {
            let patches = core::mem::take(&mut self.0);

            let lines = core::mem::take(target_lines);
            let mut template: Vec<Value> = lines.into_par_iter().map(Into::into).collect();
            apply_seq_array_directly(&mut template, patches)?;
            *target_lines = from_borrowed_value(template.into())?;
        }

        Ok(())
    }
}
