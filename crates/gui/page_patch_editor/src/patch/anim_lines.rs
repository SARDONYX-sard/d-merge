//! What each line of an animation data patch means: these files have no field names, only positions.
//!
//! ```text
//! V3                  Version                 animationsetdatasinglefile/<project>/<set>.txt
//! 1                   TriggersLen
//! attackStart         Trigger(0)
//! 1                   ConditionsLen
//! iLeftHandType       ConditionVariable(0)
//! 0                   ConditionA(0)
//! 0                   ConditionB(0)
//! 1                   AttacksLen
//! attackStart_A       AttackTrigger(0)
//! 0                   AttackContextual(0)
//! 1                   AttackClipsLen(0)
//! Attack1             AttackClip(0)
//! 1                   AnimInfosLen
//! 3064642194          AnimDir(0)
//! 1047251415          AnimFile(0)
//! 7891816             AnimExtension(0)
//! ```
//! ```text
//! Idle                ClipName                animationdatasinglefile/<project>~1/Idle~mod$0.txt
//! mod$0               ClipId
//! 1                   Speed
//! 0  0                CropStart, CropEnd
//! 1                   ClipTriggersLen
//! Draw:0.03           ClipTrigger(0)
//! ```
//! ```text
//! mod$0               MotionId                animationdatasinglefile/<project>~1/mod$0.txt
//! 3.3                 Duration
//! 1                   TranslationsLen
//! 3.3 0 0 0           Translation(0)
//! 1                   RotationsLen
//! 3.3 0 0 0 1         Rotation(0)
//! ```
//! `MOD_CODE` markers and the template's lines after `ORIGINAL` are not part of the layout.
//!
//! An anim set's anim infos go to the end of the file whatever its count says: a pushing patch does
//! not change the count (see [`nemesis_diff::asdsf::add_anim_infos`]).
//!
//! [`problems`] checks what can be wrong in a way the positions hide: a count that says something else
//! than what is there, and a clip id that is not the one in the file's name. Only a file that is a
//! whole thing: a patch that edits one pushes elements without changing the counts, which the merge
//! counts itself. e.g., `tkuc`'s `FirstPerson~1/346.txt` keeps `1` with 9 translations.

use egui_code_editor::{CompletionList, Diagnostic, Language, QuickFix, Severity, languages::Xml};

use super::{
    anim_patch::{AdsfFile, AsdsfFile, is_edit},
    language::{item, trigger_completion},
};

/// See the module docs. The number is the element's index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LineRole {
    Version,
    TriggersLen,
    Trigger(u32),
    ConditionsLen,
    ConditionVariable(u32),
    ConditionA(u32),
    ConditionB(u32),
    AttacksLen,
    AttackTrigger(u32),
    AttackContextual(u32),
    AttackClipsLen(u32),
    AttackClip(u32),
    AnimInfosLen,
    AnimDir(u32),
    AnimFile(u32),
    AnimExtension(u32),

    ClipName,
    ClipId,
    Speed,
    CropStart,
    CropEnd,
    ClipTriggersLen,
    ClipTrigger(u32),

    MotionId,
    Duration,
    TranslationsLen,
    Translation(u32),
    RotationsLen,
    Rotation(u32),

    /// A marker, a line of the template, a header, or what does not fit the layout.
    Other,
}

