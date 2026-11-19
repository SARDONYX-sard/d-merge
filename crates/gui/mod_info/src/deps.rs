//! `deps` of `info.ini` (`Nemesis_EngineExt` only): the mods a mod depends on, each with the
//! versions of it that it works with, if it says.
//!
//! ```text
//! deps=amco>=1.2.0, bfco
//!      ^^^^ ^^^^^^^   ^^^^ any version
//!      code the versions of `amco` it works with
//! ```
//!
//! A constraint is one comparator of semver (Cargo's syntax): `>=1.2.0`, `<2`, `=1.2.3`, `^1.2`,
//! `~1.2`, `1.*`. A version is read leniently: `1.2` is `1.2.0`, `v1.2.0` is `1.2.0`.
//!
//! Nothing here stops a patch: what does not meet a constraint is a problem the editor tells.

use std::str::FromStr as _;

/// The chars a constraint starts with, after the code. e.g. `>` of `amco>=1.2.0`
const REQ_STARTS: [char; 5] = ['<', '>', '=', '^', '~'];

/// One entry of `deps`. Borrows from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dep<'a> {
    /// The mod code. e.g. `amco`
    pub code: &'a str,
    /// The versions of it that work, `None` for any. e.g. `>=1.2.0`
    pub req: Option<&'a str>,
}

impl<'a> Dep<'a> {
    /// e.g. `amco>=1.2.0` -> `amco`, `>=1.2.0`; `amco ^1.2` -> `amco`, `^1.2`; `bfco` -> `bfco`, `None`
    #[must_use]
    pub fn parse(entry: &'a str) -> Self {
        let entry = entry.trim();
        let at = entry.find(|c: char| c.is_whitespace() || REQ_STARTS.contains(&c)).unwrap_or(entry.len());
        let (code, req) = entry.split_at(at);
        let req = req.trim();
        Self { code, req: (!req.is_empty()).then_some(req) }
    }
}

/// The entries of the value of `deps`. e.g. `amco>=1.2.0, bfco` -> `amco>=1.2.0`, `bfco`
pub fn entries(deps: &str) -> impl Iterator<Item = Dep<'_>> {
    deps.split(',').map(str::trim).filter(|entry| !entry.is_empty()).map(Dep::parse)
}

/// What a dependency's version is to a constraint on it. See [`check`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionCheck {
    /// It is one of the versions that work.
    Met,
    /// It is not. e.g. `1.1.0` to `>=1.2.0`
    Unmet,
    /// The mod says no version: it can't be known.
    NoVersion,
    /// The mod's version is no version. e.g. `beta`
    BadVersion,
    /// The constraint is none. e.g. `>=x`
    BadReq,
}

/// Does `version` (what the dependency's `info.ini` says) meet `req`?
///
/// e.g. `>=1.2.0`, `1.2` -> `Met`; `>=1.2.0`, `1.1.9` -> `Unmet`; `>=1.2.0`, `` -> `NoVersion`
#[must_use]
pub fn check(req: &str, version: &str) -> VersionCheck {
    let Ok(req) = semver::Comparator::from_str(req.trim()) else { return VersionCheck::BadReq };
    if version.trim().is_empty() {
        return VersionCheck::NoVersion;
    }
    match lenient_version(version) {
        Some(version) if req.matches(&version) => VersionCheck::Met,
        Some(_) => VersionCheck::Unmet,
        None => VersionCheck::BadVersion,
    }
}

/// Is `req` a constraint? e.g. `>=1.2` -> `true`, `>=x` -> `false`
#[must_use]
pub fn is_req(req: &str) -> bool {
    semver::Comparator::from_str(req.trim()).is_ok()
}

/// A mod's version, the parts it leaves out being `0`. `None`: it is no version.
///
/// e.g. `1.2` -> `1.2.0`, `v1` -> `1.0.0`, `1.2.0-beta` -> `1.2.0-beta`, `beta` -> `None`
#[must_use]
pub fn lenient_version(text: &str) -> Option<semver::Version> {
    let text = text.trim().trim_start_matches(['v', 'V']);
    if let Ok(version) = semver::Version::parse(text) {
        return Some(version);
    }

    // `1.2-beta` -> `1.2` + `-beta`
    let (core, rest) = text.split_at(text.find(['-', '+']).unwrap_or(text.len()));
    let parts = core.split('.').count();
    if parts >= 3 {
        return None;
    }
    let mut padded = String::with_capacity(text.len() + 4);
    padded.push_str(core);
    for _ in parts..3 {
        padded.push_str(".0");
    }
    padded.push_str(rest);
    semver::Version::parse(&padded).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_is_a_code_and_maybe_a_constraint() {
        assert_eq!(Dep::parse("amco>=1.2.0"), Dep { code: "amco", req: Some(">=1.2.0") });
        assert_eq!(Dep::parse(" amco ^1.2 "), Dep { code: "amco", req: Some("^1.2") });
        assert_eq!(Dep::parse("bfco"), Dep { code: "bfco", req: None });

        let all: Vec<Dep<'_>> = entries("amco>=1.2.0, bfco,").collect();
        assert_eq!(all, [Dep { code: "amco", req: Some(">=1.2.0") }, Dep { code: "bfco", req: None }]);
    }

    #[test]
    fn a_version_leaves_out_zeros() {
        assert_eq!(lenient_version("1.2"), Some(semver::Version::new(1, 2, 0)));
        assert_eq!(lenient_version("v1"), Some(semver::Version::new(1, 0, 0)));
        assert_eq!(lenient_version("1.2-beta").map(|v| v.to_string()).as_deref(), Some("1.2.0-beta"));
        assert_eq!(lenient_version("beta"), None);
        assert_eq!(lenient_version("1.2.3.4"), None);
    }

    #[test]
    fn a_version_meets_a_constraint_or_not() {
        assert_eq!(check(">=1.2.0", "1.2"), VersionCheck::Met);
        assert_eq!(check(">=1.2.0", "1.1.9"), VersionCheck::Unmet);
        assert_eq!(check("^1.2", "2.0.0"), VersionCheck::Unmet);
        assert_eq!(check(">=1.2.0", " "), VersionCheck::NoVersion);
        assert_eq!(check(">=1.2.0", "beta"), VersionCheck::BadVersion);
        assert_eq!(check(">=x", "1.2.0"), VersionCheck::BadReq);
        assert!(is_req("<2") && !is_req(">=x"));
    }
}
