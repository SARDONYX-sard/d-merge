//! Running external commands (`git`, `cargo`). Arguments are passed without a shell,
//! so e.g. commit messages need no quoting.

use std::process::{Command, Stdio};

use crate::{Result, workspace_root};

/// Runs `program args..` in the repository root, with its output shown.
pub(crate) fn run(program: &str, args: &[&str]) -> Result {
    println!("> {program} {}", args.join(" "));
    let status = Command::new(program).args(args).current_dir(workspace_root()).status()?;
    if !status.success() {
        return Err(format!("`{program} {}` failed ({status})", args.join(" ")).into());
    }
    Ok(())
}

/// Runs `git args..` and returns its stdout, trimmed. e.g., `git(&["rev-parse", "HEAD"])`
pub(crate) fn git(args: &[&str]) -> Result<String> {
    println!("> git {}", args.join(" "));
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace_root())
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(format!("`git {}` failed ({})", args.join(" "), output.status).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}
