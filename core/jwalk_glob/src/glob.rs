/// A minimal glob pattern matcher for filesystem paths.
///
/// # Supported syntax
/// - `?`  — matches any single character within a path component
/// - `*`  — matches any sequence of characters within a single path component
/// - `**` — matches zero or more path components (must be its own component)
///
/// Matching is always **case-insensitive** (ASCII only).
pub(crate) fn match_segment(pattern: &str, name: &str) -> bool {
    match_segment_inner(
        &pattern.chars().collect::<Vec<_>>(),
        &name.chars().collect::<Vec<_>>(),
    )
}

fn match_segment_inner(p: &[char], n: &[char]) -> bool {
    match (p, n) {
        // Both exhausted — full match
        ([], []) => true,
        // Pattern exhausted but name remains — no match
        ([], _) => false,
        // Name exhausted — match only if remaining pattern is all `*`
        (_, []) => p.iter().all(|&c| c == '*'),
        // `*` matches zero chars (skip `*`) or one more char (advance name)
        (['*', p_rest @ ..], _) => {
            match_segment_inner(p_rest, n) || match_segment_inner(p, &n[1..])
        }
        // `?` matches exactly one char
        (['?', p_rest @ ..], [_, n_rest @ ..]) => match_segment_inner(p_rest, n_rest),
        // Literal character — case-insensitive
        ([pc, p_rest @ ..], [nc, n_rest @ ..]) => {
            pc.eq_ignore_ascii_case(nc) && match_segment_inner(p_rest, n_rest)
        }
    }
}

/// Matches a full filesystem path against a glob pattern.
///
/// Pattern components are split on `/` or `\`.
/// Path components are extracted via [`std::path::Path::components`].
///
/// `**` must occupy its own component and matches zero or more path components.
pub(crate) fn match_glob_path(pattern: &str, path: &std::path::Path) -> bool {
    let pat_components: Vec<&str> = pattern
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .collect();
    let path_components: Vec<&str> = path
        .components()
        .filter_map(|c| match c {
            // Normalize RootDir (`/` or `\`) — already represented by structure, skip
            std::path::Component::RootDir => None,
            // Prefix (`C:`) and Normal components — keep as str
            c => c.as_os_str().to_str(),
        })
        .collect();
    match_glob_components(&pat_components, &path_components)
}

fn match_glob_components(pat: &[&str], path: &[&str]) -> bool {
    match (pat, path) {
        // Both exhausted — full match
        ([], []) => true,
        // Pattern exhausted but path remains — no match
        ([], _) => false,
        // Path exhausted — match only if all remaining pattern components are `**`
        (_, []) => pat.iter().all(|&s| s == "**"),
        // `**` matches zero or more path components
        (["**", pat_rest @ ..], _) => {
            (0..=path.len()).any(|i| match_glob_components(pat_rest, &path[i..]))
        }
        // Literal or `*`/`?` segment — match current component then recurse
        ([p, pat_rest @ ..], [n, path_rest @ ..]) => {
            match_segment(p, n) && match_glob_components(pat_rest, path_rest)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    // ── match_segment ────────────────────────────────────────────────────────

    #[test]
    fn segment_literal_match() {
        assert!(match_segment("hello", "hello"));
    }

    #[test]
    fn segment_literal_case_insensitive() {
        assert!(match_segment("Hello", "hELLO"));
    }

    #[test]
    fn segment_literal_no_match() {
        assert!(!match_segment("hello", "world"));
    }

    #[test]
    fn segment_star_empty() {
        assert!(match_segment("*", ""));
    }

    #[test]
    fn segment_star_any() {
        assert!(match_segment("*", "anything"));
    }

    #[test]
    fn segment_star_prefix() {
        assert!(match_segment("FNIS*", "FNISFlyer"));
        assert!(match_segment("fnis*", "FNISFlyer")); // case-insensitive
        assert!(!match_segment("FNIS*", "TKDodge"));
    }

    #[test]
    fn segment_star_suffix() {
        assert!(match_segment("*Flyer", "FNISFlyer"));
        assert!(!match_segment("*Flyer", "FNISZoo"));
    }

    #[test]
    fn segment_star_infix() {
        assert!(match_segment("F*r", "FNISFlyer"));
        assert!(!match_segment("F*r", "FNISFlye"));
    }

    #[test]
    fn segment_question_mark() {
        assert!(match_segment("f?is", "fnis"));
        assert!(!match_segment("f?is", "fnnis"));
        assert!(!match_segment("f?is", "fis"));
    }

    #[test]
    fn segment_multiple_stars() {
        assert!(match_segment("a*b*c", "aXbYc"));
        assert!(!match_segment("a*b*c", "aXYc"));
    }

    // ── match_glob_path ──────────────────────────────────────────────────────

    #[test]
    fn glob_trailing_star() {
        assert!(match_glob_path("mods/*", Path::new("mods/FNISFlyer")));
        assert!(!match_glob_path("mods/*", Path::new("mods/a/b")));
    }

    #[test]
    fn glob_mid_star() {
        assert!(match_glob_path(
            "mods/*/Data",
            Path::new("mods/FNISFlyer/Data")
        ));
        assert!(!match_glob_path("mods/*/Data", Path::new("mods/a/b/Data")));
    }

    #[test]
    fn glob_partial_name() {
        assert!(match_glob_path(
            "mods/FNIS*/Data",
            Path::new("mods/FNISFlyer/Data")
        ));
        assert!(!match_glob_path(
            "mods/FNIS*/Data",
            Path::new("mods/TKDodge/Data")
        ));
    }

    #[test]
    fn glob_double_star_zero_components() {
        assert!(match_glob_path("mods/**/Data", Path::new("mods/Data")));
    }

    #[test]
    fn glob_double_star_one_component() {
        assert!(match_glob_path("mods/**/Data", Path::new("mods/a/Data")));
    }

    #[test]
    fn glob_double_star_many_components() {
        assert!(match_glob_path(
            "mods/**/Data",
            Path::new("mods/a/b/c/Data")
        ));
        assert!(!match_glob_path(
            "mods/**/Data",
            Path::new("mods/a/b/Other")
        ));
    }

    #[test]
    fn glob_double_star_only() {
        assert!(match_glob_path("**", Path::new("anything/at/all")));
        assert!(match_glob_path("**", Path::new("single")));
    }

    #[test]
    fn glob_case_insensitive() {
        assert!(match_glob_path(
            "MODS/*/data",
            Path::new("mods/FNISFlyer/Data")
        ));
    }

    #[test]
    fn glob_question_mark() {
        assert!(match_glob_path(
            "mods/FNI?Flyer",
            Path::new("mods/FNISFlyer")
        ));
        assert!(!match_glob_path(
            "mods/FNI?Flyer",
            Path::new("mods/FNISSFlyer")
        ));
    }

    #[test]
    fn glob_no_pattern_literal_match() {
        assert!(match_glob_path(
            "C:/Skyrim/Data",
            Path::new("C:/Skyrim/Data")
        ));
        assert!(!match_glob_path(
            "C:/Skyrim/Data",
            Path::new("C:/Skyrim/Other")
        ));
    }

    #[test]
    fn glob_backslash_separator_in_pattern() {
        assert!(match_glob_path(
            "mods\\*\\Data",
            Path::new("mods/FNISFlyer/Data")
        ));
    }
}
