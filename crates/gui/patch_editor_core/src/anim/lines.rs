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
//! # Patches
//! A patch is a free line diff, like git's: a block may replace any lines with any number of lines,
//! and its counts don't have to say how many elements the mod's side has. So, as d-merge's parser
//! does, the layout is read on the template's side (the lines around the blocks and those after
//! `ORIGINAL`), whose counts are right, and the mod's lines get their roles from it:
//! ```text
//! 1                   ClipTriggersLen         template's side: walked
//! <!-- MOD_CODE ~m~ OPEN -->
//! a:0.1               ClipTrigger(0)          k-th line of the block = k-th line of `ORIGINAL`
//! b:0.2               ClipTrigger(1)          past them: what follows the line before, see `LineRole::after`
//! <!-- ORIGINAL -->
//! x:0.5               ClipTrigger(0)          template's side: walked
//! <!-- CLOSE -->
//! ```
//! A block without `ORIGINAL` adds lines: they follow the line before them, e.g. a push after the last
//! trigger is the next trigger. Blank lines of a block are Nemesis' marks of deleted lines.
//!
//! An anim set's anim infos go to the end of the file whatever its count says: a pushing patch does
//! not change the count (see [`nemesis_diff::asdsf::add_anim_infos`]).
//!
//! The format allows too much for these roles to tell what is wrong: they are a guide to reading a
//! file. What is wrong is for d-merge's parser to say, see [`crate::anim::patch_file`].

use std::{borrow::Cow, ops::Range};

use egui_code_editor::{CompletionList, Language, languages::Xml};

use crate::{
    anim::patch_file::{AdsfFile, AsdsfFile},
    editor::language::{item, trigger_completion},
};

/// See the module docs. The number is the element's index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineRole {
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
    fn hint(self) -> Cow<'static, str> {
        match self {
            Self::Version => Cow::Borrowed("version"),
            Self::TriggersLen | Self::ClipTriggersLen => Cow::Borrowed("triggers: count"),
            Self::Trigger(i) | Self::ClipTrigger(i) => Cow::Owned(format!("triggers[{i}]")),
            Self::ConditionsLen => Cow::Borrowed("conditions: count"),
            Self::ConditionVariable(i) => Cow::Owned(format!("conditions[{i}].variable")),
            Self::ConditionA(i) => Cow::Owned(format!("conditions[{i}].a")),
            Self::ConditionB(i) => Cow::Owned(format!("conditions[{i}].b")),
            Self::AttacksLen => Cow::Borrowed("attacks: count"),
            Self::AttackTrigger(i) => Cow::Owned(format!("attacks[{i}].trigger")),
            Self::AttackContextual(i) => Cow::Owned(format!("attacks[{i}].isContextual")),
            Self::AttackClipsLen(i) => Cow::Owned(format!("attacks[{i}].clips: count")),
            Self::AttackClip(i) => Cow::Owned(format!("attacks[{i}].clip")),
            Self::AnimInfosLen => Cow::Borrowed("animInfos: count"),
            Self::AnimDir(i) => Cow::Owned(format!("animInfos[{i}].dir")),
            Self::AnimFile(i) => Cow::Owned(format!("animInfos[{i}].file")),
            Self::AnimExtension(i) => Cow::Owned(format!("animInfos[{i}].extension")),
            Self::ClipName => Cow::Borrowed("clip name"),
            Self::ClipId | Self::MotionId => Cow::Borrowed("clip id"),
            Self::Speed => Cow::Borrowed("speed"),
            Self::CropStart => Cow::Borrowed("crop start"),
            Self::CropEnd => Cow::Borrowed("crop end"),
            Self::Duration => Cow::Borrowed("duration"),
            Self::TranslationsLen => Cow::Borrowed("translations: count"),
            Self::Translation(i) => Cow::Owned(format!("translations[{i}]: time x y z")),
            Self::RotationsLen => Cow::Borrowed("rotations: count"),
            Self::Rotation(i) => Cow::Owned(format!("rotations[{i}]: time x y z w")),
            Self::Other => Cow::Borrowed(""),
        }
    }

    /// The role of a line added after a line of this role: the next element of the list it is in, or
    /// the first element of the list it counts. An added line is far more often an element than the
    /// next field, which a patch has no reason to add.
    ///
    /// e.g., `Trigger(1)` -> `Trigger(2)`, `TriggersLen` -> `Trigger(0)`, `AnimExtension(0)` -> `AnimDir(1)`
    const fn after(self) -> Self {
        match self {
            Self::Version => Self::TriggersLen,
            Self::TriggersLen => Self::Trigger(0),
            Self::Trigger(i) => Self::Trigger(i + 1),
            Self::ConditionsLen => Self::ConditionVariable(0),
            Self::ConditionVariable(i) => Self::ConditionA(i),
            Self::ConditionA(i) => Self::ConditionB(i),
            Self::ConditionB(i) => Self::ConditionVariable(i + 1),
            Self::AttacksLen => Self::AttackTrigger(0),
            Self::AttackTrigger(i) => Self::AttackContextual(i),
            Self::AttackContextual(i) => Self::AttackClipsLen(i),
            Self::AttackClipsLen(i) | Self::AttackClip(i) => Self::AttackClip(i),
            Self::AnimInfosLen => Self::AnimDir(0),
            Self::AnimDir(i) => Self::AnimFile(i),
            Self::AnimFile(i) => Self::AnimExtension(i),
            Self::AnimExtension(i) => Self::AnimDir(i + 1),

            Self::ClipName => Self::ClipId,
            Self::ClipId => Self::Speed,
            Self::Speed => Self::CropStart,
            Self::CropStart => Self::CropEnd,
            Self::CropEnd => Self::ClipTriggersLen,
            Self::ClipTriggersLen => Self::ClipTrigger(0),
            Self::ClipTrigger(i) => Self::ClipTrigger(i + 1),

            Self::MotionId => Self::Duration,
            Self::Duration => Self::TranslationsLen,
            Self::TranslationsLen => Self::Translation(0),
            Self::Translation(i) => Self::Translation(i + 1),
            Self::RotationsLen => Self::Rotation(0),
            Self::Rotation(i) => Self::Rotation(i + 1),

            Self::Other => Self::Other,
        }
    }

    /// What the hover says.
    pub const fn doc(self) -> Option<&'static str> {
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

