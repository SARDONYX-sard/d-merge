mod class_table;
mod current_state;
mod helpers;
mod merge;
mod nemesis;
pub mod patch_json;
mod tag;

use self::{
    class_table::{find_class_info, find_json_parser_by, FieldInfo},
    current_state::{CurrentPatchJson, CurrentState},
    helpers::{delimited_multispace0, pointer},
    nemesis::{
        comment::{close_comment, comment_kind, take_till_close, CommentKind},
        variable::{event_id, variable_id},
    },
    patch_json::PatchJson,
    tag::{class_start_tag, end_tag, field_start_tag, start_tag},
};
use crate::error::{Error, Result};
use serde_hkx::{
    errors::readable::ReadableError,
    xml::de::parser::type_kind::{boolean, real, string},
};
use simd_json::{borrowed::Object, BorrowedValue, StaticNode};
use winnow::{
    ascii::{dec_int, dec_uint, multispace0},
    combinator::{alt, opt},
    error::ContextError,
    Parser,
};

/// # Errors
/// Parse failed.
pub fn parse_nemesis_patch(input: &str) -> Result<Vec<PatchJson<'_>>> {
    let mut patcher_info = PatchDeserializer::new(input);
    patcher_info
        .root_class()
        .map_err(|err| patcher_info.to_readable_err(err))?;
    Ok(patcher_info.output_patches)
}

/// Nemesis patch deserializer
#[derive(Debug, Clone, Default)]
struct PatchDeserializer<'a> {
    /// mutable pointer to str
    input: &'a str,
    /// This is readonly for error report. Not move position.
    original: &'a str,

    /// Output
    output_patches: Vec<PatchJson<'a>>,

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    // current state
    /// N time nested classes fields.
    field_infos: Vec<&'static FieldInfo>,

    /// - `<! -- CLOSE --! >`(XML) where it is temporarily stored because the operation type is unknown until a comment is found.
    /// - `<! -- CLOSE --! >` is found, have it added to `output_patches`.
    pub current: CurrentState<'a>,
    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////
}

impl<'de> PatchDeserializer<'de> {
    const fn new(input: &'de str) -> Self {
        Self {
            current: CurrentState::new(),
            field_infos: Vec::new(),
            input,
            original: input,
            output_patches: Vec::new(),
        }
    }

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    // Parser methods

    fn parse_next<O>(&mut self, mut parser: impl Parser<&'de str, O, ContextError>) -> Result<O> {
        parser
            .parse_next(&mut self.input)
            .map_err(|err| Error::ContextError { err })
    }

    /// Parse by argument parser no consume.
    ///
    /// If an error occurs, it is converted to [`ReadableError`] and returned.
    fn parse_peek<O>(&self, mut parser: impl Parser<&'de str, O, ContextError>) -> Result<O> {
        let (_, res) = parser
            .parse_peek(self.input)
            .map_err(|err| Error::ContextError { err })?;
        Ok(res)
    }

    /// Convert Visitor errors to position-assigned errors.
    ///
    /// # Why is this necessary?
    /// Because Visitor errors that occur within each `Deserialize` implementation cannot indicate the error location in XML.
    #[cold]
    fn to_readable_err(&self, err: Error) -> Error {
        let readable = match err {
            Error::ContextError { err } => ReadableError::from_context(
                err,
                self.original,
                self.original.len() - self.input.len(),
            ),
            Error::ReadableError { source } => source,
            err => ReadableError::from_display(
                err,
                self.original,
                self.original.len() - self.input.len(),
            ),
        };

        Error::ReadableError { source: readable }
    }

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    fn push_field_info(&mut self, info: &'static FieldInfo) {
        self.field_infos.push(info);
        self.current.field_info = Some(info);
    }

    fn pop_field_info(&mut self) {
        self.field_infos.pop();
        self.current.field_info = self.field_infos.last().map(|v| &**v);
    }

    fn root_class(&mut self) -> Result<()> {
        let (ptr_index, class_name) = self.parse_next(class_start_tag)?;
        self.current.path.push("$".into());
        self.current.path.push(ptr_index.into());
        self.current.path.push(class_name.into());

        {
            let field_info = find_class_info(class_name).ok_or(Error::UnknownClass {
                class_name: class_name.to_string(),
            })?;
            self.push_field_info(field_info);
        }

        while self.parse_next(opt(end_tag("hkobject")))?.is_none() {
            self.field()?;
        }

        self.pop_field_info();
        self.current.path.pop();
        Ok(())
    }

