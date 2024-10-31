mod helper;
mod tag;

use helper::{comment_kind, pointer};
use serde_hkx::{
    errors::readable::ReadableError,
    xml::de::parser::{
        comment,
        type_kind::{boolean, string},
    },
};
use simd_json::{borrowed::Object, BorrowedValue, StaticNode};
use std::mem;
use tag::{class_start_tag, end_tag, field_start_tag, start_tag};
use winnow::{
    combinator::{alt, opt},
    PResult, Parser,
};

/// # Errors
/// Parse failed.
pub fn patch_value<'a>(input: &mut &'a str) -> Result<Vec<PatchJson<'a>>, ReadableError> {
    let mut patcher_info = PatcherInfo::new(input);
    patcher_info.parse_class().map_err(|e| {
        ReadableError::from_context(e, &input, input.len() - patcher_info.input.len())
    })?;
    Ok(patcher_info.output_patches)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Add,
    Remove,
    Replace,
    // Move,
    // Copy,
    // Test,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatchJson<'a> {
    op: Op,
    /// $(root), index, className, fieldName
    /// - e.g. "$.4514.hkbStateMachineStateInfo.generator",
    path: Vec<&'a str>,
    /// patch target json value
    value: BorrowedValue<'a>,
}

type Patches<'a> = Vec<CurrentPatchJson<'a>>;

#[derive(Debug, Clone, Default, PartialEq)]
struct TempState<'xml> {
    mode_code: Option<&'xml str>,
    is_passed_original: bool,
    patches: Patches<'xml>,
}

impl<'de> TempState<'de> {
    const fn new() -> Self {
        Self {
            mode_code: None,
            patches: Vec::new(),
            is_passed_original: false,
        }
    }

    /// The following is an additional element, so push.
    /// - `<!-- MOD_CODE ~<id>~ --!>` after it is found.
    /// - `<!-- ORIGINAL --!> is not found yet.
    fn conditional_push(&mut self, path: Vec<&'de str>, value: BorrowedValue<'de>) {
        if self.mode_code.is_some() && !self.is_passed_original {
            self.patches.push(CurrentPatchJson { path, value });
        }
    }

    const fn judge_operation(&self) -> Op {
        match self.mode_code.is_some() {
            true => {
                if self.is_passed_original {
                    Op::Replace
                } else {
                    Op::Add
                }
            }
            false => Op::Remove,
        }
    }

    fn clear_flags(&mut self) {
        self.mode_code = None;
        self.is_passed_original = false;
    }
}

/// The reason this is necessary is because
/// `<!-- ORIGINAL -->` or `<! -- CLOSE -->` is read, the operation type cannot be determined.
#[derive(Debug, Clone, PartialEq)]
struct CurrentPatchJson<'a> {
    /// $(root), index, className, fieldName
    /// - e.g. "$.4514.hkbStateMachineStateInfo.generator",
    path: Vec<&'a str>,
    /// patch target json value
    value: BorrowedValue<'a>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct PatcherInfo<'a> {
    input: &'a str,
    /// Indicates the current json position during one file.
    current_path: Vec<&'a str>,

    /// - `<! -- CLOSE --! >`(XML) where it is temporarily stored because the operation type is unknown until a comment is found.
    /// - `<! -- CLOSE --! >` is found, have it added to `output_patches`.
    current: TempState<'a>,

    output_patches: Vec<PatchJson<'a>>,
}

impl<'de> PatcherInfo<'de> {
    const fn new(input: &'de str) -> Self {
        Self {
            input,
            current_path: Vec::new(),
            current: TempState::new(),
            output_patches: Vec::new(),
        }
    }

    /// # Errors
    /// Parse failed.
    fn parse_class(&mut self) -> PResult<()> {
        let (ptr_index, class_name) = class_start_tag.parse_next(&mut self.input)?;
        self.current_path.push("$");
        self.current_path.push(ptr_index);
        self.current_path.push(class_name);

        // TODO: type table loop counter
        while opt(end_tag("hkobject"))
            .parse_next(&mut self.input)?
            .is_none()
        {
            self.field()?;
        }
        Ok(())
    }

    /// # Errors
    /// Parse failed.
    fn field(&mut self) -> PResult<()> {
        self.comment()?;

        let input = &mut self.input;
        let (field_name, array_len) = field_start_tag.parse_next(input)?;
        self.current_path.push(field_name);

        let value = if array_len.is_some() {
            BorrowedValue::Array(Box::new(Vec::new()))
        } else {
            // TODO: parse with type table
            alt((
                "\u{2400}".value(BorrowedValue::Static(StaticNode::Null)), // StringPtr | CString
                boolean().map(|boolean| BorrowedValue::Static(StaticNode::Bool(boolean))),
                pointer,
                Self::class_in_field,                // Start with `hkobject`
                string().map(BorrowedValue::String), // StringPtr | CString
            ))
            .parse_next(input)?
        };

        self.current
            .conditional_push(self.current_path.clone(), value);

        end_tag("hkparam").parse_next(input)?;
        self.current_path.pop(); // remove field name

        self.comment()?;

        Ok(())
    }

    fn comment(&mut self) -> PResult<()> {
        let input = &mut self.input;

        if let Some(mut xml_comment) = opt(comment()).parse_next(input)? {
            let comment_ty = comment_kind.parse_next(&mut xml_comment)?;
            match comment_ty {
                helper::CommentKind::ModCode(id) => self.current.mode_code = Some(id),
                helper::CommentKind::Original => self.current.is_passed_original = true,
                helper::CommentKind::Close => self.add_patches_json(),
            }
        };

        Ok(())
    }

    /// # Errors
    /// Parse failed.
    fn class_in_field<'a>(input: &mut &'a str) -> PResult<BorrowedValue<'a>> {
        start_tag("hkobject").parse_next(input)?;

        end_tag("hkobject").parse_next(input)?;
        Ok(BorrowedValue::Object(Box::new(Object::new())))
    }

    fn add_patches_json(&mut self) {
        // take & reset current states
        let op = self.current.judge_operation();
        let patches = mem::take(&mut self.current.patches);
        self.current.clear_flags();

        for value in patches {
            self.output_patches.push(PatchJson {
                op,
                path: value.path.clone(),
                value: value.value,
            });
        }
    }
}

#[test]
fn parse() {
    let mut nemesis_xml = include_str!("../../../../dummy/mods/turn/1hm_behavior/#4514.txt");
    dbg!(patch_value(&mut nemesis_xml).unwrap_or_else(|e| panic!("{e}")));
}
