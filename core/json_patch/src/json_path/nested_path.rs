//! `["#0004", "hkbString"]`
//!
//! index      class             field           index
//! `["#0001", "hkbStringData", "eventTriggers"], value: vec![ValueWithPriority]` // add/remove index 5
//! `["#0001", "hkbStringData", "eventTriggers", "[5]", "local_time"]` // modify f32
//! `["#0001", "hkbStringData", "eventTriggers", "[5]", "triggers", [0], "animations", [3], "time"]` // modify f32
//! index < vec.len() && path.contain("[<integer>]")
//!
//! non_nested_array
//! seq patch
//! seq patch & nested_array one patch
//!
//! enum PatchType {
//!     NestedArray(Vec<JsonPath>),
//!     Other(JsonPath),
//! }
//! vec![PatchType]
use std::borrow::Cow;

use crate::JsonPath;

#[derive(Debug, Clone, PartialEq)]
pub enum PathType<'a> {
    /// Represents a path to a JSON value, e.g., `"#0001.hkbString.event_names"`
    Simple(JsonPath<'a>),
    /// Represents a nested path with an array index, e.g., `"#0001.hkbStringData.eventTriggers[5].triggers[0].animations[3].time"`
    /// - (base array, children)
    Nested((JsonPath<'a>, Vec<(JsonPath<'a>, JsonPath<'a>)>)),
}

pub fn sort_nested_json_path<'a>(paths: Vec<JsonPath<'a>>) -> Vec<PathType<'a>> {
    use std::collections::HashMap;

    // 1. Group paths by their first three segments
    let mut nested_cache: HashMap<[Cow<'a, str>; 3], Vec<JsonPath<'a>>> = HashMap::new();

    for path in paths {
        if path.len() > 3 && path[3].starts_with('[') {
            // This is a nested path
            let key = [path[0].clone(), path[1].clone(), path[2].clone()];
            nested_cache.entry(key).or_default().push(path);
        } else {
            // It's a simple path
            // Store simple path as a separate key that can be added later
            let key = [
                path[0].clone(),
                path.get(1).cloned().unwrap_or(Cow::Borrowed("")),
                path.get(2).cloned().unwrap_or(Cow::Borrowed("")),
            ];
            nested_cache.entry(key).or_default().push(path);
        }
    }

    let mut results = Vec::new();

    for (key, group) in nested_cache {
        // Split into parent vs nested
        let parent_opt = group.iter().find(|p| p.len() <= 3).cloned();
        let nested_paths: Vec<_> = group
            .into_iter()
            .filter(|p| p.len() > 3)
            .map(|p| {
                let children = p.iter().skip(3).cloned().collect();
                (p, children)
            })
            .collect();

        if !nested_paths.is_empty() {
            results.push(PathType::Nested((
                parent_opt.unwrap_or_else(|| vec![key[0].clone(), key[1].clone(), key[2].clone()]),
                nested_paths,
            )));
        } else if let Some(parent) = parent_opt {
            results.push(PathType::Simple(parent));
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cow_path<'a>(v: &[&'a str]) -> JsonPath<'a> {
        v.iter().cloned().map(Cow::from).collect()
    }

    #[test]
    fn test_sort_nested_json_path_strict() {
        // key (long -> short order)
        let paths = vec![
            cow_path(&[
                "#0001",
                "hkbStringData",
                "eventTriggers",
                "[5]",
                "triggers",
                "[0]",
                "animations",
                "[3]",
                "time",
            ]),
            cow_path(&[
                "#0001",
                "hkbStringData",
                "eventTriggers",
                "[5]",
                "local_time",
            ]), // op priority, value
            cow_path(&["#0001", "hkbStringData", "eventTriggers"]), //vec![{ op, range, priority }]
            cow_path(&["#0002", "otherObject", "name"]),
            cow_path(&["#0003", "oneMore", "no_array"]),
        ];

        let sorted = sort_nested_json_path(paths);

        dbg!(&sorted);

        assert_eq!(sorted.len(), 3, "Expected three path groups");

        // Find nested
        let nested_paths = sorted
            .iter()
            .find_map(|p| {
                if let PathType::Nested(paths) = p {
                    Some(paths)
                } else {
                    None
                }
            })
            .expect("Expected a nested path group");

        assert_eq!(
            nested_paths.1.len(),
            2,
            "Nested group should have 3 entries"
        );
        assert_eq!(
            nested_paths.0,
            cow_path(&["#0001", "hkbStringData", "eventTriggers"])
        );

        assert_eq!(
            nested_paths.1[0].1,
            cow_path(&["[5]", "triggers", "[0]", "animations", "[3]", "time"])
        );
        assert_eq!(nested_paths.1[1].1, cow_path(&["[5]", "local_time"]));

        // Check simple path #0002
        let simple_2 = sorted
            .iter()
            .find(|p| match p {
                PathType::Simple(path) => path[0] == "#0002",
                PathType::Nested(_) => false,
            })
            .expect("Expected Simple path for #0002");
        match simple_2 {
            PathType::Simple(path) => {
                assert_eq!(path, &cow_path(&["#0002", "otherObject", "name"]));
            }
            PathType::Nested(_) => panic!("Expected Simple variant"),
        }

        // Check simple path #0003
        let simple_3 = sorted
            .iter()
            .find(|p| match p {
                PathType::Simple(path) => path[0] == "#0003",
                PathType::Nested(_) => false,
            })
            .expect("Expected Simple path for #0003");
        match simple_3 {
            PathType::Simple(path) => {
                assert_eq!(path, &cow_path(&["#0003", "oneMore", "no_array"]));
            }
            PathType::Nested(_) => panic!("Expected Simple variant"),
        }
    }
}
