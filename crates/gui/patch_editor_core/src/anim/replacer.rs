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
}