/// Where a line of a patch is. See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    /// Around the blocks: both the template's and the mod's.
    Common,
    /// From `OPEN` to `ORIGINAL` or `CLOSE`: the mod's.
    Open,
    /// From `ORIGINAL` to `CLOSE`: the template's.
    Original,
    /// `<!-- ... -->`
    Marker,
}

/// One block of a patch, by line numbers.
///
/// ```text
/// <!-- MOD_CODE ~m~ OPEN -->
/// a                   open
/// <!-- ORIGINAL -->
/// x                   original (empty: the block adds `open`)
/// <!-- CLOSE -->
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
struct Block {
    open: Range<usize>,
    original: Range<usize>,
}

/// The part of every line, and the blocks.
fn parts(lines: &[&str]) -> (Vec<Part>, Vec<Block>) {
    let mut parts = Vec::with_capacity(lines.len());
    let mut blocks = Vec::new();
    let mut section = Part::Common;
    for (no, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with("<!--") {
            parts.push(section);
            continue;
        }
        parts.push(Part::Marker);
        if trimmed.starts_with("<!-- MOD_CODE") {
            section = Part::Open;
            blocks.push(Block { open: no + 1..no + 1, original: no + 1..no + 1 });
        } else if trimmed.starts_with("<!-- ORIGINAL") {
            section = Part::Original;
            if let Some(block) = blocks.last_mut() {
                block.open.end = no;
                block.original = no + 1..no + 1;
            }
        } else if trimmed.starts_with("<!-- CLOSE") {
            if let Some(block) = blocks.last_mut() {
                match section {
                    Part::Open => block.open.end = no,
                    Part::Original => block.original.end = no,
                    Part::Common | Part::Marker => {}
                }
            }
            section = Part::Common;
        }
    }
    (parts, blocks)
}

/// Goes through the lines of the template's side, setting their roles.
struct Walk<'a> {
    /// `(line number, text)`
    lines: Vec<(usize, &'a str)>,
    at: usize,
    roles: Vec<LineRole>,
    /// A count was not a number: the rest of the text is not in this layout.
    misfit: bool,
}

