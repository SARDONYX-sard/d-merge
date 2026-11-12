//! Searching the text of every object of a behavior, like VS Code's `Ctrl+Shift+F`.
//!
//! e.g., `$eventID[moveStart]$` finds where a mod uses that event; `#0106` finds who points to it.
//! Text ignoring ASCII case, or with `.*` on a regex ignoring case (e.g. `#0\d{3}`). A regex not
//! finished yet finds nothing. Drawn by [`super::ui::search`].

use std::ops::Range;

use super::workspace::ObjectText;

/// The most hits kept: a short query matches nearly every line of a big behavior.
const MAX_HITS: usize = 1000;

/// One matching line. Kept across frames, so it owns its text.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Hit {
    pub(super) object: Box<str>,
    /// 0-based.
    pub(super) line: usize,
    /// Without its indent.
    pub(super) text: Box<str>,
    /// Where `text` matches (byte ranges), to show them.
    pub(super) matches: Box<[Range<usize>]>,
}

/// See the module docs.
#[derive(Debug, Default)]
pub(super) struct Search {
    pub(super) query: String,
    /// `.*`: `query` is a regex.
    pub(super) regex: bool,
    pub(super) hits: Vec<Hit>,
    /// The objects whose hits are folded away, as VS Code's results of a file.
    pub(super) folded: fast_hash::HashSet<Box<str>>,
    /// What `hits` was computed from: the query, `regex`, and the state of the objects.
    searched: Option<(String, bool, usize, u64)>,
}

impl Search {
    /// Searches again if the query, the behavior or the objects changed.
    pub(super) fn refresh<'a>(
        &mut self,
        behavior: usize,
        generation: u64,
        objects: impl Iterator<Item = ObjectText<'a>>,
    ) {
        let unchanged = self.searched.as_ref().is_some_and(|(query, regex, searched, searched_generation)| {
            *query == self.query && *regex == self.regex && *searched == behavior && *searched_generation == generation
        });
        if !unchanged {
            let query = self.query.trim();
            self.hits = if self.regex {
                // `(?i)`: as the text search, case does not matter.
                regex::RegexBuilder::new(query).case_insensitive(true).build().map_or_else(
                    |_| Vec::new(),
                    |regex| find(objects, query, |line| regex.find_iter(line).map(|found| found.range()).collect()),
                )
            } else {
                find(objects, query, |line| text_matches(line, query))
            };
            self.searched = Some((self.query.clone(), self.regex, behavior, generation));
        }
    }

    /// Hit the limit: there may be more.
    pub(super) const fn is_cut(&self) -> bool {
        self.hits.len() >= MAX_HITS
    }

    /// The rows of the results: each object (with its count), then its hits unless folded.
    ///
    /// e.g. hits `#1:3`, `#1:9`, `#2:0` -> `#1 (2)`, `#1:3`, `#1:9`, `#2 (1)`, `#2:0`
    pub(super) fn rows(&self) -> Vec<SearchRow> {
        let mut rows = Vec::with_capacity(self.hits.len() + 16);
        let mut at = 0;
        for group in self.hits.chunk_by(|a, b| a.object == b.object) {
            rows.push(SearchRow::Object { first: at, count: group.len() });
            if !self.folded.contains(&group[0].object) {
                rows.extend((at..at + group.len()).map(SearchRow::Hit));
            }
            at += group.len();
        }
        rows
    }

    /// How many objects have hits.
    pub(super) fn object_count(&self) -> usize {
        self.hits.chunk_by(|a, b| a.object == b.object).count()
    }

    /// Folds or unfolds the hits of `object`.
    pub(super) fn toggle(&mut self, object: &str) {
        if !self.folded.remove(object) {
            self.folded.insert(object.into());
        }
    }
}

/// A row of the results. See [`Search::rows`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SearchRow {
    /// The object of `hits[first]`, with `count` hits.
    Object { first: usize, count: usize },
    /// `hits[index]`.
    Hit(usize),
}

