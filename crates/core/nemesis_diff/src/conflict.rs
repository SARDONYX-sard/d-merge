//! Moving one change of a mod's patch into another patch, to resolve a conflict.
//!
//! Which mods conflict is what the merge knows (`page_patch`'s `conflicts`). When the user picks the
//! value to keep, only that change should go to the user patch: the rest of the object may be changed
//! by other mods, and an array push repeated in the user patch would add its elements twice. A patch
//! file has its changes in blocks:
//! ```text
//! <hkparam name="a">1</hkparam>
//! <!-- MOD_CODE ~mymod~ OPEN -->
//! <hkparam name="b">9</hkparam>            what the mod wants
//! <!-- ORIGINAL -->
//! <hkparam name="b">2</hkparam>            what it replaces
//! <!-- CLOSE -->
//! ```
//! The change is known by its JSON path, e.g. `#0106/hkbModifierList/b`. Which block makes it is
//! asked of the parser, as only it knows nested objects and array positions:
//! [`block_of_change`]. Then [`apply_block`] puts that block into the object being edited.

use std::ops::Range;

use nemesis_xml::patch::parse_nemesis_patch;

/// A block of a patch that replaces lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replaced {
    /// 0-based lines of what it patches, as the patch counts them.
    pub lines: Range<u32>,
    /// Those lines, as the patch says they were.
    pub original: Box<str>,
    /// What replaces them. Empty: they are removed.
    pub text: Box<str>,
}

/// The blocks of `patch` that replace lines, in order. `None`: it has no `MOD_CODE` blocks, so the
/// file is the thing itself, e.g. a new object. What only adds lines is not one of them.
pub fn replacing_blocks(patch: &str) -> Option<Vec<Replaced>> {
    enum In {
        Target,
        New,
        Original,
    }
    if !patch.contains("<!-- MOD_CODE") {
        return None;
    }
    let mut blocks = Vec::new();
    let (mut state, mut line) = (In::Target, 0_u32);
    let (mut text, mut original, mut start) = (String::new(), String::new(), 0_u32);
    for source in patch.lines() {
        let trimmed = source.trim();
        if trimmed.starts_with("<!-- MOD_CODE") {
            state = In::New;
            text.clear();
            original.clear();
            start = line;
        } else if trimmed.starts_with("<!-- ORIGINAL") {
            state = In::Original;
        } else if trimmed.starts_with("<!-- CLOSE") {
            if line > start {
                blocks.push(Replaced {
                    lines: start..line,
                    original: original.as_str().into(),
                    text: text.as_str().into(),
                });
            }
            state = In::Target;
        } else {
            match state {
                In::Target => line += 1,
                In::New => push_line(&mut text, source),
                In::Original => {
                    push_line(&mut original, source);
                    line += 1;
                }
            }
        }
    }
    Some(blocks)
}

fn push_line(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}

/// `#0106/hkbX/items/[0:2]` -> (`#0106/hkbX/items`, `0..2`). `None`: not an array range.
fn split_range(path: &str) -> Option<(&str, Range<usize>)> {
    let (base, range) = path.rsplit_once("/[")?;
    let (start, end) = range.strip_suffix(']')?.split_once(':')?;
    Some((base, start.parse().ok()?..end.parse().ok()?))
}

/// Does `patches`, which one block makes, make the change at `path`?
fn makes_change(patches: &nemesis_xml::patch::PatchesMap<'_>, path: &str) -> bool {
    let Some((base, range)) = split_range(path) else {
        return patches.keys().any(|key| key.join("/") == path);
    };
    patches.iter().any(|(key, patch)| {
        let same_array = key.join("/") == base;
        same_array
            && matches!(&patch.action, json_patch::Action::Seq { range: own, .. } if own.start < range.end && range.start < own.end)
    })
}

/// The block of `patch` that makes the change at `path`, asking the parser about each block alone.
///
/// `path`: e.g. `#0106/hkbModifierList/enable`, or for a part of an array `#0106/hkbX/items/[0:2]`.
pub fn block_of_change(patch: &str, path: &str) -> Option<Replaced> {
    let blocks = replacing_blocks(patch)?;
    let hacks = Some(nemesis_xml::hack::HackOptions::enable_all());
    (0..blocks.len()).find_map(|at| {
        let only_this = with_only_block(patch, at);
        let (patches, _) = parse_nemesis_patch(&only_this, hacks).ok()?;
        makes_change(&patches, path).then(|| blocks[at].clone())
    })
}

