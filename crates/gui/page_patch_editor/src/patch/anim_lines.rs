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

use egui_code_editor::{CompletionList, Language, languages::Xml};

use super::{
    anim_patch::{AdsfFile, AsdsfFile},
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

/// The editor's language for these files.
#[derive(Debug)]
pub(super) struct AnimDataLanguage<'a> {
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

    #[test]
    fn what_does_not_fit_has_no_role() {
        assert!(roles(false, "A~A/x.txt", "V3\nnot a count\nx\n")[2..].iter().all(|role| *role == LineRole::Other));
        assert_eq!(roles(false, "A~A/$header$.txt", "1\nNew.txt\n")[0], LineRole::Other);
    }

    #[test]
    fn completes_by_the_role_of_the_cursor_line() {
        let roles = roles(false, "A~A/x.txt", "V3\n0\n0\n0\n1\n1\n2\n");
        let language = AnimDataLanguage { roles: &roles, cursor_line: 7, trigger_names: &[] };
        let list = language.complete("", 0);
        assert_eq!(list.items[0].insert, "7891816");
        assert_eq!(language.inlay_hints_at(5, "1"), ["animInfos[0].dir"]);
    }
}
