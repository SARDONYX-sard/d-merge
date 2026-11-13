//! Time as people read it.

use core::fmt;

/// How long ago `time` was at `now` (both seconds since the Unix epoch), as GitLens and GitHub say
/// it. Shown with `{}`; no text is made until then.
///
/// e.g. 90 s -> `1 minute ago`, 3 days -> `3 days ago`, under a minute -> `just now`
#[must_use]
#[inline]
pub const fn ago(now: u64, time: u64) -> Ago {
    Ago(now.saturating_sub(time))
}

/// See [`ago`]. Seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ago(pub u64);

impl fmt::Display for Ago {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const UNITS: [(u64, &str); 5] =
            [(31_536_000, "year"), (2_592_000, "month"), (86_400, "day"), (3600, "hour"), (60, "minute")];
        let Some(&(unit, name)) = UNITS.iter().find(|(unit, _)| self.0 >= *unit) else {
            return f.write_str("just now");
        };
        let count = self.0 / unit;
        write!(f, "{count} {name}{} ago", if count == 1 { "" } else { "s" })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn how_long_ago_as_gitlens_says_it() {
        assert_eq!(ago(100, 90).to_string(), "just now");
        assert_eq!(ago(200, 100).to_string(), "1 minute ago");
        assert_eq!(ago(3 * 86_400 + 5, 5).to_string(), "3 days ago");
        assert_eq!(ago(2 * 2_592_000, 0).to_string(), "2 months ago");
        assert_eq!(ago(31_536_000, 0).to_string(), "1 year ago");
        assert_eq!(ago(0, 100).to_string(), "just now", "in the future");
    }
}
