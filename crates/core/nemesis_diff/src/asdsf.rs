//! `animationsetdatasinglefile.txt` split into its anim sets, the unit of a Nemesis asdsf patch.
//!
//! A Nemesis mod patches one anim set per file,
//! `animationsetdatasinglefile/DefaultMaleData~DefaultMale/_MTSolo.txt`, with the same
//! `MOD_CODE` blocks as a behavior object (so [`crate::object_patch`] works on [`AnimSet::text`]).
//!
//! # File layout
//! ```text
//! 49                                  project count
//! DefaultMaleData\DefaultMale.txt     project names (-> folder `DefaultMaleData~DefaultMale`)
//! ...
//! (per project, in order)
//! 3                                   anim set file count (absent: no sets)
//! _MTSolo.txt                         file names
//! ...
//! V3                                  an anim set, per file:
//! 0  <triggers>  0  <conditions: 3 lines each>  4  <attacks>  106  <anim infos: 3 lines each>
//! ```
//! An attack is `trigger`, `0|1`, `clip count`, clip names.

/// One anim set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimSet {
    /// Patch folder, e.g. `DefaultMaleData~DefaultMale`
    pub project: String,
    /// e.g. `_MTSolo.txt`
    pub file: String,
    /// From `V3` to its last anim info, each line ending with `\n`.
    pub text: String,
}

impl AnimSet {
    /// `project/file`, e.g. `DefaultMaleData~DefaultMale/_MTSolo.txt` (its patch path).
    pub fn id(&self) -> String {
        format!("{}/{}", self.project, self.file)
    }
}

/// Splits an `animationsetdatasinglefile.txt` (`\n` or `\r\n`).
///
/// # Errors
/// The text does not have the layout above (the line number is in the message).
pub fn anim_sets(asdsf: &str) -> Result<Vec<AnimSet>, String> {
    let mut lines = Lines { lines: asdsf.lines().collect(), at: 0 };

    let project_count = lines.number()?;
    let mut projects = Vec::with_capacity(project_count);
    for _ in 0..project_count {
        // `DefaultMaleData\DefaultMale.txt` -> `DefaultMaleData~DefaultMale`
        projects.push(lines.next()?.trim_end_matches(".txt").replace(['\\', '/'], "~"));
    }

    let mut sets = Vec::new();
    for project in projects {
        // Like d-merge's asdsf parser: no count line (the next line is `V3`) = no files.
        match lines.peek() {
            None => break,
            Some("V3") => continue,
            Some(_) => {}
        }
        let file_count = lines.number()?;
        let files: Vec<String> =
            (0..file_count).map(|_| lines.next().map(str::to_string)).collect::<Result<_, _>>()?;

        for file in files {
            let start = lines.at;
            if lines.next()? != "V3" {
                return Err(format!("{project}/{file}: line {}: expected `V3`", start + 1));
            }
            let triggers = lines.number()?;
            lines.skip(triggers)?;
            let conditions = lines.number()?;
            lines.skip(conditions * 3)?;
            for _ in 0..lines.number()? {
                lines.skip(2)?; // trigger, 0|1
                let clips = lines.number()?;
                lines.skip(clips)?;
            }
            let anim_infos = lines.number()?;
            lines.skip(anim_infos * 3)?;

            let mut text = lines.lines[start..lines.at].join("\n");
            text.push('\n');
            sets.push(AnimSet { project: project.clone(), file, text });
        }
    }
    Ok(sets)
}

/// Line cursor with errors that tell the line number.
struct Lines<'a> {
    lines: Vec<&'a str>,
    at: usize,
}

impl<'a> Lines<'a> {
    fn peek(&self) -> Option<&'a str> {
        self.lines.get(self.at).copied()
    }

    fn next(&mut self) -> Result<&'a str, String> {
        let line = self.peek().ok_or_else(|| format!("line {}: unexpected end", self.at + 1))?;
        self.at += 1;
        Ok(line)
    }

    fn number(&mut self) -> Result<usize, String> {
        let line = self.next()?;
        line.trim()
            .parse()
            .map_err(|_| format!("line {}: expected a number, got `{line}`", self.at))
    }

    fn skip(&mut self, n: usize) -> Result<(), String> {
        for _ in 0..n {
            self.next()?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_sets() {
        let txt = "2\nAData\\A.txt\nBData\\B.txt\n1\nx.txt\nV3\n1\ntrig\n1\nvar\n0\n1\n1\natk\n0\n1\nclip\n1\n1\n2\n3\n0\n";
        // the trailing `0`: project B has no sets.
        let sets = anim_sets(txt).unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].id(), "AData~A/x.txt");
        assert!(sets[0].text.starts_with("V3\n1\ntrig\n") && sets[0].text.ends_with("1\n2\n3\n"));
    }
}
