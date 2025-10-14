mod current_state;
pub mod deserializer;

use std::{borrow::Cow, ops::Range};

use json_patch::Op;

use crate::adsf::normal::AnimDataHeader;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AnimHeaderDiffPatch<'a> {
    lead_int: Option<Cow<'a, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "DiffProjectAssets<'a>: serde::Deserialize<'de>"))
    )]
    project_assets: Option<DiffProjectAssets<'a>>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct DiffProjectAssets<'a> {
    op: Op,
    range: Range<usize>,
    values: Vec<Cow<'a, str>>,
}

impl AnimHeaderDiffPatch<'_> {
    const DEFAULT: Self = Self {
        lead_int: None,
        project_assets: None,
    };

    pub fn merge(&mut self, other: Self) {
        if other.lead_int.is_some() {
            self.lead_int = other.lead_int;
        }
        if other.project_assets.is_some() {
            self.project_assets = other.project_assets;
        }
    }
}

impl<'a> AnimHeaderDiffPatch<'a> {
    pub fn into_apply(self, anim_data_header: &mut AnimDataHeader<'a>) {
        if let Some(lead_int) = self.lead_int {
            anim_data_header.lead_int = lead_int;
        }

        if let Some(project_assets) = self.project_assets {
            let op = project_assets.op;
            let range = project_assets.range.clone();
            match op {
                Op::Add => {
                    if range.start >= anim_data_header.project_assets.len() {
                        // Out-of-bounds → append at the end
                        anim_data_header
                            .project_assets
                            .extend(project_assets.values);
                    } else {
                        // In-bounds → insert at the middle
                        anim_data_header
                            .project_assets
                            .splice(range.start..range.start, project_assets.values);
                    }
                }
                Op::Replace => {
                    let vec_len = anim_data_header.project_assets.len();
                    let start = range.start.min(vec_len);
                    let end = range.end.min(vec_len);

                    let (replace_vals, append_vals) = {
                        let replace_count = end.saturating_sub(start);
                        let mut values = project_assets.values.into_iter();
                        let replace_vals: Vec<_> = values.by_ref().take(replace_count).collect();
                        let append_vals: Vec<_> = values.collect();
                        (replace_vals, append_vals)
                    };

                    // Replace within the valid range
                    anim_data_header
                        .project_assets
                        .splice(start..end, replace_vals);

                    // Append any remaining values (out-of-range)
                    if !append_vals.is_empty() {
                        anim_data_header.project_assets.extend(append_vals);
                    }
                }
                Op::Remove => {
                    let vec_len = anim_data_header.project_assets.len();
                    let start = range.start.min(vec_len);
                    let end = range.end.min(vec_len);
                    if start < end {
                        anim_data_header.project_assets.drain(start..end);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
enum LineKind {
    #[default]
    LeadInt,
    ProjectAssetsLen,
    ProjectAssets,
    HasMotionData,
}
