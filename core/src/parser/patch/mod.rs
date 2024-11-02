mod class_table;
mod helper;
mod tag;

use class_table::{find_class_info, find_json_parser_by, FieldInfo};
use helper::{comment_kind, pointer};
use serde_hkx::{
    errors::readable::ReadableError,
    xml::de::parser::{
        comment,
        type_kind::{boolean, real, string, vector4},
    },
};
use simd_json::{borrowed::Object, BorrowedValue, StaticNode};
use std::mem;
use tag::{class_start_tag, end_tag, field_start_tag, start_tag};
use winnow::{
    ascii::{dec_int, dec_uint, multispace0},
    combinator::{alt, opt},
    token::take_until,
    PResult, Parser,
};

/// # Errors
/// Parse failed.
pub fn patch_value(input: &str) -> Result<Vec<PatchJson<'_>>, ReadableError> {
    let mut patcher_info = PatcherInfo::new(input);
    patcher_info.root_class().map_err(|err| {
        let err_pos = input.len() - patcher_info.input.len();
        ReadableError::from_context(err, input, err_pos)
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

#[derive(Debug, Clone, Default)]
struct TempState<'xml> {
    field_info: Option<&'static FieldInfo>,
    mode_code: Option<&'xml str>,
    is_passed_original: bool,
    patches: Patches<'xml>,
}

impl<'de> TempState<'de> {
    const fn new() -> Self {
        Self {
            field_info: None,
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

#[derive(Debug, Clone, Default)]
struct PatcherInfo<'a> {
    /// mutable pointer to str
    input: &'a str,
    /// Indicates the current json position during one patch file.
    current_path: Vec<&'a str>,

    /// - `<! -- CLOSE --! >`(XML) where it is temporarily stored because the operation type is unknown until a comment is found.
    /// - `<! -- CLOSE --! >` is found, have it added to `output_patches`.
    current: TempState<'a>,

    /// N time nested classes fields.
    field_infos: Vec<&'static FieldInfo>,

    output_patches: Vec<PatchJson<'a>>,
}

impl<'de> PatcherInfo<'de> {
    const fn new(input: &'de str) -> Self {
        Self {
            input,
            current_path: Vec::new(),
            current: TempState::new(),
            field_infos: Vec::new(),
            output_patches: Vec::new(),
        }
    }

    fn push_field_info(&mut self, info: &'static FieldInfo) {
        self.field_infos.push(info);
        self.current.field_info = Some(info);
    }

    fn pop_field_info(&mut self) {
        self.field_infos.pop();
        self.current.field_info = self.field_infos.last().map(|v| &**v);
    }

    fn root_class(&mut self) -> PResult<()> {
        let (ptr_index, class_name) = class_start_tag.parse_next(&mut self.input)?;
        self.current_path.push("$");
        self.current_path.push(ptr_index);
        self.current_path.push(class_name);

        self.push_field_info(
            find_class_info(class_name).unwrap_or_else(|| panic!("Not found {class_name} class")),
        );

        while opt(end_tag("hkobject"))
            .parse_next(&mut self.input)?
            .is_none()
        {
            self.field()?;
        }

        self.pop_field_info();
        Ok(())
    }

    /// # Errors
    /// Parse failed.
    fn field(&mut self) -> PResult<()> {
        self.opt_comment()?;

        let (field_name, _array_len) = field_start_tag.parse_next(&mut self.input)?;
        self.current_path.push(field_name);

        let value = if let Some(field_info) = self.current.field_info {
            if let Some(json_type) = find_json_parser_by(field_name, field_info) {
                self.parse_value(json_type)?
            } else {
                panic!("No field info: {field_name}. expected: {field_info:?}");
            }
        } else {
            panic!("Failed to get field_info");
        };

        self.current
            .conditional_push(self.current_path.clone(), value); // FIXME: Wrong
        end_tag("hkparam").parse_next(&mut self.input)?;
        self.current_path.pop(); // The class is at the bottom of the hierarchy when we exit field
        self.opt_comment()?;

        Ok(())
    }

    /// - Bool
    /// - I64
    /// - U64
    /// - F64
    /// - Null
    /// - Pointer
    /// - String
    /// - Object|<ClassName>
    /// - Array|<TypeName>
    /// - Array|Object|<ClassName>
    fn parse_value(&mut self, json_parser_type: &'static str) -> PResult<BorrowedValue<'de>> {
        self.opt_comment()?;
        multispace0.parse_next(&mut self.input)?;

        let value = match json_parser_type {
            "Null" => BorrowedValue::Static(StaticNode::Null),
            "I64" => dec_int
                .map(|int| BorrowedValue::Static(StaticNode::I64(int)))
                .parse_next(&mut self.input)?,
            "U64" => dec_uint
                .map(|uint| BorrowedValue::Static(StaticNode::U64(uint)))
                .parse_next(&mut self.input)?,
            "F64" => real()
                .map(|real| BorrowedValue::Static(StaticNode::F64(real.into())))
                .parse_next(&mut self.input)?,
            "String" => {
                alt((
                    "\u{2400}".value(BorrowedValue::Static(StaticNode::Null)), // StringPtr | CString
                    string().map(BorrowedValue::String),
                ))
                .parse_next(&mut self.input)?
            } // StringPtr | CString
            "Pointer" => pointer.parse_next(&mut self.input)?,
            "Bool" => boolean()
                .map(|boolean| BorrowedValue::Static(StaticNode::Bool(boolean)))
                .parse_next(&mut self.input)?,
            obj if obj.starts_with("Object|") => {
                let class_name = &obj[7..]; // Remove "object|"

                if class_name.starts_with("Vector4") {
                    vector4()
                        .map(|v| {
                            let mut obj = Object::new();
                            obj.insert("x".into(), v.x.into());
                            obj.insert("y".into(), v.y.into());
                            obj.insert("z".into(), v.z.into());
                            obj.insert("w".into(), v.w.into());
                            BorrowedValue::Object(Box::new(obj))
                        })
                        .parse_next(&mut self.input)?
                } else {
                    self.class_in_field(class_name)? // Start with `<hkobject>`
                }
            }
            arr if arr.starts_with("Array|") => {
                let name = &arr[6..]; // Remove "array|"
                let mut vec = vec![];

                if name.starts_with("String") {
                    // TODO: Array target (e.g. `Array|Vector4`)
                    while {
                        let (_remain, res) = opt(end_tag("hkparam")).parse_peek(self.input)?;
                        res.is_none()
                    } {
                        self.opt_comment()?;
                        start_tag("hkcstring").parse_next(&mut self.input)?;
                        let value = self.parse_value(name)?;
                        end_tag("hkcstring").parse_next(&mut self.input)?;
                        self.opt_comment()?;
                        vec.push(value);
                    }
                } else if !name.starts_with("Null") {
                    // TODO: Array target (e.g. `Array|Vector4`)
                    while {
                        let (_remain, res) = opt(end_tag("hkparam")).parse_peek(self.input)?;
                        res.is_none()
                    } {
                        let value = self.parse_value(name)?;
                        vec.push(value);
                    }
                };

                BorrowedValue::Array(Box::new(vec)) // `Null`(void)
            }
            unknown => panic!("Unknown type: {unknown}"),
        };

        multispace0.parse_next(&mut self.input)?;
        self.opt_comment()?;

        Ok(value)
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    /// # Errors
    /// Parse failed.
    fn class_in_field(&mut self, class_name: &str) -> PResult<BorrowedValue<'de>> {
        start_tag("hkobject").parse_next(&mut self.input)?;

        // dbg!(&self);

        dbg!("field in class");
        self.push_field_info(
            find_class_info(class_name).unwrap_or_else(|| panic!("Not found {class_name} class")),
        );

        let mut obj = Object::new();
        while opt(end_tag("hkobject"))
            .parse_next(&mut self.input)?
            .is_none()
        {
            let (field_name, value) = self.get_field()?;
            dbg!(&field_name, &value);
            obj.insert(field_name.into(), value);
        }

        self.pop_field_info();

        Ok(BorrowedValue::Object(Box::new(obj)))
    }

    /// # Errors
    /// Parse failed.
    fn get_field(&mut self) -> PResult<(&'de str, BorrowedValue<'de>)> {
        self.opt_comment()?;

        let (field_name, _array_len) = field_start_tag.parse_next(&mut self.input)?;
        self.current_path.push(field_name);

        let value = if let Some(field_info) = self.current.field_info {
            if let Some(json_type) = find_json_parser_by(field_name, field_info) {
                self.parse_value(json_type)?
            } else {
                panic!("No field info: {field_name}. expected: {field_info:?}");
            }
        } else {
            panic!("Failed to get field_info");
        };
        self.opt_comment()?;

        Ok((field_name, value))
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    fn opt_comment(&mut self) -> PResult<()> {
        if let Some(mut xml_comment) = opt(comment()).parse_next(&mut self.input)? {
            let comment_ty = comment_kind.parse_next(&mut xml_comment)?;
            match comment_ty {
                helper::CommentKind::ModCode(id) => self.current.mode_code = Some(id),
                helper::CommentKind::Original => {
                    self.current.is_passed_original = true;
                    // FIXME: Support Caseless
                    take_until(0.., "<!-- CLOSE -->").parse_next(&mut self.input)?;
                    self.add_patches_json();
                }
                helper::CommentKind::Close => self.add_patches_json(),
            }
        };

        Ok(())
    }

    fn add_patches_json(&mut self) {
        // take & reset current states
        let op = self.current.judge_operation();
        let patches = mem::take(&mut self.current.patches);
        self.current.clear_flags();

        for json in patches {
            self.output_patches.push(PatchJson {
                op,
                path: json.path.clone(),
                value: json.value,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_field() {
        let nemesis_xml = r###"
 		<hkobject name="#0010" class="hkbProjectData" signature="0x13a39ba7">
			<hkparam name="worldUpWS">(0.000000 0.000000 1.000000 0.000000)</hkparam>
<!-- MOD_CODE ~id~ OPEN -->
			<hkparam name="stringData">$id</hkparam>
<!-- ORIGINAL -->
			<hkparam name="stringData">#0009</hkparam>
<!-- CLOSE -->
			<hkparam name="defaultEventMode">EVENT_MODE_IGNORE_FROM_GENERATOR</hkparam>
		</hkobject>
"###;

        let actual = patch_value(nemesis_xml).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            actual,
            vec![PatchJson {
                op: Op::Replace,
                path: vec!["$", "0010", "hkbProjectData", "stringData"],
                value: "$id".into(),
            }]
        );
    }

    #[ignore = "cannot pass yet."]
    #[test]
    fn push_array() {
        let nemesis_xml = r###"
		<hkobject name="#0009" class="hkbProjectStringData" signature="0x76ad60a">
			<hkparam name="animationFilenames" numelements="0"></hkparam>
			<hkparam name="behaviorFilenames" numelements="0"></hkparam>
			<hkparam name="characterFilenames" numelements="1">
				<hkcstring>Characters\DefaultMale.hkx</hkcstring>
<!-- MOD_CODE ~id~ OPEN -->
            <hkcstring>PushDummy</hkcstring>
<!-- CLOSE -->
			</hkparam>
			<hkparam name="eventNames" numelements="0"></hkparam>
			<hkparam name="animationPath"></hkparam>
			<hkparam name="behaviorPath"></hkparam>
			<hkparam name="characterPath"></hkparam>
			<hkparam name="fullPathToSource"></hkparam>
		</hkobject>
"###;

        let actual = patch_value(nemesis_xml).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            actual,
            vec![PatchJson {
                op: Op::Add,
                // path: https://crates.io/crates/jsonpath-rust
                path: vec!["$", "0009", "hkbProjectData", "characterFilenames[:]"],
                value: "PushDummy".into(),
            }]
        );
    }

    #[ignore = "cannot pass yet."]
    #[test]
    fn field_in_class_patch() {
        let nemesis_xml = r###"
		<hkobject name="#0008" class="hkRootLevelContainer" signature="0x2772c11e">
			<hkparam name="namedVariants" numelements="1">
				<hkobject>
<!-- MOD_CODE ~id~ OPEN --!>
					<hkparam name="name">ReplaceDummy</hkparam>
<!-- ORIGINAL --!>
					<hkparam name="name">hkbProjectData</hkparam>
<!-- CLOSE --!>
					<hkparam name="className">hkbProjectData</hkparam>
					<hkparam name="variant">#0010</hkparam>
				</hkobject>
			</hkparam>
		</hkobject>
"###;
        let actual = patch_value(nemesis_xml).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            actual,
            vec![PatchJson {
                op: Op::Add,
                // path: https://crates.io/crates/jsonpath-rust
                path: vec!["$", "0009", "hkbProjectData", "characterFilenames"],
                value: "PushDummy".into(),
            }]
        );
    }
}