    /// # Errors
    /// Parse failed.
    fn class_in_field(&mut self, class_name: &'static str) -> Result<BorrowedValue<'de>> {
        self.parse_next(start_tag("hkobject"))?;
        self.current.path.push(class_name.into());

        {
            let field_info = find_class_info(class_name).ok_or(Error::UnknownClass {
                class_name: class_name.to_string(),
            })?;
            self.push_field_info(field_info);
        }

        let mut obj = Object::new();
        while self.parse_next(opt(end_tag("hkobject")))?.is_none() {
            let (field_name, value) = self.field()?;
            obj.insert(field_name.into(), value);
        }

        self.pop_field_info();
        self.current.path.pop();

        Ok(BorrowedValue::Object(Box::new(obj)))
    }

    /// # Errors
    /// Parse failed.
    fn field(&mut self) -> Result<(&'de str, BorrowedValue<'de>)> {
        let should_take_in_this = self.parse_start_maybe_comment()?;
        let (field_name, _) = self.parse_next(field_start_tag)?;
        self.current.path.push(field_name.into());

        let value = {
            let json_type = self
                .current
                .field_info
                .and_then(|field_info| find_json_parser_by(field_name, field_info))
                .ok_or(Error::UnknownField {
                    field_name: field_name.to_string(),
                })?;
            let value = self.parse_value(json_type)?;

            #[cfg(feature = "tracing")]
            tracing::debug!(?field_name, ?value);
            if should_take_in_this {
                self.current.push_current_patch(value);
                Default::default() // return dummy
            } else {
                value
            }
        };

        self.parse_next(end_tag("hkparam"))?;
        self.parse_maybe_close_comment()?;
        self.current.path.pop();

        Ok((field_name, value))
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    /// non Array or Struct(Object)
    /// - Bool
    /// - I64
    /// - U64
    /// - F64
    /// - Null
    /// - Pointer
    /// - String
    fn parse_plane_value(&mut self, field_type: &'static str) -> Result<BorrowedValue<'de>> {
        let value = match field_type {
            "Null" => BorrowedValue::Static(StaticNode::Null),
            "I64" => self.parse_i64()?,
            "U64" => self.parse_u64()?,
            "F64" => self.parse_next(real.map(|real| real.into()))?,
            "String" => self.parse_string_ptr()?, // StringPtr | CString
            "Pointer" => self.parse_next(pointer)?,
            "Bool" => self.parse_next(boolean.map(|boolean| boolean.into()))?,
            unknown => {
                return Err(Error::UnknownFieldType {
                    field_type: unknown.to_string(),
                })
            }
        };
        Ok(value)
    }

    /// Parse [`i64`]
    fn parse_i64(&mut self) -> Result<BorrowedValue<'de>> {
        let event_parser = event_id.map(|s| s.into());
        let var_parser = variable_id.map(|s| s.into());
        let int_parser = dec_int.map(|int: i64| int.into());
        let value = self.parse_next(alt((int_parser, event_parser, var_parser)))?;
        Ok(value)
    }

    fn parse_u64(&mut self) -> Result<BorrowedValue<'de>> {
        let event_parser = event_id.map(|s| s.into());
        let var_parser = variable_id.map(|s| s.into());
        let int_parser = dec_uint.map(|int: u64| int.into());
        let value = self.parse_next(alt((int_parser, event_parser, var_parser)))?;
        Ok(value)
    }

    /// Parse `CString` | `StringPtr`
    fn parse_string_ptr(&mut self) -> Result<BorrowedValue<'de>> {
        self.parse_next(alt((
            "\u{2400}".value(BorrowedValue::Static(StaticNode::Null)), // StringPtr | CString
            string.map(BorrowedValue::String),
        )))
    }

    fn parse_real(&mut self) -> Result<f32> {
        self.parse_next(multispace0)?;
        let should_take_in_this = self.parse_start_maybe_comment()?;
        self.parse_next(multispace0)?;

        let value = self.parse_next(real)?;
        if should_take_in_this {
            self.current.push_current_patch(value.into());
        };

        self.parse_next(multispace0)?;
        self.parse_maybe_close_comment()?;
        Ok(value)
    }

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    // math types

    /// Parse `Matrix3`, `Rotation`
    fn parse_matrix3(&mut self) -> Result<BorrowedValue<'de>> {
        let mut obj = Object::new();

        obj.insert("x".into(), self.parse_vector4()?);
        obj.insert("y".into(), self.parse_vector4()?);
        obj.insert("z".into(), self.parse_vector4()?);

