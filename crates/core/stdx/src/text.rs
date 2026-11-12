//! Text helpers that need no allocation: for what runs on many strings, each frame or each key.

/// Does `haystack` contain `needle`, ignoring ASCII case? Names here are ASCII: `to_lowercase` would
/// allocate a string for each of them, only to compare it.
///
/// e.g., `SoundPlay.Foo` contains `soundp`. Everything contains the empty text.
#[must_use]
#[inline]
pub fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    let (haystack, needle) = (haystack.as_bytes(), needle.as_bytes());
    needle.is_empty() || haystack.windows(needle.len()).any(|window| window.eq_ignore_ascii_case(needle))
}

/// Does `text` start with `prefix`, ignoring ASCII case? `const`: also for checks at compile time.
///
/// e.g., `#Mod$0` starts with `#mod`.
#[must_use]
#[inline]
pub const fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
    let (text, prefix) = (text.as_bytes(), prefix.as_bytes());
    text.len() >= prefix.len() && text.split_at(prefix.len()).0.eq_ignore_ascii_case(prefix)
}

/// Does `text` end with `suffix`, ignoring ASCII case? `const`, as [`starts_with_ignore_case`].
///
/// e.g., `0_master.HKX` ends with `.hkx`.
#[must_use]
#[inline]
pub const fn ends_with_ignore_case(text: &str, suffix: &str) -> bool {
    let (text, suffix) = (text.as_bytes(), suffix.as_bytes());
    text.len() >= suffix.len() && text.split_at(text.len() - suffix.len()).1.eq_ignore_ascii_case(suffix)
}

/// The two texts in order, ignoring ASCII case.
#[must_use]
#[inline]
pub fn cmp_ignore_case(a: &str, b: &str) -> core::cmp::Ordering {
    a.bytes().map(|byte| byte.to_ascii_lowercase()).cmp(b.bytes().map(|byte| byte.to_ascii_lowercase()))
}

/// `items` with `separator` between them, at the end of `out`.
///
/// e.g., `a`, `b` and `, ` -> `a, b`
#[inline]
pub fn push_joined<'a>(out: &mut String, items: impl IntoIterator<Item = &'a str>, separator: &str) {
    for (at, item) in items.into_iter().enumerate() {
        if at > 0 {
            out.push_str(separator);
        }
        out.push_str(item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_ignores_ascii_case() {
        assert!(contains_ignore_case("SoundPlay.Foo", "soundp"));
        assert!(contains_ignore_case("SoundPlay.Foo", "FOO"));
        assert!(contains_ignore_case("anything", ""));
        assert!(contains_ignore_case("", ""));
        assert!(!contains_ignore_case("abc", "abcd"));
        assert!(!contains_ignore_case("abc", "x"));
        assert!(contains_ignore_case("日本語のclip", "CLIP"), "bytes of other scripts just compare");
        assert!(contains_ignore_case("日本語", "本"));
    }

    #[test]
    fn starts_with_ignores_ascii_case() {
        assert!(starts_with_ignore_case("#Mod$0", "#mod"));
        assert!(starts_with_ignore_case("null", ""));
        assert!(!starts_with_ignore_case("nu", "null"));
        assert!(!starts_with_ignore_case("#Mod", "mod"));
    }

    #[test]
    fn ends_with_ignores_ascii_case() {
        assert!(ends_with_ignore_case("0_master.HKX", ".hkx"));
        assert!(ends_with_ignore_case("x", ""));
        assert!(!ends_with_ignore_case("kx", ".hkx"));
        const _AT_COMPILE_TIME: () = assert!(ends_with_ignore_case("A.TXT", ".txt"));
    }

    #[test]
    fn orders_ignoring_ascii_case() {
        use core::cmp::Ordering::{Equal, Greater, Less};
        assert_eq!(cmp_ignore_case("abc", "ABD"), Less);
        assert_eq!(cmp_ignore_case("B", "a"), Greater);
        assert_eq!(cmp_ignore_case("Same", "sAME"), Equal);
    }

    #[test]
    fn joins_at_the_end() {
        let mut out = "deps=".to_owned();
        push_joined(&mut out, ["a", "b"], ",");
        assert_eq!(out, "deps=a,b");
        push_joined(&mut out, [], ",");
        assert_eq!(out, "deps=a,b");
    }
}