impl<'a> Walk<'a> {
    fn new(lines: &[&'a str], parts: &[Part]) -> Self {
        let last = lines.len().saturating_sub(1);
        let mut walked = Vec::new();
        for (no, (line, part)) in lines.iter().zip(parts).enumerate() {
            // Blank lines may separate elements. The last one is where the next line gets typed:
            // it has the role of what comes next, for the completion.
            let is_separator = line.trim().is_empty() && no != last;
            if matches!(part, Part::Common | Part::Original) && !is_separator {
                walked.push((no, *line));
            }
        }
        Self { lines: walked, at: 0, roles: vec![LineRole::Other; lines.len()], misfit: false }
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
        let count = self.set(role)?.trim().parse().ok();
        self.misfit |= count.is_none();
        count
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
///
/// The template's side is walked, then the mod's lines get their roles from it: see the module docs.
pub fn roles(is_adsf: bool, id: &str, text: &str) -> Vec<LineRole> {
    let lines: Vec<&str> = text.split('\n').collect();
    let (parts, blocks) = parts(&lines);
    let (mut roles, fits) = walk_template_side(is_adsf, id, &lines, &parts);
    for block in &blocks {
        set_block_roles(&mut roles, &lines, block, fits);
    }
    // The line being typed at the end, if the walk did not get to it.
    if fits
        && let Some(last) = lines.len().checked_sub(1)
        && lines[last].trim().is_empty()
        && parts[last] == Part::Common
        && roles[last] == LineRole::Other
    {
        roles[last] = role_before(&roles, last).after();
    }
    roles
}

/// The last role before line `at` (`Other` if none).
fn role_before(roles: &[LineRole], at: usize) -> LineRole {
    roles[..at].iter().rev().find(|role| **role != LineRole::Other).copied().unwrap_or(LineRole::Other)
}

/// The roles of the mod's lines of `block`: the k-th has the role of the k-th line it replaces, and
/// those past them follow the line before.
///
/// e.g., `OPEN` `a`, `b`, `ORIGINAL` `x` (`ClipTrigger(0)`) -> `a`: `ClipTrigger(0)`, `b`: `ClipTrigger(1)`
///
/// `fits`: the template's side fits the layout. If not, nothing is guessed: only replacing lines get roles.
fn set_block_roles(roles: &mut [LineRole], lines: &[&str], block: &Block, fits: bool) {
    let mut before = role_before(roles, block.open.start);
    let mut replaced = block.original.clone(); // the lines of `ORIGINAL` not matched yet
    for no in block.open.clone() {
        if lines[no].trim().is_empty() {
            continue; // Nemesis' mark of a deleted line.
        }
        let replacing = replaced.find(|at| roles[*at] != LineRole::Other).map(|at| roles[at]);
        let role = replacing.unwrap_or_else(|| if fits { before.after() } else { LineRole::Other });
        roles[no] = role;
        before = role;
    }
}

/// The roles of the lines of the template's side, and whether they fit the layout. See [`Walk`].
fn walk_template_side(is_adsf: bool, id: &str, lines: &[&str], parts: &[Part]) -> (Vec<LineRole>, bool) {
    let mut walk = Walk::new(lines, parts);
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
    let fits = !walk.misfit && walk.at > 0; // A header has no layout: nothing was walked.
    (walk.roles, fits)
}

/// The editor's language for these files.
#[derive(Debug)]
pub struct AnimDataLanguage<'a> {
    /// See [`roles`], per line of the open file.
    pub roles: &'a [LineRole],
    /// The cursor line last frame: [`Language::complete`] only gets the text of a line.
    pub cursor_line: usize,
    /// The known trigger names.
    pub trigger_names: &'a [String],
}

impl Language for AnimDataLanguage<'_> {
    /// Its lines have no indentation. As the rest of the patch editor.
    fn indent(&self) -> egui_code_editor::Indent {
        egui_code_editor::Indent::Tab(4)
    }

    /// The markers are colored like in XML.
    fn highlight(&self, line: &str, visuals: &egui::Visuals) -> Vec<(std::ops::Range<usize>, egui::Color32)> {
        Xml.highlight(line, visuals)
    }

    fn hover_at(&self, _text: &str, line: usize, _column: usize) -> Option<String> {
        self.roles.get(line)?.doc().map(str::to_owned)
    }

    fn inlay_hints_at(&self, line_no: usize, line: &str) -> Vec<Cow<'_, str>> {
        if line.trim().is_empty() {
            return Vec::new();
        }
        let hint = self.roles.get(line_no).map_or(Cow::Borrowed(""), |role| role.hint());
        if hint.is_empty() { Vec::new() } else { vec![hint] }
    }