/// `patch` with its replacing block number `keep` only: the others are undone.
fn with_only_block(patch: &str, keep: usize) -> String {
    let mut out = String::with_capacity(patch.len());
    let lines: Vec<&str> = patch.lines().collect();
    let (mut at, mut replacing) = (0, 0);
    while at < lines.len() {
        if !lines[at].trim().starts_with("<!-- MOD_CODE") {
            push_line(&mut out, lines[at]);
            at += 1;
            continue;
        }
        let close =
            lines[at..].iter().position(|l| l.trim().starts_with("<!-- CLOSE")).map_or(lines.len() - 1, |i| at + i);
        let block = &lines[at..=close];
        let original = block.iter().position(|l| l.trim().starts_with("<!-- ORIGINAL"));
        match original {
            Some(_) if replacing == keep => block.iter().for_each(|line| push_line(&mut out, line)),
            Some(original) => block[original + 1..block.len() - 1].iter().for_each(|line| push_line(&mut out, line)),
            None => {} // only adds: undoing it is leaving it out
        }
        replacing += usize::from(original.is_some());
        at = close + 1;
    }
    out
}

/// `line` for comparing: a field by its name, as its value can be written in many ways (`A|B`,
/// `B|A`); anything else as it is.
///
/// e.g. `\t<hkparam name="flags">A|B</hkparam>` -> `<hkparam name="flags">`
fn line_key(line: &str) -> &str {
    let line = line.trim();
    if line.starts_with("<hkparam")
        && let Some(end) = line.find('>')
    {
        return &line[..=end];
    }
    line
}

/// `text` (a whole object) with `block` applied, and the 0-based line it went to. It is found by the
/// lines it replaces, as near as possible to where they were: other changes may have moved them.
///
/// Both are compared [`crate::normalize`]d: a template may have the pointers of an array on one line,
/// where a mod has one per line. e.g. `#0338 #0340` holds what `#0338` replaces.
///
/// `None`: those lines are not there. e.g., a mod made for another version of the template.
pub fn apply_block(text: &str, block: &Replaced) -> Option<(String, usize)> {
    let text = crate::normalize(text);
    let original = crate::normalize(&block.original);
    let lines: Vec<&str> = text.split('\n').collect();
    let original: Vec<&str> = original.lines().collect();
    let fits = |at: usize| {
        lines
            .get(at..at + original.len())
            .is_some_and(|part| part.iter().map(|l| line_key(l)).eq(original.iter().map(|l| line_key(l))))
    };
    let near = block.lines.start as usize;
    let at = (near..=lines.len()).chain((0..near).rev()).find(|&at| fits(at))?;

    let mut out = lines[..at].join("\n");
    if at > 0 {
        out.push('\n');
    }
    out.push_str(&block.text);
    out.push_str(&lines[at + original.len()..].join("\n"));
    Some((out, at))
}

