//! `cargo x license-tree`: the license tree of `README.md`, made from `cargo metadata`, so that it
//! follows the crates as they are.
//!
//! ```text
//! d_merge (GPL-3.0-or-later)                          <- crates/gui
//! ├── behavior_merge (GPL-3.0-or-later)               <- crates/core
//! │   ├── behavior_merge_types (GPL-3.0-or-later)
//! │   ...
//! ├── page_patch (GPL-3.0-or-later)                    <- crates/gui
//! │   ├── behavior_merge (GPL-3.0-or-later) (*)       <- shown above: not again
//! ```
//! Only the workspace's crates, their normal dependencies (not dev or build ones). What most crates
//! use is left out: [`LEFT_OUT`]. The tree is written between [`START`] and [`END`] in `README.md`;
//! the rest of the file is not touched.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    process::Command,
};

use crate::{Result, workspace_root};

pub(crate) const USAGE: &str = "[--check]";
pub(crate) const HELP: &str = "\
Writes the license tree of README.md (between its `license-tree` markers) from `cargo metadata`.
  --check  only tell whether README.md is up to date (an error if not)";

/// The crate the tree starts from: the app.
const ROOT: &str = "d_merge";

/// Used by most crates: left out of the tree, as the README says.
const LEFT_OUT: [&str; 2] = ["fast_hash", "stdx"];

/// The lines around the tree in `README.md`.
const START: &str = "<!-- license-tree:start (cargo x license-tree) -->";
const END: &str = "<!-- license-tree:end -->";

/// The column of `<- crates/..` on a line, as in the README.
const NOTE_COLUMN: usize = 52;

/// One crate of the workspace, as the tree needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Crate {
    /// e.g. `GPL-3.0-or-later`, `MIT OR Apache-2.0`
    license: String,
    /// e.g. `crates/core`, `crates/gui`
    group: String,
    /// The workspace crates it depends on (normal dependencies), but [`LEFT_OUT`].
    deps: Vec<String>,
}

pub(crate) fn run(args: &[String]) -> Result {
    let check = match args {
        [] => false,
        [flag] if flag == "--check" => true,
        _ => return Err(format!("usage: cargo x license-tree {USAGE}").into()),
    };

    let tree = {
        let crates = workspace_crates()?;
        render(&crates, ROOT)?
    };

    let path = workspace_root().join("README.md");
    let readme = std::fs::read_to_string(&path)?;
    let updated = replace_between(&readme, &tree)?;

    if updated == readme {
        println!("README.md: the license tree is up to date");
        return Ok(());
    }
    if check {
        return Err("README.md: the license tree is old (run `cargo x license-tree`)".into());
    }
    std::fs::write(&path, updated)?;
    println!("README.md: the license tree is written");
    Ok(())
}

/// The crates of the workspace, by name, from `cargo metadata`.
fn workspace_crates() -> Result<BTreeMap<String, Crate>> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(workspace_root())
        .output()?;
    if !output.status.success() {
        return Err(format!("`cargo metadata` failed ({})", output.status).into());
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let packages = metadata["packages"].as_array().ok_or("`cargo metadata`: no `packages`")?;

    let names: BTreeSet<&str> = packages.iter().filter_map(|package| package["name"].as_str()).collect();
    let mut crates = BTreeMap::new();
    for package in packages {
        let Some(name) = package["name"].as_str() else { continue };
        crates.insert(name.to_owned(), crate_of(package, &names));
    }
    Ok(crates)
}

/// One package of `cargo metadata`. `workspace`: the names of the workspace's crates.
fn crate_of(package: &serde_json::Value, workspace: &BTreeSet<&str>) -> Crate {
    let license = package["license"].as_str().unwrap_or("(no license)").to_owned();

    // `<root>/crates/core/behavior_merge/Cargo.toml` -> `crates/core`
    let group = {
        let manifest = package["manifest_path"].as_str().unwrap_or_default().replace('\\', "/");
        let crates_at = manifest.find("/crates/").map(|at| at + 1);
        crates_at.map_or_else(String::new, |at| manifest[at..].split('/').take(2).collect::<Vec<_>>().join("/"))
    };

    // A normal dependency has no `kind` (`"dev"` and `"build"` are the others).
    let deps: BTreeSet<String> = package["dependencies"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|dep| dep["kind"].is_null())
        .filter_map(|dep| dep["name"].as_str())
        .filter(|name| workspace.contains(name) && !LEFT_OUT.contains(name))
        .map(str::to_owned)
        .collect();

    Crate { license, group, deps: deps.into_iter().collect() }
}