    fn completion_triggers(&self) -> &[char] {
        &['$']
    }

    fn complete(&self, line: &str, column: usize) -> CompletionList {
        let before: String = line.chars().take(column).collect();
        let whole = |values: &[(&'static str, &'static str)]| {
            let typed = before.trim_start();
            let items = values
                .iter()
                .filter(|(value, _)| value.starts_with(typed))
                .map(|&(value, doc)| item(value, "◆", value, doc))
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
    fn a_replacing_line_has_the_role_of_the_line_it_replaces() {
        let clip = "Idle\n1\n1\n0\n0\n<!-- MOD_CODE ~m~ OPEN -->\n1\n<!-- ORIGINAL -->\n0\n<!-- CLOSE -->\nDraw:0.5\n";
        let roles = roles(true, "D~1/Idle~348.txt", clip);
        assert_eq!(roles[4], LineRole::CropEnd);
        assert_eq!(roles[5], LineRole::Other); // marker
        assert_eq!(roles[6], LineRole::ClipTriggersLen);
        assert_eq!(roles[8], LineRole::ClipTriggersLen); // the template's line it replaces
        assert_eq!(roles[10], LineRole::ClipTrigger(0));
    }

    /// The mod's count says 2, but there is one trigger: the template's count is what the layout is read
    /// by, as d-merge's parser does.
    #[test]
    fn the_counts_are_the_templates() {
        let lines = [
            "V3",
            "<!-- MOD_CODE ~m~ OPEN -->",
            "2", // 2
            "<!-- ORIGINAL -->",
            "1", // 4
            "<!-- CLOSE -->",
            "trig", // 6
            "0",    // 7
            "0",    // 8
            "0",    // 9
        ];
        let roles = roles(false, "A~A/x.txt", &lines.join("\n"));
        assert_eq!(roles[2], LineRole::TriggersLen);
        assert_eq!(roles[4], LineRole::TriggersLen);
        assert_eq!(roles[6], LineRole::Trigger(0));
        assert_eq!(roles[7], LineRole::ConditionsLen);
        assert_eq!(roles[8], LineRole::AttacksLen);
        assert_eq!(roles[9], LineRole::AnimInfosLen);
    }

    #[test]
    fn added_lines_follow_the_line_before_them() {
        let lines = [
            "Idle",
            "1",
            "1",
            "0",
            "0",
            "1",
            "a:0.1", // 6: ClipTrigger(0)
            "<!-- MOD_CODE ~m~ OPEN -->",
            "b:0.2", // 8: a push
            "c:0.3", // 9
            "<!-- CLOSE -->",
        ];
        let roles = roles(true, "D~1/Idle~348.txt", &lines.join("\n"));
        assert_eq!(roles[6], LineRole::ClipTrigger(0));
        assert_eq!(roles[8], LineRole::ClipTrigger(1));
        assert_eq!(roles[9], LineRole::ClipTrigger(2));
    }

    /// More lines than it replaces: those past them follow; blank ones mark deleted lines.
    #[test]
    fn a_block_may_have_any_number_of_lines() {
        let lines = [
            "m$0",
            "1",
            "1",
            "<!-- MOD_CODE ~m~ OPEN -->",
            "0.5 0 0 0", // 4: Translation(0)
            "1 0 0 0",   // 5: Translation(1)
            "<!-- ORIGINAL -->",
            "1 0 0 0", // 7
            "<!-- CLOSE -->",
            "1",
            "<!-- MOD_CODE ~m~ OPEN -->",
            "", // 11: deleted
            "<!-- ORIGINAL -->",
            "1 0 0 0 1", // 13
            "<!-- CLOSE -->",
        ];
        let roles = roles(true, "D~1/m$0.txt", &lines.join("\n"));
        assert_eq!(roles[4], LineRole::Translation(0));
        assert_eq!(roles[5], LineRole::Translation(1));
        assert_eq!(roles[7], LineRole::Translation(0));
        assert_eq!(roles[9], LineRole::RotationsLen);
        assert_eq!(roles[11], LineRole::Other);
        assert_eq!(roles[13], LineRole::Rotation(0));
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