impl LineRole {
    /// The label at the end of the line, e.g., `animInfos[3].file`.
    pub(super) fn hint(self) -> String {
        match self {
            Self::Version => "version".to_owned(),
            Self::TriggersLen | Self::ClipTriggersLen => "triggers: count".to_owned(),
            Self::Trigger(i) | Self::ClipTrigger(i) => format!("triggers[{i}]"),
            Self::ConditionsLen => "conditions: count".to_owned(),
            Self::ConditionVariable(i) => format!("conditions[{i}].variable"),
            Self::ConditionA(i) => format!("conditions[{i}].a"),
            Self::ConditionB(i) => format!("conditions[{i}].b"),
            Self::AttacksLen => "attacks: count".to_owned(),
            Self::AttackTrigger(i) => format!("attacks[{i}].trigger"),
            Self::AttackContextual(i) => format!("attacks[{i}].isContextual"),
            Self::AttackClipsLen(i) => format!("attacks[{i}].clips: count"),
            Self::AttackClip(i) => format!("attacks[{i}].clip"),
            Self::AnimInfosLen => "animInfos: count".to_owned(),
            Self::AnimDir(i) => format!("animInfos[{i}].dir"),
            Self::AnimFile(i) => format!("animInfos[{i}].file"),
            Self::AnimExtension(i) => format!("animInfos[{i}].extension"),
            Self::ClipName => "clip name".to_owned(),
            Self::ClipId | Self::MotionId => "clip id".to_owned(),
            Self::Speed => "speed".to_owned(),
            Self::CropStart => "crop start".to_owned(),
            Self::CropEnd => "crop end".to_owned(),
            Self::Duration => "duration".to_owned(),
            Self::TranslationsLen => "translations: count".to_owned(),
            Self::Translation(i) => format!("translations[{i}]: time x y z"),
            Self::RotationsLen => "rotations: count".to_owned(),
            Self::Rotation(i) => format!("rotations[{i}]: time x y z w"),
            Self::Other => String::new(),
        }
    }

    /// What the hover says.
    pub(super) const fn doc(self) -> Option<&'static str> {
        Some(match self {
            Self::Version => "**Version** of the anim set. Always `V3`.",
            Self::TriggersLen => "**Count** of the triggers below. Nemesis and d-merge count them when merging.",
            Self::Trigger(_) => "**Trigger**: an event name this anim set reacts to.",
            Self::ConditionsLen => "**Count** of the conditions below.",
            Self::ConditionVariable(_) => "**Condition variable**: when this anim set applies, e.g. `iLeftHandType`.",
            Self::ConditionA(_) => "**Condition value A**: the lowest value of the variable.",
            Self::ConditionB(_) => "**Condition value B**: the highest value of the variable.",
            Self::AttacksLen => "**Count** of the attacks below.",
            Self::AttackTrigger(_) => "**Attack trigger**: the event that starts it, e.g. `attackStart_Attack1`.",
            Self::AttackContextual(_) => "**Is contextual**: `0` or `1`.",
            Self::AttackClipsLen(_) => "**Count** of this attack's clips below.",
            Self::AttackClip(_) => "**Clip name** of this attack.",
            Self::AnimInfosLen => {
                "**Count** of the animations below. A push does not change it: it is counted when merging."
            }
            Self::AnimDir(_) => {
                "**Directory** of an hkx to cache: the CRC32 of the lowercase path with backslashes, e.g. `meshes\\actors\\character\\animations\\a`. `$crc32[path]$` computes it."
            }
            Self::AnimFile(_) => {
                "**File name** of an hkx to cache: the CRC32 of the lowercase name without extension. `$crc32[name]$` computes it."
            }
            Self::AnimExtension(_) => "**Extension**: always `7891816`, the CRC32 of `hkx`.",
            Self::ClipName => "**Clip name**: the `name` of the `hkbClipGenerator`.",
            Self::ClipId => {
                "**Clip id**: `<mod code>$<number>` gets a free id when merging; the motion file has the same one."
            }
            Self::Speed => "**Playback speed**. `1` is normal.",
            Self::CropStart => "**Crop start**: seconds cut from the start.",
            Self::CropEnd => "**Crop end**: seconds cut from the end.",
            Self::ClipTriggersLen => "**Count** of the triggers below.",
            Self::ClipTrigger(_) => "**Trigger**: `name:time`, e.g. `AnimObjDraw:0.03`.",
            Self::MotionId => "**Clip id**: the same one as the clip's file.",
            Self::Duration => "**Duration** in seconds.",
            Self::TranslationsLen => "**Count** of the translation keys below.",
            Self::Translation(_) => "**Translation key**: `time x y z`. The last one is at the duration.",
            Self::RotationsLen => "**Count** of the rotation keys below.",
            Self::Rotation(_) => "**Rotation key**: `time x y z w`, a quaternion. The last one is at the duration.",
            Self::Other => return None,
        })
    }
}