/// [`apply_block`] on an object that may already be changed there: the value picked before is what
/// the block's lines became, so it gives way to this one. The object's other changes stay.
///
/// e.g. `#0338` was replaced by `#flinch$2`, and now `#zcbe$107` is picked for that place.
pub fn apply_block_over(template: &str, text: &str, block: &Replaced) -> Option<(String, usize)> {
    apply_block(text, block).or_else(|| {
        (0..crate::hunks(template, text).len())
            .find_map(|index| apply_block(&crate::revert_hunk(template, text, index), block))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPEN: &str = "\t\t\t<!-- MOD_CODE ~m~ OPEN -->\n";
    const ORIGINAL: &str = "\t\t\t<!-- ORIGINAL -->\n";
    const CLOSE: &str = "\t\t\t<!-- CLOSE -->\n";

    fn object(fields: &str) -> String {
        format!(
            "\t\t<hkobject name=\"#0106\" class=\"hkbModifierList\" signature=\"0xa4180ca1\">\n{fields}\t\t</hkobject>\n"
        )
    }

    #[test]
    fn reads_the_blocks_that_replace_lines() {
        let two = format!(
            "{OPEN}a\nb\nc\n{ORIGINAL}t0\n{CLOSE}t1\n{OPEN}d\n{ORIGINAL}t2\nt3\n{CLOSE}{OPEN}only added\n{CLOSE}"
        );
        let blocks = replacing_blocks(&two).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].lines, 0..1);
        assert_eq!(&*blocks[0].text, "a\nb\nc\n");
        assert_eq!(blocks[1].lines, 2..4);
        assert_eq!(&*blocks[1].original, "t2\nt3\n");
        assert_eq!(replacing_blocks("<hkobject/>"), None);
    }

    #[test]
    fn the_parser_tells_which_block_makes_a_change() {
        let two = object(&format!(
            "{OPEN}\t\t\t<hkparam name=\"userData\">1</hkparam>\n{ORIGINAL}\t\t\t<hkparam name=\"userData\">0</hkparam>\n{CLOSE}{OPEN}\t\t\t<hkparam name=\"enable\">false</hkparam>\n{ORIGINAL}\t\t\t<hkparam name=\"enable\">true</hkparam>\n{CLOSE}"
        ));
        let block = block_of_change(&two, "#0106/hkbModifierList/enable").unwrap();
        assert!(block.text.contains("false"));
        assert!(!block.text.contains("userData"));
        assert_eq!(block_of_change(&two, "#0106/hkbModifierList/other"), None);
    }

    #[test]
    fn an_array_change_is_found_by_its_range() {
        assert_eq!(split_range("#1/C/items/[0:2]"), Some(("#1/C/items", 0..2)));
        assert_eq!(split_range("#1/C/enable"), None);

        let replacing = object(&format!(
            "\t\t\t<hkparam name=\"modifiers\" numelements=\"2\">\n{OPEN}\t\t\t\t#0108\n{ORIGINAL}\t\t\t\t#0107\n{CLOSE}\t\t\t\t#0109\n\t\t\t</hkparam>\n"
        ));
        assert!(block_of_change(&replacing, "#0106/hkbModifierList/modifiers/[0:1]").is_some());
        assert!(block_of_change(&replacing, "#0106/hkbModifierList/modifiers/[1:2]").is_none());
    }

    #[test]
    fn applies_a_block_where_its_field_is_now() {
        let block = Replaced {
            lines: 1..2,
            original: "\t<hkparam name=\"f\">A|B</hkparam>\n".into(),
            text: "\t<hkparam name=\"f\">C</hkparam>\n".into(),
        };
        // The flags are written in another order, and other lines moved it.
        let text = "a\nb\nc\n\t<hkparam name=\"f\">B|A</hkparam>\nd\n";
        let (applied, line) = apply_block(text, &block).unwrap();
        assert_eq!(applied, "a\nb\nc\n\t<hkparam name=\"f\">C</hkparam>\nd\n");
        assert_eq!(line, 3);
        assert_eq!(apply_block("gone\n", &block), None);

        let first = Replaced { lines: 0..1, original: "t0\n".into(), text: "n\n".into() };
        assert_eq!(apply_block("t0\nt1\n", &first), Some(("n\nt1\n".to_owned(), 0)));
    }

    /// As in `0_master`'s `#2489`: the template has `#0338 #0340` on one line, the mods one per line.
    #[test]
    fn takes_in_values_for_pointers_on_one_line_one_after_the_other() {
        const PATH: &str = "#0106/hkbModifierList/modifiers/[0:1]";
        let template =
            object("\t\t\t<hkparam name=\"modifiers\" numelements=\"2\">\n\t\t\t\t#0338 #0340\n\t\t\t</hkparam>\n");
        let patch = |value: &str| {
            object(&format!(
                "\t\t\t<hkparam name=\"modifiers\" numelements=\"2\">\n{OPEN}\t\t\t\t{value}\n{ORIGINAL}\t\t\t\t#0338\n{CLOSE}\t\t\t\t#0340\n\t\t\t</hkparam>\n"
            ))
        };
        let wanted = |value: &str| {
            object(&format!(
                "\t\t\t<hkparam name=\"modifiers\" numelements=\"2\">\n\t\t\t\t{value}\n\t\t\t\t#0340\n\t\t\t</hkparam>\n"
            ))
        };
        let first = block_of_change(&patch("#flinch$2"), PATH).unwrap();
        let (taken, line) = apply_block_over(&template, &template, &first).unwrap();
        assert_eq!(taken, wanted("#flinch$2"));
        assert_eq!(line, 2);

        // Another mod's value for the same place, then the first one again.
        let second = block_of_change(&patch("#zcbe$107"), PATH).unwrap();
        let (taken, _) = apply_block_over(&template, &taken, &second).unwrap();
        assert_eq!(taken, wanted("#zcbe$107"));
        let (taken, _) = apply_block_over(&template, &taken, &first).unwrap();
        assert_eq!(taken, wanted("#flinch$2"));

        // What it writes changes that element only.
        let written = crate::object_patch("upatch", &template, &taken).unwrap();
        assert!(written.contains("#flinch$2\n"), "{written}");
        assert!(written.contains("<!-- ORIGINAL -->\n\t\t\t\t#0338\n"), "{written}");
    }

    #[test]
    fn another_change_of_the_object_stays_when_a_place_is_picked_again() {
        let template = object("\t\t\t<hkparam name=\"a\">0</hkparam>\n\t\t\t<hkparam name=\"b\">0</hkparam>\n");
        let edited = object("\t\t\t<hkparam name=\"a\">1</hkparam>\n\t\t\t<hkparam name=\"b\">2</hkparam>\n");
        // Blocks next to nothing else: one hunk per field needs a line between them.
        let template = template
            .replace("\t\t\t<hkparam name=\"b\">", "\t\t\t<hkparam name=\"c\">9</hkparam>\n\t\t\t<hkparam name=\"b\">");
        let edited = edited
            .replace("\t\t\t<hkparam name=\"b\">", "\t\t\t<hkparam name=\"c\">9</hkparam>\n\t\t\t<hkparam name=\"b\">");
        let block = Replaced {
            lines: 3..4,
            original: "\t\t\t<hkparam name=\"b\">0</hkparam>\n".into(),
            text: "\t\t\t<hkparam name=\"b\">7</hkparam>\n".into(),
        };
        // `line_key` goes by the field, so a value picked for a field fits at once.
        let (taken, _) = apply_block_over(&template, &edited, &block).unwrap();
        assert!(taken.contains("name=\"a\">1<"), "{taken}");
        assert!(taken.contains("name=\"b\">7<"), "{taken}");
    }
}
