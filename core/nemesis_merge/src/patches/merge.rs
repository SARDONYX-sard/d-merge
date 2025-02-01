use crate::aliases::{MergedPatchMap, SortedPatchMap, TemplatePatchMap};
use crate::errors::Result;
use crate::paths::parse::get_nemesis_id;
use json_patch::{JsonPatch, JsonPath, Op, RangeKind};
use rayon::prelude::*;
use simd_json::{base::ValueTryAsArrayMut as _, derived::ValueTryIntoArray};
use std::borrow::Cow;
use std::collections::HashMap;
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

            let sorted_patches: Vec<SortedPatchMap<'_>> =
                ids.iter().filter_map(|id| patches.remove(id)).collect();

            let mut merged_result = HashMap::new();
            for patch_map in sorted_patches {
                merge_json_patches(&mut merged_result, patch_map);
            }

            let vec_result: Vec<_> = merged_result.into_iter().map(|(_, patch)| patch).collect();
            merged_patches
                .entry(template_name.clone())
                .or_default()
                .par_extend(vec_result);
        });
    });

    Ok(merged_patches)
}

fn merge_json_patches<'a>(base: &mut SortedPatchMap<'a>, additional: SortedPatchMap<'a>) {
    for (key, value) in additional {
        if let Some(base_mut) = base.get_mut(&key) {
            merge_inner(base_mut, key, value);
        } else {
            base.insert(key, value);
        }
    }
}

fn merge_inner<'a>(base: &mut JsonPatch<'a>, key: JsonPath<'a>, patch: JsonPatch<'a>) {
    // 1 separate range and non-range
    match patch.range {
        Some(range_kind) => match range_kind {
            RangeKind::One(range) => {}
            RangeKind::Multi(vec) => {}
        },
        None => {
            if base.op == patch.op {
                match base.op {
                    Op::Add => {
                        if let Ok(base_arr) = base.value.try_as_array_mut() {
                            if let Ok(additional_arr) = patch.value.try_into_array() {
                                base_arr.extend(additional_arr);
                            }
                        }
                    }
                    Op::Replace => {
                        base.value = patch.value;
                    }
                    Op::Remove => {}
                }
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

// Complete
// - range:
//    - add
//
// TODO
// - range:
//    - remove
//    - replace
// - one field
//    - replace
//    - add (maybe mod author's miss)
//    - remove
#[cfg(test)]
mod tests {
    use super::*;
    use json_patch::{json_path, JsonPatch, Op};

    #[test]
    fn test_merge_add_json_patches() {
        let mut base_hash_map = HashMap::new();

        base_hash_map.insert(
            json_path!["#0009", "hkbProjectStringData", "characterFilenames"],
            JsonPatch {
                op: Op::Add,
                path: vec![
                    Cow::Borrowed("#0029"),                  // class index
                    Cow::Borrowed("hkbCharacterStringData"), // class name
                    Cow::Borrowed("animationNames"),         // field name
                    Cow::Borrowed("[1656:1657]"),            // range(optional)
                ],
                value: vec![Cow::Borrowed("Animations\\1hm_UnsheatheAttack.hkx")].into(),
                range: Default::default(),
            },
        );

        let mut add_hash_map = HashMap::new();

        add_hash_map.insert(
            json_path!["#0009", "hkbProjectStringData", "characterFilenames"],
            JsonPatch {
                op: Op::Add,
                path: vec![
                    Cow::Borrowed("#0029"),
                    Cow::Borrowed("hkbCharacterStringData"),
                    Cow::Borrowed("animationNames"),
                    Cow::Borrowed("[1656:1657]"),
                ],
                value: vec![Cow::Borrowed("Animations\\MomoAJ\\mt_jumpfallback.hkx")].into(),
                range: Default::default(),
            },
        );

        merge_json_patches(&mut base_hash_map, add_hash_map);

        let mut expected = HashMap::new();
        let json_path = json_path![
            "#0029",
            "hkbCharacterStringData",
            "animationNames",
            "[1656:1657]"
        ];

        expected.insert(
            json_path,
            JsonPatch {
                op: Op::Add,
                path: json_path![
                    "#0029",
                    "hkbCharacterStringData",
                    "animationNames",
                    "[1656:1657]"
                ],
                value: vec![
                    Cow::Borrowed("Animations\\1hm_UnsheatheAttack.hkx"),
                    Cow::Borrowed("Animations\\MomoAJ\\mt_jumpfallback.hkx"),
                ]
                .into(),
                range: Default::default(),
            },
        );

        assert_eq!(base_hash_map, expected);
    }

    #[test]
    fn test_merge_replace_json_patches() {
        let mut base = HashMap::new();
        let json_path = json_path!["#0010", "hpathkbProjectData", "stringData"];

        base.insert(
            json_path.clone(),
            JsonPatch {
                op: Op::Replace,
                path: json_path,
                value: "$id".into(),
                range: Default::default(),
            },
        );

        let mut additional = HashMap::new();
        let json_path = json_path!["#0010", "hpathkbProjectData", "stringData"];
        additional.insert(
            json_path.clone(),
            JsonPatch {
                op: Op::Replace,
                path: json_path,
                value: "$id2".into(),
                range: Default::default(),
            },
        );

        merge_json_patches(&mut base, additional.clone());
        assert_eq!(base, additional);
    }
}