/// Where `query` is in `line`, ignoring ASCII case (byte ranges, not overlapping).
///
/// e.g. `ab Ab`, `ab` -> `0..2`, `3..5`
fn text_matches(line: &str, query: &str) -> Vec<Range<usize>> {
    let (bytes, query) = (line.as_bytes(), query.as_bytes());
    let mut found = Vec::new();
    let mut at = 0;
    while !query.is_empty() && at + query.len() <= bytes.len() {
        if bytes[at..at + query.len()].eq_ignore_ascii_case(query) {
            found.push(at..at + query.len());
            at += query.len();
        } else {
            at += 1;
        }
    }
    found
}

/// The lines where `matches` finds something (none for an empty `query`), with what it found.
fn find<'a>(
    objects: impl Iterator<Item = ObjectText<'a>>,
    query: &str,
    matches: impl Fn(&str) -> Vec<Range<usize>>,
) -> Vec<Hit> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for object in objects {
        for (line, text) in object.text.lines().enumerate() {
            let text = text.trim();
            let found = matches(text);
            if !found.is_empty() {
                hits.push(Hit { object: object.id.into(), line, text: text.into(), matches: found.into() });
                if hits.len() >= MAX_HITS {
                    return hits;
                }
            }
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object<'a>(id: &'a str, text: &'a str) -> ObjectText<'a> {
        ObjectText { id, class: "c", name: "", text, is_new: false, is_edited: false }
    }

    #[test]
    fn finds_lines_ignoring_case() {
        let objects =
            [object("#1", "a\n\t$eventID[moveStart]$\n"), object("#2", "$EVENTID[movestart]$"), object("#3", "b")];
        let mut search = Search { query: "$eventid[moveStart]$".to_owned(), ..Search::default() };
        search.refresh(0, 1, objects.into_iter());
        assert_eq!(search.hits.len(), 2);
        assert_eq!(
            search.hits[0],
            Hit {
                object: "#1".into(),
                line: 1,
                text: "$eventID[moveStart]$".into(),
                matches: std::iter::once(0..20).collect()
            }
        );
        assert_eq!(&*search.hits[1].object, "#2");
    }

    /// `.*`: the same query is a regex, ignoring case too; one not finished yet finds nothing.
    #[test]
    fn a_regex_finds_lines_too() {
        let objects = || [object("#1", "<hkparam>#0106</hkparam>\n#M$0"), object("#2", "#12")];
        let mut search = Search { query: r"#\d{4}|#m\$".to_owned(), regex: true, ..Search::default() };
        search.refresh(0, 1, objects().into_iter());
        assert_eq!(search.hits.iter().map(|hit| hit.line).collect::<Vec<_>>(), [0, 1]);

        search.query = "[0-".to_owned();
        search.refresh(0, 1, objects().into_iter());
        assert!(search.hits.is_empty());
    }

    #[test]
    fn hits_are_shown_under_their_object_unless_folded() {
        let objects = [object("#1", "a\nb\na"), object("#2", "a")];
        let mut search = Search { query: "a".to_owned(), ..Search::default() };
        search.refresh(0, 1, objects.into_iter());
        let object = |first, count| SearchRow::Object { first, count };
        assert_eq!(
            search.rows(),
            [object(0, 2), SearchRow::Hit(0), SearchRow::Hit(1), object(2, 1), SearchRow::Hit(2)]
        );
        assert_eq!(search.object_count(), 2);
        search.toggle("#1");
        assert_eq!(search.rows(), [object(0, 2), object(2, 1), SearchRow::Hit(2)]);
        assert_eq!(text_matches("ab Ab", "ab"), [0..2, 3..5]);
    }

    #[test]
    fn searches_again_only_when_something_changed() {
        let mut search = Search { query: "a".to_owned(), ..Search::default() };
        search.refresh(0, 1, std::iter::once(object("#1", "a")));
        assert_eq!(search.hits.len(), 1);
        search.refresh(0, 1, std::iter::once(object("#1", "b"))); // nothing said that it changed
        assert_eq!(search.hits.len(), 1);
        search.refresh(0, 2, std::iter::once(object("#1", "b")));
        assert!(search.hits.is_empty());

        search.query.clear();
        search.refresh(0, 2, std::iter::once(object("#1", "b")));
        assert!(search.hits.is_empty());
    }
}
