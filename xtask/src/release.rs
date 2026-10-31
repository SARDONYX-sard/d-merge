//! `cargo x release <level>`: bump the version, commit and tag (nothing is pushed).

use crate::{
    Result,
    cmd::{git, run as run_cmd},
    version,
};

pub(crate) const USAGE: &str = "<major|minor|patch|pre|pre=<tag>|current> [--sign] [--dry-run]";
pub(crate) const HELP: &str = "\
Bumps `[workspace.package] version` in Cargo.toml, updates the workspace crates in Cargo.lock,
commits `build(version): bump from <old> to <new>` and tags `<new>` (message `v<new>`).
  major, minor, patch  e.g. 2.8.0 -> 3.0.0 / 2.9.0 / 2.8.1 (also 1, 2, 3)
  pre                  next prerelease, e.g. 3.0.0-beta.1 -> 3.0.0-beta.2
  pre=<tag>            start a prerelease, e.g. 2.8.0 -> 2.8.0-beta.0
  current              only tag HEAD with the current version
  --sign     sign the commit (git commit -S) and the tag (git tag -s)
  --dry-run  only print the new version
Push with: git push origin HEAD <new>  (a pushed tag starts the release workflow)";

/// Files the release commit may contain.
const FILES: [&str; 2] = ["Cargo.toml", "Cargo.lock"];

pub(crate) fn run(args: &[String]) -> Result {
    let level = args.first().ok_or_else(|| format!("usage: cargo x release {USAGE}"))?;
    let sign = args.iter().any(|a| a == "--sign");
    let dry_run = args.iter().any(|a| a == "--dry-run");
    // `git commit -S` signs a commit, `git tag -s` a tag (and makes it annotated).
    let tag_flag = if sign { "-s" } else { "--annotate" };

    let current = version::read()?;
    if level == "current" {
        let name = current.to_string();
        ensure_new_tag(&name)?;
        if !dry_run {
            git(&["tag", tag_flag, "-m", &format!("v{name}"), &name])?;
            println!("tagged {name}; push with: git push origin {name}");
        }
        return Ok(());
    }

    let next = current.bump(level)?;
    let name = next.to_string();
    ensure_new_tag(&name)?;
    println!("{current} -> {next}");
    if dry_run {
        return Ok(());
    }

    // Only the version files are committed, so they must not hold other edits.
    let mut status_args = vec!["status", "--porcelain", "--"];
    status_args.extend(FILES);
    let dirty = git(&status_args)?;
    if !dirty.is_empty() {
        return Err(format!("commit or stash the changes of these files first:\n{dirty}").into());
    }

    version::write(&next)?;
    // Only the workspace members: the versions of the dependencies stay.
    run_cmd("cargo", &["update", "--workspace"])?;

    let message = format!("build(version): bump from {current} to {next}");
    let mut add_args = vec!["add", "--"];
    add_args.extend(FILES);
    git(&add_args)?;
    let mut commit_args = vec!["commit", "-m", message.as_str()];
    if sign {
        commit_args.push("-S");
    }
    git(&commit_args)?;
    git(&["tag", tag_flag, "-m", &format!("v{name}"), &name])?;

    println!("committed and tagged {name}; push with: git push origin HEAD {name}");
    Ok(())
}

fn ensure_new_tag(name: &str) -> Result {
    if git(&["tag", "--list", name])?.is_empty() {
        Ok(())
    } else {
        Err(format!("the tag {name} exists already").into())
    }
}