        Ok(BorrowedValue::Object(Box::new(obj)))
    }

    fn parse_matrix4(&mut self) -> Result<BorrowedValue<'de>> {
        let mut obj = Object::new();

        obj.insert("x".into(), self.parse_vector4()?);
        obj.insert("y".into(), self.parse_vector4()?);
        obj.insert("z".into(), self.parse_vector4()?);
        obj.insert("w".into(), self.parse_vector4()?);

        Ok(BorrowedValue::Object(Box::new(obj)))
    }

    fn parse_qs_transform(&mut self) -> Result<BorrowedValue<'de>> {
        let mut obj = Object::new();

        obj.insert("transition".into(), self.parse_vector4()?);
        obj.insert("quaternion".into(), self.parse_quaternion()?);
        obj.insert("scale".into(), self.parse_vector4()?);

        Ok(BorrowedValue::Object(Box::new(obj)))
    }

    fn parse_quaternion(&mut self) -> Result<BorrowedValue<'de>> {
        let mut obj = Object::new();

        self.parse_next(opt(delimited_multispace0("(")))?;
        obj.insert("x".into(), self.parse_real()?.into());
        obj.insert("y".into(), self.parse_real()?.into());
        obj.insert("z".into(), self.parse_real()?.into());
        obj.insert("scaler".into(), self.parse_real()?.into());
        self.parse_next(opt(delimited_multispace0(")")))?;

        Ok(BorrowedValue::Object(Box::new(obj)))
    }

    fn parse_transform(&mut self) -> Result<BorrowedValue<'de>> {
        let mut obj = Object::new();
        obj.insert("rotation".into(), self.parse_matrix3()?);
        obj.insert("transition".into(), self.parse_vector4()?);
        Ok(BorrowedValue::Object(Box::new(obj)))
    }

    fn parse_vector4(&mut self) -> Result<BorrowedValue<'de>> {
        let mut obj = Object::new();

        self.parse_next(opt(delimited_multispace0("(")))?;
        obj.insert("x".into(), self.parse_real()?.into());
        obj.insert("y".into(), self.parse_real()?.into());
        obj.insert("z".into(), self.parse_real()?.into());
        obj.insert("w".into(), self.parse_real()?.into());
        self.parse_next(opt(delimited_multispace0(")")))?;

        Ok(BorrowedValue::Object(Box::new(obj)))
    }

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////

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
    fn parse_value(&mut self, field_type: &'static str) -> Result<BorrowedValue<'de>> {
        self.parse_next(multispace0)?;
        let should_take_in_this = self.parse_start_maybe_comment()?;
        self.parse_next(multispace0)?;

        let value = match field_type {
            obj if obj.starts_with("Object|") => {
                let class_name = &obj[7..]; // Remove "object|"
                match class_name {
                    "Matrix3" | "Rotation" => self.parse_matrix3()?,
                    "Matrix4" => self.parse_matrix4()?,
                    "QsTransform" => self.parse_qs_transform()?,
                    "Quaternion" => self.parse_quaternion()?,
                    "Transform" => self.parse_transform()?,
                    "Vector4" => self.parse_vector4()?,
                    _ => self.class_in_field(class_name)?, // Start with `<hkobject>`
                }
            }
            arr if arr.starts_with("Array|") => {
                let mut vec = vec![];

                let name = &arr[6..]; // Remove "array|"
                if name.starts_with("String") {
                    let mut index = 0;
                    while self.parse_peek(opt(end_tag("hkparam")))?.is_none() {
                        let should_take_in_this = self.parse_start_maybe_comment()?;
                        self.parse_next(start_tag("hkcstring"))?;
                        self.current.path.push(format!("[{index}]").into());
                        index += 1;

                        let value = self.parse_string_ptr()?;
                        if should_take_in_this {
                            self.current.push_current_patch(value);
                        } else {
                            vec.push(value);
                        };
                        self.parse_next(end_tag("hkcstring"))?;
                        self.parse_maybe_close_comment()?;
                        self.current.path.pop();
                    }
                } else if !name.starts_with("Null") {
                    let mut index = 0;
                    while self.parse_peek(opt(end_tag("hkparam")))?.is_none() {
                        self.current.path.push(format!("[{index}]").into());
                        index += 1;

                        let should_take_in_this = self.parse_start_maybe_comment()?;
                        let value = self.parse_value(name)?;
                        if should_take_in_this {
                            self.current.push_current_patch(value);
                        } else {
                            vec.push(value);
                        };
                        self.parse_maybe_close_comment()?;
                        self.current.path.pop();
                    }
                };
                BorrowedValue::Array(Box::new(vec)) // `Null`(void)
            }
            other => self.parse_plane_value(other)?,
        };

        let value = if should_take_in_this {
            self.current.push_current_patch(value);
            // NOTE: Since the comment indicates that the change is to change a single value,
            //       the `value` is used only within this function, so a dummy is returned.
            Default::default()
        } else {
            value
        };

        self.parse_next(multispace0)?;
        self.parse_maybe_close_comment()?;
        Ok(value)
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    /// # Return
    /// Is the mode code comment?
    fn parse_start_maybe_comment(&mut self) -> Result<bool> {
        if let Some(comment_ty) = self.parse_next(opt(comment_kind))? {
            #[cfg(feature = "tracing")]
            tracing::debug!(?comment_ty);
            if let CommentKind::ModCode(id) = comment_ty {
                self.current.mode_code = Some(id);
                return Ok(true);
            } else {
                return Ok(false);
            }
        }
        Ok(false)
    }

    /// ORIGINAL or CLOSE
    fn parse_maybe_close_comment(&mut self) -> Result<()> {
        if let Some(comment_ty) = self.parse_next(opt(close_comment))? {
            #[cfg(feature = "tracing")]
            tracing::debug!(?comment_ty);
            match comment_ty {
                CommentKind::Original => {
                    self.current.set_is_passed_original();
                    self.parse_next(take_till_close)?;
                    self.add_patch_json();
                }
                CommentKind::Close => {
                    self.add_patch_json();
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn add_patch_json(&mut self) {
        let (op, patches) = self.current.take_patches();
        for CurrentPatchJson { path, value } in patches {
            self.output_patches.push(PatchJson { op, path, value });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use patch_json::Op;

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

        let actual = parse_nemesis_patch(nemesis_xml).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            actual,
            vec![PatchJson {
                op: Op::Replace,
                path: vec!["$", "0010", "hkbProjectData", "stringData"]
                    .into_iter()
                    .map(|s| s.into())
                    .collect(),
                value: "$id".into(),
            }]
        );
    }

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

        let actual = parse_nemesis_patch(nemesis_xml).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            actual,
            vec![PatchJson {
                op: Op::Add,
                // path: https://crates.io/crates/jsonpath-rust
                path: vec![
                    "$",
                    "0009",
                    "hkbProjectStringData",
                    "characterFilenames",
                    "[1]"
                ]
                .into_iter()
                .map(|s| s.into())
                .collect(),
                value: "PushDummy".into(),
            }]
        );
    }

    #[test]
    fn field_in_class_patch() {
        let nemesis_xml = r###"
		<hkobject name="#0008" class="hkRootLevelContainer" signature="0x2772c11e">
			<hkparam name="namedVariants" numelements="1">
				<hkobject>
<!-- MOD_CODE ~id~ OPEN -->
					<hkparam name="name">ReplaceDummy</hkparam>
<!-- ORIGINAL -->
					<hkparam name="name">hkbProjectData</hkparam>
<!-- CLOSE -->
					<hkparam name="className">hkbProjectData</hkparam>
					<hkparam name="variant">#0010</hkparam>
				</hkobject>
			</hkparam>
		</hkobject>
"###;
        let actual = parse_nemesis_patch(nemesis_xml).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            actual,
            vec![PatchJson {
                op: Op::Replace,
                // path: https://crates.io/crates/jsonpath-rust
                path: [
                    "$",
                    "0008",
                    "hkRootLevelContainer",
                    "namedVariants",
                    "[0]",
                    "hkRootLevelContainerNamedVariant",
                    "name"
                ]
                .into_iter()
                .map(|s| s.into())
                .collect(),
                value: "ReplaceDummy".into()
            }]
        );
    }

    #[cfg_attr(feature = "tracing", quick_tracing::init)]
    #[ignore = "dummy"]
    #[test]
    fn parse() {
        let nemesis_xml = &{
            // let path = "../dummy/mods/zcbe/_1stperson/staggerbehavior/#0052.txt";
            // let path = "../dummy/mods/turn/1hm_behavior/#0087.txt";
            let path = "../dummy/mods/zcbe/_1stperson/staggerbehavior/#0087.txt";
            std::fs::read_to_string(path).unwrap()
        };
        dbg!(parse_nemesis_patch(nemesis_xml).unwrap_or_else(|e| panic!("{e}")));
    }
}
