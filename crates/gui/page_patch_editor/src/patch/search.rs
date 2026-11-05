//! Searching the text of every object of a behavior, like VS Code's `Ctrl+Shift+F`.
//!
//! e.g., `$eventID[moveStart]$` finds where a mod uses that event; `#0106` finds who points to it.
//! Plain text, ignoring ASCII case. Drawn by [`super::ui::search`].

use super::{checklist::contains_ignore_ascii_case, workspace::ObjectText};

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
}

/// See the module docs.
#[derive(Debug, Default)]
pub(super) struct Search {
    pub(super) query: String,
    pub(super) hits: Vec<Hit>,
    /// What `hits` was computed from: the query, and the state of the objects.
    searched: Option<(String, usize, u64)>,
}

impl Search {
    /// Searches again if the query, the behavior or the objects changed.
    pub(super) fn refresh<'a>(
        &mut self,
        behavior: usize,
        generation: u64,
        objects: impl Iterator<Item = ObjectText<'a>>,
    ) {
        let unchanged = self.searched.as_ref().is_some_and(|(query, searched, searched_generation)| {
            *query == self.query && *searched == behavior && *searched_generation == generation
        });
        if !unchanged {
            self.hits = find(objects, self.query.trim());
            self.searched = Some((self.query.clone(), behavior, generation));
        }
    }

    /// Hit the limit: there may be more.
    pub(super) const fn is_cut(&self) -> bool {
        self.hits.len() >= MAX_HITS
    }
}

/// The lines containing `query`.
fn find<'a>(objects: impl Iterator<Item = ObjectText<'a>>, query: &str) -> Vec<Hit> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for object in objects {
        for (line, text) in object.text.lines().enumerate() {
            if contains_ignore_ascii_case(text, query) {
                hits.push(Hit { object: object.id.into(), line, text: text.trim().into() });
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
        let hits = find(objects.into_iter(), "$eventid[moveStart]$");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0], Hit { object: "#1".into(), line: 1, text: "$eventID[moveStart]$".into() });
        assert_eq!(&*hits[1].object, "#2");
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
