//! The folders of Dynamic Animation Replacer (DAR) and Open Animation Replacer (OAR).
//!
//! Their hkx are not animations of their own, but stand in for another one. A DLL swaps them in when
//! a `hkbClipGenerator` plays that one, if its conditions hold.
//!
//! ```text
//! meshes\actors\character\animations\DynamicAnimationReplacer\_CustomConditions\100\male\mt_idle.hkx
//! meshes\actors\character\animations\DynamicAnimationReplacer\Skyrim.esm\00000007\mt_idle.hkx
//! meshes\actors\character\animations\OpenAnimationReplacer\MyMod\Submod\male\mt_idle.hkx
//!                                   ^ after it: what the replacer is ^ then the path they stand in for
//!  -> meshes\actors\character\animations\male\mt_idle.hkx (and mt_idle.hkx for the second)
//! ```
//!
//! | Replacer | After `animations\`                                                    |
//! | -------- | ---------------------------------------------------------------------- |
//! | DAR      | `DynamicAnimationReplacer\_CustomConditions\<priority>\`, or `DynamicAnimationReplacer\<esp name>\<actor base id>\` |
//! | OAR      | `OpenAnimationReplacer\<mod>\<submod>\`                                 |
//!
//! So they go into no anim set and no character's `animationNames`: the one they stand in for is
//! there already. Their CRC32 is that one's too. (ref: dar-to-oar, `dar2oar_core::fs::path_changer`)

/// A replacer of animations. See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Replacer {
    /// Dynamic Animation Replacer: `DynamicAnimationReplacer`
    Dar,
    /// Open Animation Replacer: `OpenAnimationReplacer`
    Oar,
}

impl Replacer {
    /// The folder of it, right under `animations`.
    pub const fn folder(self) -> &'static str {
        match self {
            Self::Dar => "DynamicAnimationReplacer",
            Self::Oar => "OpenAnimationReplacer",
        }
    }

    /// e.g. `DAR`
    pub const fn short_name(self) -> &'static str {
        match self {
            Self::Dar => "DAR",
            Self::Oar => "OAR",
        }
    }
}

/// The replacer `path` (from `meshes`, `\` or `/` separated, any case) is in, and the path it stands
/// in for, `\` separated, its case kept. `None`: it is no replacer's.
///
/// e.g. `meshes\actors\character\animations\OpenAnimationReplacer\A\B\male\idle.hkx` ->
/// `(Oar, meshes\actors\character\animations\male\idle.hkx)`; also a folder of it:
/// `...\OpenAnimationReplacer\A\B\male` -> `(Oar, ...\animations\male)`
pub fn stands_for(path: &str) -> Option<(Replacer, String)> {
    let parts: Vec<&str> = path.split(['\\', '/']).filter(|part| !part.is_empty()).collect();
    let animations = parts.iter().position(|part| part.eq_ignore_ascii_case("animations"))?;
    let folder = *parts.get(animations + 1)?;
    let replacer = [Replacer::Dar, Replacer::Oar].into_iter().find(|r| folder.eq_ignore_ascii_case(r.folder()))?;

    // `_CustomConditions\<priority>`, `<esp name>\<actor base id>`, `<mod>\<submod>`: two folders.
    let rest = parts.get(animations + 4..)?;
    let mut original: Vec<&str> = parts[..=animations].to_vec();
    original.extend_from_slice(rest);
    Some((replacer, original.join("\\")))
}

/// Is `path` (as [`stands_for`] takes it) in a replacer's folder?
#[inline]
pub fn is_replacer(path: &str) -> bool {
    stands_for(path).is_some()
}

/// The hkx of DAR / OAR under `mod_root` that stand in for `original` (relative to `mod_root`, `\` or
/// `/` separated, any case), by path: for a mod that has only them, not the hkx they stand in for.
///
/// e.g. `meshes/actors/character/animations/mt_idle.hkx` ->
/// `[<mod_root>/meshes/actors/character/animations/OpenAnimationReplacer/Mod/Sub/mt_idle.hkx]`
pub fn stand_ins(mod_root: &std::path::Path, original: &str) -> Vec<std::path::PathBuf> {
    let wanted = original.replace('/', "\\");
    let parts: Vec<&str> = wanted.split('\\').collect();
    let Some(animations) = parts.iter().position(|part| part.eq_ignore_ascii_case("animations")) else {
        return Vec::new();
    };
    let animations_dir = parts[..=animations].iter().fold(mod_root.to_path_buf(), |dir, part| dir.join(part));

    let mut found = Vec::new();
    let mut dirs: Vec<std::path::PathBuf> =
        [Replacer::Dar, Replacer::Oar].map(|r| animations_dir.join(r.folder())).into();
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            let relative = path.strip_prefix(mod_root).ok().and_then(|relative| relative.to_str());
            let stands =
                relative.and_then(stands_for).is_some_and(|(_, original)| original.eq_ignore_ascii_case(&wanted));
            if stands {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_replacer_stands_in_for_the_path_after_its_folders() {
        let dar = r"meshes\actors\character\animations\DynamicAnimationReplacer\_CustomConditions\100\male\mt_idle.hkx";
        assert_eq!(
            stands_for(dar),
            Some((Replacer::Dar, r"meshes\actors\character\animations\male\mt_idle.hkx".into()))
        );

        let actor_base =
            "meshes/actors/character/_1stperson/animations/dynamicanimationreplacer/Skyrim.esm/00000007/a.hkx";
        let expected = r"meshes\actors\character\_1stperson\animations\a.hkx".to_owned();
        assert_eq!(stands_for(actor_base), Some((Replacer::Dar, expected)), "any case, `/` too");

        let oar = r"meshes\actors\character\animations\OpenAnimationReplacer\Mod\Sub\idle.hkx";
        assert_eq!(stands_for(oar), Some((Replacer::Oar, r"meshes\actors\character\animations\idle.hkx".into())));
        let folder = r"meshes\actors\character\animations\OpenAnimationReplacer\Mod\Sub";
        assert_eq!(stands_for(folder), Some((Replacer::Oar, r"meshes\actors\character\animations".into())));

        assert_eq!(stands_for(r"meshes\actors\character\animations\mymod\idle.hkx"), None);
        assert!(!is_replacer(r"meshes\actors\character\animations\OpenAnimationReplacer\Mod"), "no submod yet");
    }

    #[test]
    fn the_stand_ins_of_an_animation_are_found_under_the_mod() {
        let root = std::env::temp_dir().join(format!("d_merge_stand_ins_{}", std::process::id()));
        let animations = root.join("meshes/actors/character/animations");
        let oar = animations.join("OpenAnimationReplacer/Mod/Sub/male");
        let dar = animations.join("DynamicAnimationReplacer/_CustomConditions/100/male");
        for dir in [&oar, &dar] {
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(dir.join("MT_Idle.hkx"), "").unwrap();
            std::fs::write(dir.join("other.hkx"), "").unwrap();
        }

        let found = stand_ins(&root, "meshes/actors/character/animations/male/mt_idle.hkx");
        assert_eq!(found, [dar.join("MT_Idle.hkx"), oar.join("MT_Idle.hkx")], "the file in any case, by path");
        assert!(stand_ins(&root, "meshes/actors/character/animations/mt_idle.hkx").is_empty(), "not in `male`");
        std::fs::remove_dir_all(&root).unwrap();
    }
}