/// Goes through the lines that belong to the layout, setting their roles.
struct Walk<'a> {
    /// `(line number, text)`
    lines: Vec<(usize, &'a str)>,
    at: usize,
    roles: Vec<LineRole>,
}

impl<'a> Walk<'a> {
    fn new(text: &'a str) -> Self {
        let mut in_original = false;
        let mut lines = Vec::new();
        let count = text.split('\n').count();
        for (no, line) in text.split('\n').enumerate() {
            let trimmed = line.trim();
            // Blank lines may separate elements. The last one is where the next line gets typed:
            // it has the role of what comes next, for the completion.
            let is_separator = trimmed.is_empty() && no + 1 < count;
            if trimmed.starts_with("<!-- ORIGINAL") {
                in_original = true;
            } else if trimmed.starts_with("<!-- CLOSE") {
                in_original = false;
            } else if !trimmed.starts_with("<!--") && !in_original && !is_separator {
                lines.push((no, line));
            }
        }
        Self { lines, at: 0, roles: vec![LineRole::Other; count] }
    }

    const fn is_done(&self) -> bool {
        self.at >= self.lines.len()
    }

    /// Gives the next line this role.
    fn set(&mut self, role: LineRole) -> Option<&'a str> {
        let (no, line) = *self.lines.get(self.at)?;
        self.roles[no] = role;
        self.at += 1;
        Some(line)
    }

    /// Gives the next line this role, and reads it as a count.
    fn count(&mut self, role: LineRole) -> Option<u32> {
        self.set(role)?.trim().parse().ok()
    }

    /// `count` lines per element.
    fn each(&mut self, count: u32, role: impl Fn(u32) -> LineRole) -> Option<()> {
        for i in 0..count {
            self.set(role(i))?;
        }
        Some(())
    }
}

/// The roles of an anim set. It stops where the text does not fit the layout.
fn walk_set(walk: &mut Walk<'_>) -> Option<()> {
    walk.set(LineRole::Version)?;
    let triggers = walk.count(LineRole::TriggersLen)?;
    walk.each(triggers, LineRole::Trigger)?;
    let conditions = walk.count(LineRole::ConditionsLen)?;
    for i in 0..conditions {
        walk.set(LineRole::ConditionVariable(i))?;
        walk.set(LineRole::ConditionA(i))?;
        walk.set(LineRole::ConditionB(i))?;
    }
    let attacks = walk.count(LineRole::AttacksLen)?;
    for i in 0..attacks {
        walk.set(LineRole::AttackTrigger(i))?;
        walk.set(LineRole::AttackContextual(i))?;
        let clips = walk.count(LineRole::AttackClipsLen(i))?;
        walk.each(clips, |_| LineRole::AttackClip(i))?;
    }
    walk.count(LineRole::AnimInfosLen)?;
    let mut i = 0;
    while !walk.is_done() {
        walk.set(LineRole::AnimDir(i))?;
        walk.set(LineRole::AnimFile(i))?;
        walk.set(LineRole::AnimExtension(i))?;
        i += 1;
    }
    Some(())
}

/// The roles of a clip. All that follows the count is a trigger: a wrong count is `Strict`'s error.
fn walk_clip(walk: &mut Walk<'_>) -> Option<()> {
    walk.set(LineRole::ClipName)?;
    walk.set(LineRole::ClipId)?;
    walk.set(LineRole::Speed)?;
    walk.set(LineRole::CropStart)?;
    walk.set(LineRole::CropEnd)?;
    walk.set(LineRole::ClipTriggersLen)?;
    let mut i = 0;
    while !walk.is_done() {
        walk.set(LineRole::ClipTrigger(i))?;
        i += 1;
    }
    Some(())
}

fn walk_motion(walk: &mut Walk<'_>) -> Option<()> {
    walk.set(LineRole::MotionId)?;
    walk.set(LineRole::Duration)?;
    let translations = walk.count(LineRole::TranslationsLen)?;
    walk.each(translations, LineRole::Translation)?;
    let rotations = walk.count(LineRole::RotationsLen)?;
    walk.each(rotations, LineRole::Rotation)
}

