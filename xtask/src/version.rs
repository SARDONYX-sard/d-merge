//! The release version: `[workspace.package] version` in the root `Cargo.toml`.
//!
//! Parsed by hand (no regex): `<major>.<minor>.<patch>[-<tag>.<n>]`, e.g. `2.8.0`, `3.0.0-beta.1`.

use std::{fmt, path::PathBuf};

use crate::{Result, workspace_root};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Version {
    pub(crate) major: u64,
    pub(crate) minor: u64,
    pub(crate) patch: u64,
    /// e.g., `("beta", 1)` for `-beta.1`
    pub(crate) pre: Option<(String, u64)>,
}

impl Version {
    pub(crate) fn parse(s: &str) -> Result<Self> {
        let err = || format!("not a `x.y.z[-tag.n]` version: `{s}`");
        let (core, pre) = match s.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (s, None),
        };

        let mut parts = core.split('.');
        let mut number = || -> Result<u64> { Ok(parts.next().ok_or_else(err)?.parse()?) };
        let (major, minor, patch) = (number()?, number()?, number()?);
        if parts.next().is_some() {
            return Err(err().into());
        }

        let pre = match pre {
            Some(pre) => {
                let (tag, n) = pre.split_once('.').ok_or_else(err)?;
                Some((tag.to_string(), n.parse()?))
            }
            None => None,
        };
        Ok(Self { major, minor, patch, pre })
    }

    /// `level`: `major`, `minor`, `patch` (or `1`, `2`, `3`), `pre` (next prerelease number)
    /// or `pre=<tag>` (start a prerelease, e.g. `pre=beta` -> `x.y.z-beta.0`).
    pub(crate) fn bump(&self, level: &str) -> Result<Self> {
        let Self { major, minor, patch, .. } = *self;
        let next = |major, minor, patch| Self { major, minor, patch, pre: None };
        Ok(match level {
            "major" | "1" => next(major + 1, 0, 0),
            "minor" | "2" => next(major, minor + 1, 0),
            "patch" | "3" => next(major, minor, patch + 1),
            "pre" => {
                let (tag, n) =
                    self.pre.clone().ok_or_else(|| format!("{self} is not a prerelease"))?;
                Self { pre: Some((tag, n + 1)), ..self.clone() }
            }
            _ => match level.strip_prefix("pre=") {
                Some(tag) if !tag.is_empty() => {
                    Self { pre: Some((tag.to_string(), 0)), ..self.clone() }
                }
                _ => return Err(format!("unknown level `{level}`").into()),
            },
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some((tag, n)) = &self.pre {
            write!(f, "-{tag}.{n}")?;
        }
        Ok(())
    }
}

/// The root `Cargo.toml`.
pub(crate) fn manifest_path() -> PathBuf {
    workspace_root().join("Cargo.toml")
}

/// Reads `[workspace.package] version`.
pub(crate) fn read() -> Result<Version> {
    let manifest = std::fs::read_to_string(manifest_path())?;
    let line = version_line(&manifest).ok_or("no `[workspace.package] version` in Cargo.toml")?;
    Version::parse(value_of(line))
}

/// Rewrites `[workspace.package] version`, keeping the rest of the file (and its line endings) as is.
pub(crate) fn write(version: &Version) -> Result {
    let manifest = std::fs::read_to_string(manifest_path())?;
    let line = version_line(&manifest).ok_or("no `[workspace.package] version` in Cargo.toml")?;
    let old = value_of(line);
    let new_line = line.replacen(&format!("\"{old}\""), &format!("\"{version}\""), 1);
    let updated = manifest.replacen(line, &new_line, 1);
    std::fs::write(manifest_path(), updated)?;
    Ok(())
}

/// The `version = "..."` line inside `[workspace.package]`.
fn version_line(manifest: &str) -> Option<&str> {
    let mut in_section = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed == "[workspace.package]";
        } else if in_section
            && let Some((key, _)) = trimmed.split_once('=')
            && key.trim() == "version"
        {
            return Some(line);
        }
    }
    None
}

/// `version = "2.8.0"` -> `2.8.0`
fn value_of(line: &str) -> &str {
    let value = line.split_once('=').map_or("", |(_, value)| value.trim());
    value.trim_matches('"')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bumps() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.bump("minor").unwrap().to_string(), "1.3.0");
        assert_eq!(v.bump("pre=beta").unwrap().to_string(), "1.2.3-beta.0");

        let pre = Version::parse("1.2.3-alpha.7").unwrap();
        assert_eq!(pre.bump("pre").unwrap().to_string(), "1.2.3-alpha.8");
        assert_eq!(pre.bump("patch").unwrap().to_string(), "1.2.4");

        assert!(Version::parse("1.2").is_err());
        assert!(v.bump("pre").is_err());
    }

    #[test]
    fn finds_workspace_version_only() {
        let toml = "[package]\nversion = \"0.1.0\"\n[workspace.package]\nversion = \"2.8.0\"\n";
        assert_eq!(value_of(version_line(toml).unwrap()), "2.8.0");
    }
}
