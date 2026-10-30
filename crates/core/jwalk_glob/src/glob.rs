/// A minimal glob pattern matcher for filesystem paths.
///
/// # Supported syntax
/// - `?`  — matches any single character within a path component
/// - `*`  — matches any sequence of characters within a single path component
/// - `**` — matches zero or more path components (must be its own component)
///
/// Matching is always **case-insensitive** (ASCII only).
pub(crate) fn match_segment(pattern: &str, name: &str) -> bool {
    match_segment_inner(&pattern.chars().collect::<Vec<_>>(), &name.chars().collect::<Vec<_>>())
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

#[cfg(test)]
mod tests {
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
}