/// The role of every line of an anim set patch or a clip data patch, by the file `id`.
pub(super) fn roles(is_adsf: bool, id: &str, text: &str) -> Vec<LineRole> {
    let mut walk = Walk::new(text);
    let _partial = if is_adsf {
        match AdsfFile::of(id) {
            AdsfFile::AddAnim | AdsfFile::IndexedAnim => walk_clip(&mut walk),
            AdsfFile::AddMotion | AdsfFile::IndexedMotion => walk_motion(&mut walk),
            AdsfFile::ProjectNamesHeader | AdsfFile::AnimHeader => None,
        }
    } else {
        match AsdsfFile::of(id) {
            AsdsfFile::AddSet | AsdsfFile::EditSet => walk_set(&mut walk),
            AsdsfFile::ProjectsHeader | AsdsfFile::SetsHeader => None,
        }
    };
    walk.roles
}

/// What [`problems`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LineProblem {
    /// 0-based line.
    pub(super) line: usize,
    pub(super) message: String,
    pub(super) severity: Severity,
    /// What the line should say. e.g., `3`
    pub(super) fixed: String,
}

/// Is this no name? e.g., `1`, `0.5`, `$crc32[name]$`
fn is_value(line: &str) -> bool {
    let line = line.trim();
    line.parse::<f64>().is_ok() || line.starts_with("$crc32")
}

/// The lines of the layout, which [`problems`] goes through by what they are: where a count is wrong,
/// it cannot go by the counts.
struct Reader<'a> {
    /// `(line number, text)`
    lines: Vec<(usize, &'a str)>,
    at: usize,
    problems: Vec<LineProblem>,
}

impl<'a> Reader<'a> {
    fn new(text: &'a str) -> Self {
        let mut lines = Walk::new(text).lines;
        lines.retain(|(_, line)| !line.trim().is_empty());
        Self { lines, at: 0, problems: Vec::new() }
    }

    fn next(&mut self) -> Option<(usize, &'a str)> {
        let line = *self.lines.get(self.at)?;
        self.at += 1;
        Some(line)
    }

