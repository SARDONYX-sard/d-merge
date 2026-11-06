//! `cargo x tag`: tag HEAD as a dev build and push it (the release workflow builds every tag).

use crate::{Result, cmd::git, version};

pub(crate) const USAGE: &str = "[-m <message>] [--no-push]";
pub(crate) const HELP: &str = "\
Tags HEAD as `<version>-dev.<short sha>` (signed, annotated, replacing a tag of the same name)
and pushes only that tag to origin, which starts the release workflow.
  -m         the tag message (default: the subject of the last commit)
  --no-push  only create the tag";

pub(crate) fn run(args: &[String]) -> Result {
    let version = version::read()?;
    let sha = git(&["rev-parse", "--short=7", "HEAD"])?;
    let name = format!("{version}-dev.{sha}");

    let message = match args.iter().position(|a| a == "-m") {
        Some(i) => args.get(i + 1).cloned().ok_or("`-m` needs a message")?,
        None => git(&["log", "-1", "--format=%s"])?,
    };
    git(&["tag", &name, "-m", &message, "-s", "--annotate", "-f"])?;

    if args.iter().any(|a| a == "--no-push") {
        println!("tagged {name}");
        return Ok(());
    }
    // Forced like the tag itself, so that a retag replaces the remote one.
    git(&["push", "--force", "origin", &format!("refs/tags/{name}")])?;
    println!("pushed {name}");
    Ok(())
}