/// The tree from `root`, as the README shows it (in a `txt` block).
///
/// The root's dependencies are in the order of their group (`crates/core` before `crates/gui`), then
/// of their name; deeper ones by name. A crate shown with its dependencies once is shown again with
/// `(*)` only.
fn render(crates: &BTreeMap<String, Crate>, root: &str) -> Result<String> {
    let root_crate = crates.get(root).ok_or_else(|| format!("no crate `{root}` in the workspace"))?;

    let mut out = String::from("```txt\n");
    push_line(&mut out, &format!("{root} ({})", root_crate.license), Some(&root_crate.group));

    let mut children: Vec<&String> = root_crate.deps.iter().collect();
    children.sort_by_key(|name| (crates.get(name.as_str()).map(|dep| dep.group.as_str()), name.as_str()));

    let mut shown = BTreeSet::from([root.to_owned()]);
    for (at, name) in children.iter().enumerate() {
        let last = at + 1 == children.len();
        push_node(&mut out, (crates, &mut shown), name, ("", last), true);
    }

    out.push_str("```\n");
    Ok(out)
}

/// `name` and, unless shown before, its dependencies. `(prefix, last)`: the lines' start, and whether
/// it is the last of its parent's. `noted`: it gets its group, as the root's dependencies do.
fn push_node(
    out: &mut String,
    (crates, shown): (&BTreeMap<String, Crate>, &mut BTreeSet<String>),
    name: &str,
    (prefix, last): (&str, bool),
    noted: bool,
) {
    let branch = if last { "└── " } else { "├── " };
    let Some(krate) = crates.get(name) else { return };

    let again = !shown.insert(name.to_owned());
    let mark = if again && !krate.deps.is_empty() { " (*)" } else { "" };
    let text = format!("{prefix}{branch}{name} ({}){mark}", krate.license);
    push_line(out, &text, noted.then_some(krate.group.as_str()));
    if again {
        return;
    }

    let child_prefix = format!("{prefix}{}", if last { "    " } else { "│   " });
    for (at, dep) in krate.deps.iter().enumerate() {
        let last = at + 1 == krate.deps.len();
        push_node(out, (crates, shown), dep, (&child_prefix, last), false);
    }
}

/// A line of the tree, with `<- group` at [`NOTE_COLUMN`] if given.
fn push_line(out: &mut String, text: &str, group: Option<&str>) {
    match group {
        Some(group) => {
            let _ = writeln!(out, "{text:<NOTE_COLUMN$}<- {group}");
        }
        None => {
            let _ = writeln!(out, "{text}");
        }
    }
}

/// `readme` with `tree` between [`START`] and [`END`] in place of what was there.
fn replace_between(readme: &str, tree: &str) -> Result<String> {
    let missing = || format!("README.md: no `{START}` .. `{END}` around the license tree");
    let start = readme.find(START).ok_or_else(missing)? + START.len();
    let end = readme[start..].find(END).ok_or_else(missing)? + start;

    Ok(format!("{}\n\n{tree}\n{}", &readme[..start], &readme[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn krate(license: &str, group: &str, deps: &[&str]) -> Crate {
        Crate {
            license: license.to_owned(),
            group: group.to_owned(),
            deps: deps.iter().map(|dep| (*dep).to_owned()).collect(),
        }
    }

    /// The root's dependencies core first; a crate shown once is `(*)` the next time.
    #[test]
    fn the_tree_shows_each_crate_once() {
        let crates = BTreeMap::from([
            ("app".to_owned(), krate("GPL", "crates/gui", &["page", "engine"])),
            ("engine".to_owned(), krate("GPL", "crates/core", &["xml"])),
            ("page".to_owned(), krate("GPL", "crates/gui", &["engine", "widgets"])),
            ("widgets".to_owned(), krate("MIT", "crates/gui", &[])),
            ("xml".to_owned(), krate("MIT", "crates/core", &[])),
        ]);

        let tree = render(&crates, "app").unwrap();
        let lines: Vec<&str> = tree.lines().map(str::trim_end).collect();
        assert_eq!(
            lines,
            [
                "```txt",
                "app (GPL)                                           <- crates/gui",
                "├── engine (GPL)                                    <- crates/core",
                "│   └── xml (MIT)",
                "└── page (GPL)                                      <- crates/gui",
                "    ├── engine (GPL) (*)",
                "    └── widgets (MIT)",
                "```",
            ]
        );
    }

    #[test]
    fn only_what_is_between_the_markers_is_replaced() {
        let readme = format!("before\n{START}\n\nold\n\n{END}\nafter\n");
        let updated = replace_between(&readme, "new\n").unwrap();
        assert_eq!(updated, format!("before\n{START}\n\nnew\n\n{END}\nafter\n"));
        assert!(replace_between("no markers", "new").is_err());
    }
}