    fn peek(&self) -> Option<&'a str> {
        self.lines.get(self.at).map(|(_, line)| *line)
    }

    /// The names from here on, which are passed. Returns how many.
    fn skip_names(&mut self) -> u32 {
        let mut names = 0;
        while self.peek().is_some_and(|line| !is_value(line)) {
            self.at += 1;
            names += 1;
        }
        names
    }

    /// The lines with several values from here on, which are passed: what is next after them is a count,
    /// one value. Returns how many. How many values is the parser's to check, not a miscount.
    fn skip_keys(&mut self) -> u32 {
        let mut keys = 0;
        while self.peek().is_some_and(|line| line.split_whitespace().nth(1).is_some()) {
            self.at += 1;
            keys += 1;
        }
        keys
    }

    /// The count of anim infos is next: it and then three lines per anim info, all values.
    fn is_at_anim_infos(&self) -> bool {
        let rest = &self.lines[self.at.min(self.lines.len())..];
        rest.len() % 3 == 1 && rest.iter().all(|(_, line)| is_value(line))
    }

    /// `(line, text)` is a count of `what`, and there are `actual`.
    fn check_count(&mut self, (line, text): (usize, &str), what: &str, actual: u32) {
        if text.trim().parse::<u32>().is_ok_and(|declared| declared != actual) {
            self.problems.push(LineProblem {
                line,
                message: format!("The count of {what} is {}, but there are {actual}.", text.trim()),
                severity: Severity::Error,
                fixed: actual.to_string(),
            });
        }
    }

    /// `id` is the clip id in the file's name.
    fn check_clip_id(&mut self, (line, text): (usize, &str), id: &str) {
        if text.trim() != id {
            self.problems.push(LineProblem {
                line,
                message: format!("The clip id is `{}`, but the file's name says `{id}`.", text.trim()),
                severity: Severity::Warning,
                fixed: id.to_owned(),
            });
        }
    }

    /// See [`walk_set`].
    fn read_set(&mut self) -> Option<()> {
        self.next()?; // the version
        let count = self.next()?;
        let triggers = self.skip_names();
        self.check_count(count, "triggers", triggers);

        let count = self.next()?;
        let mut conditions = 0;
        while self.peek().is_some_and(|line| !is_value(line)) {
            self.at += 3;
            conditions += 1;
        }
        self.check_count(count, "conditions", conditions);

        let count = self.next()?;
        let attacks = self.read_attacks()?;
        self.check_count(count, "attacks", attacks);

        let count = self.next()?;
        let anim_infos = u32::try_from(self.lines.len().saturating_sub(self.at) / 3).ok()?;
        self.check_count(count, "anim infos", anim_infos);
        Some(())
    }

    /// Returns how many attacks there are. What ends an attack's clips is not known by the line, as the
    /// next attack starts with a name too: it is the last name before the next attack's values, unless
    /// the anim infos are next.
    fn read_attacks(&mut self) -> Option<u32> {
        let mut attacks = 0;
        while !self.is_at_anim_infos() && self.peek().is_some_and(|line| !is_value(line)) {
            self.next()?; // its trigger
            self.next()?; // is it contextual?
            let count = self.next()?;
            let names = self.skip_names();
            let clips = if names > 0 && self.peek().is_some() && !self.is_at_anim_infos() {
                self.at -= 1;
                names - 1
            } else {
                names
            };
            self.check_count(count, "this attack's clips", clips);
            attacks += 1;
        }
        Some(attacks)
    }

    /// See [`walk_clip`]. `id`: the clip id in the file's name.
    fn read_clip(&mut self, id: &str) -> Option<()> {
        self.next()?; // the name
        let clip_id = self.next()?;
        self.check_clip_id(clip_id, id);
        self.at += 3; // the speed and the crops
        let count = self.next()?;
        let triggers = u32::try_from(self.lines.len().saturating_sub(self.at)).ok()?;
        self.check_count(count, "triggers", triggers);
        Some(())
    }

    /// See [`walk_motion`]. `id`: the clip id in the file's name.
    fn read_motion(&mut self, id: &str) -> Option<()> {
        let clip_id = self.next()?;
        self.check_clip_id(clip_id, id);
        self.next()?; // the duration
        let count = self.next()?;
        let translations = self.skip_keys();
        self.check_count(count, "translations", translations);
        let count = self.next()?;
        let rotations = self.skip_keys();
        self.check_count(count, "rotations", rotations);
        Some(())
    }
}

/// `DefaultMale~1/Idle~mod$0.txt` -> `mod$0`, `DefaultMale~1/mod$0.txt` -> `mod$0`
fn clip_id_of_file(id: &str) -> &str {
    let file = id.rsplit_once('/').map_or(id, |(_, file)| file);
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    stem.rsplit_once('~').map_or(stem, |(_, clip_id)| clip_id)
}

/// The counts that are wrong, and a clip id that is not the file's. See the module docs.
pub(super) fn problems(is_adsf: bool, id: &str, text: &str) -> Vec<LineProblem> {
    let mut reader = Reader::new(text);
    let _partial = if is_adsf {
        match AdsfFile::of(id) {
            _ if is_edit(text) => None,
            AdsfFile::AddAnim | AdsfFile::IndexedAnim => reader.read_clip(clip_id_of_file(id)),
            AdsfFile::AddMotion | AdsfFile::IndexedMotion => reader.read_motion(clip_id_of_file(id)),
            AdsfFile::ProjectNamesHeader | AdsfFile::AnimHeader => None,
        }
    } else {
        match AsdsfFile::of(id) {
            AsdsfFile::AddSet => reader.read_set(),
            AsdsfFile::EditSet | AsdsfFile::ProjectsHeader | AsdsfFile::SetsHeader => None,
        }
    };
    reader.problems
}

/// [`problems`] as editor underlines (the trimmed line).
pub(super) fn diagnostics(is_adsf: bool, id: &str, text: &str) -> Vec<Diagnostic> {
    let lines: Vec<&str> = text.split('\n').collect();
    let columns = |line: usize| {
        let source = lines.get(line).copied().unwrap_or_default();
        let start = source.chars().take_while(|c| c.is_whitespace()).count();
        start..start + source.trim().chars().count()
    };
    problems(is_adsf, id, text)
        .into_iter()
        .map(|problem| Diagnostic {
            line: problem.line,
            columns: columns(problem.line),
            severity: problem.severity,
            message: problem.message,
        })
        .collect()
}

