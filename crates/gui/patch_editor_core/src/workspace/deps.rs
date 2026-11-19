//! The mods a work depends on (`deps` of its `info.ini`), and whether the referenced ones are of the
//! versions it works with. See [`mod_info::deps`] for the syntax.
//!
//! ```text
//! References: [Flinching (flinch) ✖] [>=1.2.0] ⚠    <- `flinch` says `1.1.0`
//!                                     ^^^^^^^^ `Workspace::ref_req_mut`
//! export:     deps=flinch>=1.2.0                     <- `Workspace::deps`
//! ```
//!
//! What does not meet a constraint is only told, in the editor and the user patch: nothing stops.

use std::{collections::BTreeMap, path::Path};

use ::mod_info::deps::{VersionCheck, check};

use super::{
    Workspace,
    mod_info::{DepEntry, parse_info},
};

/// The version the referenced mod at `dir` (its patch directory) says, in its `info.ini`. Empty: it
/// says none (a mod of Nemesis, of Pandora), or it can't be read.
///
/// e.g. `D:/mods/a/Nemesis_EngineExt/mod/amco` -> `1.2.0`
pub(super) fn ref_version(dir: &Path) -> String {
    let ini = std::fs::read_to_string(dir.join("info.ini")).unwrap_or_default();
    parse_info(&ini).version
}

/// The mod code of a referenced mod, its patch directory's name. e.g. `D:/mods/a/Nemesis_Engine/mod/amco` -> `amco`
fn code_of_ref(dir: &str) -> Option<&str> {
    Path::new(dir).file_name()?.to_str()
}

impl Workspace {
    /// What `deps` says, each mod once by its code: what it said when it was read, and the mods it
    /// refers to, with the versions of them it works with (empty: any). `own_code`, this mod, is left
    /// out.
    ///
    /// e.g. read `bfco`, references `amco` (set to `>=1.2.0`) -> `[(amco, >=1.2.0), (bfco, "")]`
    pub(super) fn deps(&self, own_code: &str) -> Vec<(&str, &str)> {
        let mut deps: BTreeMap<&str, &str> = BTreeMap::new();
        for dep in &self.info.deps {
            deps.insert(&dep.code, dep.req.trim());
        }
        for code in self.ref_mods.iter().filter_map(|dir| code_of_ref(dir)) {
            deps.entry(code).or_default();
        }
        deps.remove(own_code);
        deps.into_iter().collect()
    }

    /// The versions of the referenced mod `at` of [`Self::ref_mods`] this one works with, to be
    /// edited. Empty: any.
    ///
    /// e.g. `>=1.2.0`
    pub fn ref_req_mut(&mut self, at: usize) -> Option<&mut String> {
        let code = code_of_ref(self.ref_mods.get(at)?)?;
        let known = self.info.deps.iter().position(|dep| dep.code.eq_ignore_ascii_case(code));
        let index = known.unwrap_or_else(|| {
            self.info.deps.push(DepEntry { code: code.to_owned(), req: String::new() });
            self.info.deps.len() - 1
        });
        Some(&mut self.info.deps[index].req)
    }

    /// Is the referenced mod `at` of [`Self::ref_mods`] of the versions this one works with? With
    /// what it says it is. `None`: any version works.
    ///
    /// e.g. `>=1.2.0` and `amco` says `1.1.0` -> `(Unmet, "1.1.0")`
    pub fn ref_check(&self, at: usize) -> Option<(VersionCheck, &str)> {
        let code = code_of_ref(self.ref_mods.get(at)?)?;
        let dep = self.info.deps.iter().find(|dep| dep.code.eq_ignore_ascii_case(code))?;
        if dep.req.trim().is_empty() {
            return None;
        }
        let version = self.ref_versions.get(at).map_or("", String::as_str);
        Some((check(&dep.req, version), version))
    }

    /// No longer depends on the referenced mod `at` of [`Self::ref_mods`]: it is not read, nor in
    /// `deps`.
    pub fn remove_ref(&mut self, at: usize) {
        if at >= self.ref_mods.len() {
            return;
        }
        let dir = self.ref_mods.remove(at);
        if let Some(code) = code_of_ref(&dir) {
            self.info.deps.retain(|dep| !dep.code.eq_ignore_ascii_case(code));
        }
        self.reload_refs();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work_with_refs(dirs: &[&str]) -> Workspace {
        Workspace {
            ref_mods: dirs.iter().map(|dir| (*dir).to_owned()).collect(),
            ref_versions: vec![String::new(); dirs.len()],
            ..Workspace::default()
        }
    }

    #[test]
    fn a_referenced_mod_is_checked_once_a_version_is_asked() {
        let mut workspace = work_with_refs(&["D:/mods/a/Nemesis_EngineExt/mod/amco"]);
        workspace.ref_versions[0] = "1.1".to_owned();
        assert_eq!(workspace.ref_check(0), None, "any version");

        *workspace.ref_req_mut(0).unwrap() = ">=1.2.0".to_owned();
        assert_eq!(workspace.ref_check(0), Some((VersionCheck::Unmet, "1.1")));
        workspace.ref_versions[0] = "1.2".to_owned();
        assert_eq!(workspace.ref_check(0), Some((VersionCheck::Met, "1.2")));
        workspace.ref_versions[0].clear();
        assert_eq!(workspace.ref_check(0), Some((VersionCheck::NoVersion, "")));
        assert_eq!(workspace.ref_check(1), None, "no such reference");
    }

    #[test]
    fn deps_has_each_mod_once_with_its_versions() {
        let mut workspace = work_with_refs(&["D:/mods/a/Nemesis_Engine/mod/amco", "D:/mods/m/Nemesis_Engine/mod/mine"]);
        workspace.info.deps = vec![DepEntry { code: "bfco".into(), req: String::new() }];
        *workspace.ref_req_mut(0).unwrap() = " >=1.2.0 ".to_owned();

        let expected = [("amco", ">=1.2.0"), ("bfco", "")];
        assert_eq!(workspace.deps("mine"), expected);

        workspace.remove_ref(0);
        assert_eq!(workspace.deps("mine"), expected[1..], "removed: no longer a dependency");
    }

    #[test]
    fn a_referenced_mod_says_its_version_in_its_info() {
        let dir = std::env::temp_dir().join(format!("d_merge_ref_version_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(ref_version(&dir), "", "no info.ini");
        std::fs::write(dir.join("info.ini"), "name=a\nversion=1.2.0\n").unwrap();
        assert_eq!(ref_version(&dir), "1.2.0");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
