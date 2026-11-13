//! Resolving a conflict in the editor as git's merge editor does: the mods' values side by side, one
//! taken (or all of them), and the resolution kept to be done again.
//!
//! ```text
//! #2489/hkbManualSelectorGenerator/generators/[0:1]
//! Original  | Flinching       | Impact
//! #0338     | #flinch$2       | #neymar$1019
//!           [Accept]          [Accept]          [Accept all]
//! ```
//! - Accept: the lines of the conflict in the object are replaced by that mod's. Where they are is
//!   found by what is there now: the original lines, or what was accepted before ([`replace_block`]).
//! - Accept all: every mod's lines, each line once, as git's "accept both".
//!
//! # Patterns
//! A resolution is kept by the conflict's path and what each mod has there ([`Resolution`]). When the
//! same conflict comes again (the same mods with the same values), it is resolved the same way at once.
//! A mod that changes its value makes another conflict: nothing is reused for it.

use std::hash::{Hash as _, Hasher as _};

use super::resolve::MergeValues;

/// A resolution kept. See the module docs.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Resolution {
    /// e.g. `#2489/hkbManualSelectorGenerator/generators/[0:1]`
    pub(crate) path: String,
    /// Each mod's name and a hash of its lines there, sorted: which conflict this was.
    pub(crate) sides: Vec<(String, u64)>,
    /// The lines it was resolved to.
    pub(crate) result: Vec<String>,
}

/// A hash of `lines`, as they are compared: without their indentation.
fn lines_hash(lines: &[String]) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    for line in lines {
        line.trim().hash(&mut hasher);
    }
    hasher.finish()
}

/// What tells conflict `values` from another: its mods and their lines.
fn sides_of(values: &MergeValues) -> Vec<(String, u64)> {
    let mut sides: Vec<(String, u64)> =
        values.sides.iter().map(|side| (side.name.clone(), lines_hash(&side.lines))).collect();
    sides.sort();
    sides
}

/// The resolution kept for `values`, if this conflict was resolved before.
pub(crate) fn find<'a>(resolutions: &'a [Resolution], values: &MergeValues) -> Option<&'a Resolution> {
    let sides = sides_of(values);
    resolutions.iter().find(|resolution| resolution.path == values.path && resolution.sides == sides)
}

/// Keeps that `values` was resolved to `result`, in place of what was kept for it before.
pub(crate) fn remember(resolutions: &mut Vec<Resolution>, values: &MergeValues, result: Vec<String>) {
    let sides = sides_of(values);
    resolutions.retain(|resolution| resolution.path != values.path || resolution.sides != sides);
    resolutions.push(Resolution { path: values.path.clone(), sides, result });
}

/// Every mod's lines, each line once (by its text without indentation): git's "accept both".
///
/// e.g. `a`, `b` and `b`, `c` -> `a`, `b`, `c`
pub(crate) fn all_lines(values: &MergeValues) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for line in values.sides.iter().flat_map(|side| &side.lines) {
        if !lines.iter().any(|known| known.trim() == line.trim()) {
            lines.push(line.clone());
        }
    }
    lines
}

/// `text` with the first run of lines equal to one of `candidates` (ignoring indentation) replaced
/// by `with`. `None`: none of them is there.
///
/// e.g. `a\nX\nb\n`, candidates `X` -> with `Y`, `Z` -> `a\nY\nZ\nb\n`
pub(crate) fn replace_block(text: &str, candidates: &[&[String]], with: &[String]) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let found = candidates.iter().filter(|candidate| !candidate.is_empty()).find_map(|candidate| {
        let at = lines.windows(candidate.len()).position(|window| {
            window.iter().zip(candidate.iter()).all(|(line, wanted)| line.trim() == wanted.trim())
        })?;
        Some(at..at + candidate.len())
    })?;
    let mut out: Vec<&str> = lines[..found.start].to_vec();
    out.extend(with.iter().map(String::as_str));
    out.extend_from_slice(&lines[found.end..]);
    let mut joined = out.join("\n");
    joined.push('\n');
    Some(joined)
}

/// Where the lines of the conflict may be in the object now: what was resolved before, each mod's,
/// and the original.
pub(crate) fn candidates<'a>(values: &'a MergeValues, before: &'a [String]) -> Vec<&'a [String]> {
    let mut candidates: Vec<&[String]> = vec![before];
    candidates.extend(values.sides.iter().map(|side| side.lines.as_slice()));
    candidates.push(&values.original);
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::editor::resolve::MergeSide;

    fn values() -> MergeValues {
        let side = |name: &str, lines: &[&str]| MergeSide {
            name: name.to_owned(),
            lines: lines.iter().map(|line| (*line).to_owned()).collect(),
        };
        MergeValues {
            id: "#2489".to_owned(),
            path: "#2489/x/generators/[0:1]".to_owned(),
            original: vec!["\t#0338".to_owned()],
            sides: vec![side("Flinching", &["\t#flinch$2"]), side("Impact", &["\t#neymar$1019", "\t#flinch$2"])],
        }
    }

    #[test]
    fn accept_replaces_the_lines_of_the_conflict() {
        let values = values();
        let text = "a\n\t#0338\nb\n";
        let accepted = replace_block(text, &candidates(&values, &[]), &values.sides[0].lines).unwrap();
        assert_eq!(accepted, "a\n\t#flinch$2\nb\n");
        // Accepting another now finds what was accepted.
        let all = all_lines(&values);
        assert_eq!(all, ["\t#flinch$2", "\t#neymar$1019"]);
        let again = replace_block(&accepted, &candidates(&values, &[]), &all).unwrap();
        assert_eq!(again, "a\n\t#flinch$2\n\t#neymar$1019\nb\n");
        assert_eq!(replace_block("a\n", &candidates(&values, &[]), &all), None);
    }

    #[test]
    fn a_resolution_is_found_again_for_the_same_conflict_only() {
        let values = values();
        let mut kept = Vec::new();
        remember(&mut kept, &values, vec!["\t#flinch$2".to_owned()]);
        remember(&mut kept, &values, all_lines(&values));
        assert_eq!(kept.len(), 1, "the last resolution of it");
        assert_eq!(find(&kept, &values).map(|resolution| resolution.result.len()), Some(2));

        let mut changed = values.clone();
        changed.sides[0].lines = vec!["\t#flinch$9".to_owned()];
        assert_eq!(find(&kept, &changed), None, "a mod changed its value: another conflict");
    }
}