/// The editor's language for these files.
#[derive(Debug)]
pub(super) struct AnimDataLanguage<'a> {
    /// Which of the two templates, and the file: see [`problems`].
    pub(super) is_adsf: bool,
    pub(super) id: &'a str,
    /// `LspOptions::code_action`
    pub(super) code_action: bool,
    /// See [`roles`], per line of the open file.
    pub(super) roles: &'a [LineRole],
    /// The cursor line last frame: [`Language::complete`] only gets the text of a line.
    pub(super) cursor_line: usize,
    /// The known trigger names.
    pub(super) trigger_names: &'a [String],
}

impl Language for AnimDataLanguage<'_> {
    /// The markers are colored like in XML.
    fn highlight(&self, line: &str, visuals: &egui::Visuals) -> Vec<(std::ops::Range<usize>, egui::Color32)> {
        Xml.highlight(line, visuals)
    }

    fn hover_at(&self, _text: &str, line: usize, _column: usize) -> Option<String> {
        self.roles.get(line)?.doc().map(str::to_owned)
    }

    fn inlay_hints_at(&self, line_no: usize, line: &str) -> Vec<String> {
        if line.trim().is_empty() {
            return Vec::new();
        }
        let hint = self.roles.get(line_no).map_or_else(String::new, |role| role.hint());
        if hint.is_empty() { Vec::new() } else { vec![hint] }
    }

    fn completion_triggers(&self) -> &[char] {
        &['$']
    }

    fn quick_fixes(&self, text: &str, line: usize, _column: usize) -> Vec<QuickFix> {
        if !self.code_action {
            return Vec::new();
        }
        let source = text.split('\n').nth(line).unwrap_or_default();
        let start = source.chars().take_while(|c| c.is_whitespace()).count();
        let columns = start..start + source.trim().chars().count();
        problems(self.is_adsf, self.id, text)
            .into_iter()
            .filter(|problem| problem.line == line)
            .map(|problem| QuickFix {
                title: format!("Change it to `{}`", problem.fixed),
                line,
                columns: columns.clone(),
                new_text: problem.fixed,
            })
            .collect()
    }

    fn complete(&self, line: &str, column: usize) -> CompletionList {
        let before: String = line.chars().take(column).collect();
        let whole = |values: &[(&str, &str)]| {
            let typed = before.trim_start();
            let items = values
                .iter()
                .filter(|(value, _)| value.starts_with(typed))
                .map(|(value, doc)| item(value, "◆", (*value).to_owned(), (*doc).to_owned()))
                .collect();
            CompletionList { replace: column - typed.chars().count()..column, items }
        };
        match self.roles.get(self.cursor_line) {
            Some(LineRole::Version) => whole(&[("V3", "The only version.")]),
            Some(LineRole::AttackContextual(_)) => whole(&[("0", "Not contextual"), ("1", "Contextual")]),
            Some(LineRole::AnimExtension(_)) => whole(&[("7891816", "The CRC32 of `hkx`")]),
            Some(LineRole::AnimDir(_)) => whole(&[(
                "$crc32[meshes\\actors\\character\\animations]$",
                "The CRC32 of a directory, computed when merging",
            )]),
            Some(LineRole::AnimFile(_)) => {
                whole(&[("$crc32[name]$", "The CRC32 of a file name without extension, computed when merging")])
            }
            Some(LineRole::ClipTrigger(_) | LineRole::Trigger(_)) => {
                trigger_completion(self.trigger_names, &before, column)
            }
            _ => CompletionList::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_line_of_an_anim_set_has_its_role() {
        let set = "V3\n1\ntrig\n1\nvar\n0\n1\n1\natk\n0\n1\nclip\n1\n1\n2\n3\n4\n5\n6\n";
        let roles = roles(false, "A~A/x.txt", set);
        assert_eq!(roles[0], LineRole::Version);
        assert_eq!(roles[2], LineRole::Trigger(0));
        assert_eq!(roles[6], LineRole::ConditionB(0));
        assert_eq!(roles[11], LineRole::AttackClip(0));
        assert_eq!(roles[12], LineRole::AnimInfosLen);
        assert_eq!(roles[15], LineRole::AnimExtension(0));
        // Past the count: a push does not change it.
        assert_eq!(roles[16], LineRole::AnimDir(1));
        assert_eq!(roles[18], LineRole::AnimExtension(1));
    }

    #[test]
    fn markers_and_the_template_lines_are_not_in_the_layout() {
        let clip = "Idle\n1\n1\n0\n0\n<!-- MOD_CODE ~m~ OPEN -->\n1\n<!-- ORIGINAL -->\n0\n<!-- CLOSE -->\nDraw:0.5\n";
        let roles = roles(true, "D~1/Idle~348.txt", clip);
        assert_eq!(roles[4], LineRole::CropEnd);
        assert_eq!(roles[5], LineRole::Other);
        assert_eq!(roles[6], LineRole::ClipTriggersLen);
        assert_eq!(roles[8], LineRole::Other);
        assert_eq!(roles[10], LineRole::ClipTrigger(0));
    }

    #[test]
    fn a_motion_is_not_a_clip() {
        let roles = roles(true, "D~1/m$0.txt", "m$0\n1\n1\n1 0 0 0\n1\n1 0 0 0 1\n");
        assert_eq!(roles[0], LineRole::MotionId);
        assert_eq!(roles[3], LineRole::Translation(0));
        assert_eq!(roles[5], LineRole::Rotation(0));
    }

    /// `(line, what it should say)`
    fn found(is_adsf: bool, id: &str, text: &str) -> Vec<(usize, String)> {
        problems(is_adsf, id, text).into_iter().map(|problem| (problem.line, problem.fixed)).collect()
    }

    const CLIP: &str = "D~1/Idle~m$0.txt";
    const SET: &str = "A~A/m$New.txt";

    #[test]
    fn a_clip_has_the_triggers_its_count_says() {
        assert!(found(true, CLIP, "Idle\nm$0\n1\n0\n0\n1\nDraw:0.5\n").is_empty());
        // 7 triggers, and none.
        assert_eq!(found(true, CLIP, "Idle\nm$0\n1\n0\n0\n7\n"), [(5, "0".to_owned())]);
        assert_eq!(found(true, CLIP, "Idle\nm$0\n1\n0\n0\n0\nDraw:0.5\nHit:0.6\n"), [(5, "2".to_owned())]);
    }

    #[test]
    fn a_clip_id_is_the_one_of_the_file_name() {
        let problems = problems(true, CLIP, "Idle\nother$3\n1\n0\n0\n0\n");
        assert_eq!(problems.len(), 1);
        assert_eq!((problems[0].line, problems[0].fixed.as_str()), (1, "m$0"));
        assert_eq!(problems[0].severity, Severity::Warning, "it merges, but the motion is not found by it");

        assert_eq!(found(true, "D~1/Idle~348.txt", "Idle\n349\n1\n0\n0\n0\n"), [(1, "348".to_owned())]);
        assert_eq!(found(true, "D~1/m$0.txt", "m$1\n1\n0\n0\n"), [(0, "m$0".to_owned())]);
    }

    #[test]
    fn a_motion_has_the_keys_its_counts_say() {
        let motion = "D~1/m$0.txt";
        assert!(found(true, motion, "m$0\n1\n1\n1 0 0 0\n1\n1 0 0 0 1\n").is_empty());
        assert_eq!(found(true, motion, "m$0\n1\n2\n1 0 0 0\n1\n1 0 0 0 1\n"), [(2, "1".to_owned())]);
        assert_eq!(found(true, motion, "m$0\n1\n1\n1 0 0 0\n0\n1 0 0 0 1\n"), [(4, "1".to_owned())]);
        // Keys that are not what they should be are not a wrong count: real mods have such ones.
        assert!(found(true, motion, "m$0\n1\n1\n1 animmotion 0 0 0\n0\n").is_empty());
    }

    #[test]
    fn an_anim_set_has_what_its_counts_say() {
        let good = "V3\n1\ntrig\n1\nvar\n0\n1\n2\natk\n0\n1\nclip\natk2\n0\n2\nclip1\nclip2\n1\n1\n2\n3\n";
        assert!(found(false, SET, good).is_empty(), "{:?}", found(false, SET, good));

        // The triggers are gone: it stays readable, for the rest to be checked.
        let no_triggers = good.replace("V3\n1\ntrig\n", "V3\n1\n");
        assert_eq!(found(false, SET, &no_triggers), [(1, "0".to_owned())]);

        let conditions = good.replace("1\nvar\n0\n1\n", "3\nvar\n0\n1\n");
        assert_eq!(found(false, SET, &conditions), [(3, "1".to_owned())]);

        let attacks = good.replace("\n2\natk\n", "\n5\natk\n");
        assert_eq!(found(false, SET, &attacks), [(7, "2".to_owned())]);

        // The first attack's clips: its one, then the next attack's trigger.
        let clips = good.replace("atk\n0\n1\nclip\n", "atk\n0\n4\nclip\n");
        assert_eq!(found(false, SET, &clips), [(10, "1".to_owned())]);
        // The last attack's: up to the anim infos.
        let clips = good.replace("atk2\n0\n2\n", "atk2\n0\n9\n");
        assert_eq!(found(false, SET, &clips), [(14, "2".to_owned())]);

        let infos = good.replace("clip2\n1\n1\n2\n3\n", "clip2\n4\n1\n2\n3\n");
        assert_eq!(found(false, SET, &infos), [(17, "1".to_owned())]);
    }

    /// A push does not change the counts in a patch that edits, so that is no mistake there.
    #[test]
    fn an_edit_may_push_without_changing_the_counts() {
        let text = "V3\n0\n0\n0\n1\n1\n2\n3\n4\n5\n6\n";
        assert!(found(false, "A~A/_MTSolo.txt", text).is_empty());
        assert_eq!(found(false, SET, text), [(4, "2".to_owned())]);

        let motion = "346\n1\n1\n<!-- MOD_CODE ~m~ OPEN -->\n1 0 0 0\n2 0 0 0\n<!-- CLOSE -->\n1\n1 0 0 0 1\n";
        assert!(found(true, "D~1/346.txt", motion).is_empty());
    }

    #[test]
    fn a_quick_fix_changes_the_count_to_what_is_there() {
        let text = "Idle\nm$0\n1\n0\n0\n\t7\n";
        let language = AnimDataLanguage {
            is_adsf: true,
            id: CLIP,
            code_action: true,
            roles: &[],
            cursor_line: 0,
            trigger_names: &[],
        };
        let fixes = language.quick_fixes(text, 5, 0);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].new_text, "0");
        assert_eq!(fixes[0].columns, 1..2);
        assert!(language.quick_fixes(text, 4, 0).is_empty(), "another line");

        let off = AnimDataLanguage { code_action: false, ..language };
        assert!(off.quick_fixes(text, 5, 0).is_empty());
    }

    #[test]
    fn what_does_not_fit_has_no_role() {
        assert!(roles(false, "A~A/x.txt", "V3\nnot a count\nx\n")[2..].iter().all(|role| *role == LineRole::Other));
        assert_eq!(roles(false, "A~A/$header$.txt", "1\nNew.txt\n")[0], LineRole::Other);
    }

    #[test]
    fn completes_by_the_role_of_the_cursor_line() {
        let roles = roles(false, "A~A/x.txt", "V3\n0\n0\n0\n1\n1\n2\n");
        let language = AnimDataLanguage {
            is_adsf: false,
            id: "A~A/x.txt",
            code_action: true,
            roles: &roles,
            cursor_line: 7,
            trigger_names: &[],
        };
        let list = language.complete("", 0);
        assert_eq!(list.items[0].insert, "7891816");
        assert_eq!(language.inlay_hints_at(5, "1"), ["animInfos[0].dir"]);
    }
}
