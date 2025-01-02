use crate::aliases::{MergedPatchMap, TemplatePatchMap};
use crate::errors::Result;
use crate::paths::parse::get_nemesis_id;
use json_patch::JsonPatch;
use rayon::prelude::*;
use simd_json::{base::ValueTryAsArrayMut as _, derived::ValueTryIntoArray};
use std::borrow::Cow;
use std::path::PathBuf;

pub fn paths_to_ids(paths: &[PathBuf]) -> Vec<String> {
    paths
        .par_iter()
        .filter_map(|path| get_nemesis_id(path).ok())
        .collect()
}

pub fn merge_patches<'p>(
    patches: TemplatePatchMap<'p>,
    ids: &[String],
) -> Result<MergedPatchMap<'p>> {
    let merged_patches = MergedPatchMap::new();

    patches.into_par_iter().for_each(|idx_map| {
        let (template_name, patch_idx_map) = idx_map;

        patch_idx_map.into_par_iter().for_each(|patch| {
            let (_index, mut patches) = patch;

            let sorted_patches: Vec<Vec<JsonPatch<'p>>> = ids
                .iter()
                .filter_map(|id| patches.remove(id))
                .collect::<Vec<_>>();

            let mut merged_result = Vec::new();
            for patch_vec in sorted_patches {
                merge_json_patches(&mut merged_result, patch_vec);
            }

            merged_patches
                .entry(template_name.clone())
                .or_default()
                .par_extend(merged_result);
        });
    });

    Ok(merged_patches)
}

fn merge_json_patches<'a>(base: &mut Vec<JsonPatch<'a>>, additional: Vec<JsonPatch<'a>>) {
    for patch in additional {
        if let Some(range_index) = find_range_in_path(&patch.path) {
            let mut merged = false;

            for existing_patch in base.iter_mut() {
                if is_same_base_path(&existing_patch.path, &patch.path, range_index) {
                    let existing_range = parse_range(&existing_patch.path[range_index]);
                    let new_range = parse_range(&patch.path[range_index]);

                    if let (Some((e_start, e_end)), Some((n_start, n_end))) =
                        (existing_range, new_range)
                    {
                        if ranges_overlap_or_adjacent(e_start, e_end, n_start, n_end) {
                            // Merge ranges and values
                            let merged_range = merge_ranges(e_start, e_end, n_start, n_end);
                            existing_patch.path[range_index] =
                                Cow::Owned(format!("[{}:{}]", merged_range.0, merged_range.1));
                            if let Ok(base_arr) = existing_patch.value.try_as_array_mut() {
                                let _ = patch.value.clone().try_into_array().ok().map(|arr| {
                                    base_arr.extend(arr);
                                });
                            }
                            merged = true;
                            break;
                        }
                    }
                }
            }

            if !merged {
                base.push(patch);
            }
        } else {
            // No range found; simple append if not a duplicate
            if !base.contains(&patch) {
                base.push(patch);
            }
        }
    }
}

/// Find the index of a range (e.g., "[1656:1657]") in the path.
fn find_range_in_path(path: &[Cow<str>]) -> Option<usize> {
    path.iter()
        .position(|segment| segment.starts_with('[') && segment.ends_with(']'))
}

/// Parse a range string (e.g., "[1656:1657]") into a tuple (start, end).
fn parse_range(range: &str) -> Option<(usize, usize)> {
    if range.starts_with('[') && range.ends_with(']') {
        let parts: Vec<_> = range[1..range.len() - 1].split(':').collect();
        if parts.len() == 2 {
            if let (Ok(start), Ok(end)) = (parts[0].parse(), parts[1].parse()) {
                return Some((start, end));
            }
        }
    }
    None
}

/// Check if two paths have the same base, excluding the range segment.
fn is_same_base_path(base: &[Cow<str>], new: &[Cow<str>], range_index: usize) -> bool {
    base.len() == new.len()
        && base
            .iter()
            .enumerate()
            .all(|(i, segment)| i == range_index || segment == &new[i])
}

/// Check if two ranges overlap or are adjacent.
const fn ranges_overlap_or_adjacent(
    e_start: usize,
    e_end: usize,
    n_start: usize,
    n_end: usize,
) -> bool {
    !(n_end < e_start || e_end < n_start)
}

/// Merge two ranges into one.
fn merge_ranges(e_start: usize, e_end: usize, n_start: usize, n_end: usize) -> (usize, usize) {
    (e_start.min(n_start), e_end.max(n_end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use json_patch::{json_path, Op};

    #[test]
    fn test_merge_json_patches() {
        let mut base = vec![JsonPatch {
            op: Op::Add,
            path: vec![
                Cow::Borrowed("#0029"),
                Cow::Borrowed("hkbCharacterStringData"),
                Cow::Borrowed("animationNames"),
                Cow::Borrowed("[1656:1657]"),
            ],
            value: vec![Cow::Borrowed("Animations\\1hm_UnsheatheAttack.hkx")].into(),
        }];

        let additional = vec![JsonPatch {
            op: Op::Add,
            path: vec![
                Cow::Borrowed("#0029"),
                Cow::Borrowed("hkbCharacterStringData"),
                Cow::Borrowed("animationNames"),
                Cow::Borrowed("[1656:1657]"),
            ],
            value: vec![Cow::Borrowed("Animations\\MomoAJ\\mt_jumpfallback.hkx")].into(),
        }];

        merge_json_patches(&mut base, additional);

        assert_eq!(
            base,
            [JsonPatch {
                op: Op::Add,
                path: json_path![
                    "#0029",
                    "hkbCharacterStringData",
                    "animationNames",
                    "[1656:1657]"
                ],
                value: vec![
                    Cow::Borrowed("Animations\\1hm_UnsheatheAttack.hkx"),
                    Cow::Borrowed("Animations\\MomoAJ\\mt_jumpfallback.hkx")
                ]
                .into()
            }]
        );
    }
}
