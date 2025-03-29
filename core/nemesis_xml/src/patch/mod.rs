pub mod class_table;
mod current_state;

use self::{
    class_table::{find_class_info, FieldInfo},
    current_state::{CurrentJsonPatch, CurrentState},
};
use crate::helpers::{
    comment::{close_comment, comment_kind, take_till_close, CommentKind},
    delimited_multispace0,
    ptr::pointer,
    tag::{class_start_tag, end_tag, field_start_tag, start_tag},
    variable::{event_id, variable_id},
};
use crate::{
    error::{Error, Result},
    helpers::tag::PointerType,
};
use json_patch::{JsonPatch, JsonPath, Op, OpRange, OpRangeKind};
use rayon::prelude::*;
use serde_hkx::{
    errors::readable::ReadableError,
    xml::de::parser::type_kind::{boolean, real, string},
};
use simd_json::{
    base::ValueTryAsArrayMut, borrowed::Object, BorrowedValue, StaticNode, ValueBuilder,
};
use std::{collections::HashMap, mem};
use winnow::{
    ascii::{dec_int, dec_uint, multispace0},
    combinator::{alt, opt},
    error::ContextError,
    Parser,
};

pub type PatchesMap<'a> = HashMap<JsonPath<'a>, JsonPatch<'a>>;

/// # Errors
/// Parse failed.
pub fn parse_nemesis_patch(nemesis_xml: &str) -> Result<(PatchesMap<'_>, Option<&str>)> {
    let mut patcher_info = PatchDeserializer::new(nemesis_xml);
    patcher_info
        .root_class()
        .map_err(|err| patcher_info.to_readable_err(err))?;
    Ok((patcher_info.output_patches, patcher_info.id_index))
}

/// Nemesis patch deserializer
#[derive(Debug, Clone, Default)]
struct PatchDeserializer<'a> {
    /// mutable pointer to str
    input: &'a str,
    /// This is readonly for error report. Not move position.
    original: &'a str,

    /// Output
    // output_patches: Vec<JsonPatch<'a>>,
    output_patches: HashMap<JsonPath<'a>, JsonPatch<'a>>,

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    // current state
    /// N time nested classes fields.
    field_infos: Vec<&'static FieldInfo>,

    /// - `<! -- CLOSE --! >`(XML) where it is temporarily stored because the operation type is unknown until a comment is found.
    /// - `<! -- CLOSE --! >` is found, have it added to `output_patches`.
    pub current: CurrentState<'a>,

    /// `hkbBehaviorGraphStringData`
    ///
    /// event_id, variable_id
    id_index: Option<&'a str>,
    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////
}

impl<'de> PatchDeserializer<'de> {
    fn new(input: &'de str) -> Self {
        Self {
            current: CurrentState::new(),
            field_infos: Vec::new(),
            input,
            original: input,
            output_patches: HashMap::new(),
            id_index: None,
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

    fn push_current_field_table(&mut self, info: &'static FieldInfo) {
        self.field_infos.push(info);
        self.current.field_info = Some(info);
    }

    fn pop_current_field_table(&mut self) {
        self.field_infos.pop();
        self.current.field_info = self.field_infos.last().map(|v| &**v);
    }

    fn root_class(&mut self) -> Result<()> {
        let (ptr_index, class_name) = self.parse_next(class_start_tag)?;

        let (should_take_in_this, ptr_index) = match ptr_index {
            PointerType::Index(index) => (false, index),
            PointerType::Var(id) => (true, id), // $id$2
        };

        if class_name == "hkbBehaviorGraphStringData" {
            self.id_index = Some(ptr_index);
        }
        self.current.path.push(ptr_index.into());
        self.current.path.push(class_name.into());

        {
            let field_info = find_class_info(class_name).ok_or(Error::UnknownClass {
                class_name: class_name.to_string(),
            })?;
            self.push_current_field_table(field_info);
        }

        let mut obj = Object::new();
        while self.parse_next(opt(end_tag("hkobject")))?.is_none() {
            let (field_name, value) = self.field()?;
            if should_take_in_this {
                obj.insert(field_name.into(), value);
            }
        }
        self.pop_current_field_table();

        if should_take_in_this {
            let path = mem::take(&mut self.current.path);
            self.output_patches.insert(
                path,
                JsonPatch {
                    op: OpRangeKind::Pure(Op::Add), // root class
                    value: BorrowedValue::Object(Box::new(obj)),
                },
            );
        }

        // NOTE: no need remove class name on root class.
        Ok(())
    }

    /// # Errors
    /// Parse failed.
    fn class_in_field(&mut self, class_name: &'static str) -> Result<BorrowedValue<'de>> {
        self.parse_next(start_tag("hkobject"))?;

        {
            let field_info = find_class_info(class_name).ok_or(Error::UnknownClass {
                class_name: class_name.to_string(),
            })?;
            self.push_current_field_table(field_info);
        }

        let mut obj = Object::new();
        while self.parse_next(opt(end_tag("hkobject")))?.is_none() {
            let (field_name, value) = self.field()?;
            obj.insert(field_name.into(), value);
        }
        self.pop_current_field_table();

        Ok(BorrowedValue::Object(Box::new(obj)))
    }

    /// # Errors
    /// Parse failed.
    fn field(&mut self) -> Result<(&'de str, BorrowedValue<'de>)> {
        let should_take_in_this = self.parse_start_maybe_comment()?;

        let field_info = self
            .current
            .field_info
            .ok_or_else(|| Error::MissingFieldInfo)?;
        let (field_name, field_type, _) = self.parse_next(field_start_tag(field_info))?;
        self.current.path.push(field_name.into());

        let value = {
            let value = self.parse_value(field_type)?;

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

    fn parse_real(&mut self) -> Result<BorrowedValue<'de>> {
        self.parse_next(multispace0)?;
        let should_take_in_this = self.parse_start_maybe_comment()?;
        self.parse_next(multispace0)?;

        let value = self.parse_next(real)?;
        #[cfg(feature = "tracing")]
        tracing::debug!(should_take_in_this, ?value);
        if should_take_in_this {
            self.current.push_current_patch(value.into());
        };

        self.parse_next(multispace0)?;
        self.parse_maybe_close_comment()?;
        self.parse_next(multispace0)?;
        Ok(value.into())
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
        obj.insert("x".into(), self.parse_real()?);
        obj.insert("y".into(), self.parse_real()?);
        obj.insert("z".into(), self.parse_real()?);
        obj.insert("scaler".into(), self.parse_real()?);
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
        obj.insert("x".into(), self.parse_real()?);
        obj.insert("y".into(), self.parse_real()?);
        obj.insert("z".into(), self.parse_real()?);
        obj.insert("w".into(), self.parse_real()?);
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
        // NOTE: Is there any possibility of a comment indicating field change intent coming here?
        // If there is a comment directly below `<hkparam>` indicating a differential change, it is a change in value alone,
        // such as a part of Vector4.
        // If we want to change the field data itself, we should indicate the `<hkparam>` situation with a
        // comment showing the difference.

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
            arr if arr.starts_with("Array|") => self.parse_array(arr)?,
            other => self.parse_plane_value(other)?,
        };

        Ok(value)
    }

    fn parse_array(&mut self, arr: &'static str) -> Result<BorrowedValue<'de>> {
        // Array|Null
        let name = &arr[6..]; // Remove "array|"
        let mut vec = vec![];

        if name.starts_with("Null") {
            return Ok(BorrowedValue::Array(Box::new(vec))); // `Null`(void)
        };

        let mut index = 0;
        let mut should_take_in_this = false;
        while self.parse_peek(opt(end_tag("hkparam")))?.is_none() {
            // seq start
            let is_start = self.parse_start_maybe_comment()?;
            if is_start {
                should_take_in_this = true;
                self.current.seq_range = Some(index..index); // Start range
            }

            // seq inner
            let value = if name.starts_with("String") {
                // <hkcstring>String</hkcstring>
                self.parse_next(start_tag("hkcstring"))?;
                let value = self.parse_string_ptr()?;
                self.parse_next(end_tag("hkcstring"))?;
                value
            } else {
                // NOTE: In the case of nested seq patterns, intermediate indexes
                // need to be added here because they require a path
                // (in case of Array|Object|<ClassName>)
                if self.current.seq_range.is_none() {
                    self.current.path.push(format!("[{index}]").into()); // only for class
                }
                let value = self.parse_value(name)?;
                // If not a class, remove `[index]` because of non-nesting.
                //  Array|Object|<ClassName> ...Array|<TypeName> -> pop
                if self.current.seq_range.is_none() {
                    self.current.path.pop();
                }
                value
            };

            #[cfg(feature = "tracing")]
            {
                tracing::trace!("{:#?}", self.current);
                tracing::trace!("{value:#?}");
            }

            // seq end
            if should_take_in_this {
                self.current.push_current_patch(value);
            } else {
                vec.push(value);
            };
            index += 1;
            self.current.increment_range();
            if self.parse_maybe_close_comment()? {
                should_take_in_this = false;
            };
        }

        self.current.seq_range = None;
        Ok(BorrowedValue::Array(Box::new(vec)))
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    /// # Return
    /// Is the mode code comment?
    fn parse_start_maybe_comment(&mut self) -> Result<bool> {
        if let Some(comment_ty) = self.parse_next(opt(comment_kind))? {
            #[cfg(feature = "tracing")]
            tracing::debug!(?comment_ty);
            match comment_ty {
                CommentKind::ModCode(id) => {
                    self.current.mode_code = Some(id);
                    self.parse_maybe_close_comment()?; // When close comes here, it is Remove.
                    return Ok(true);
                }
                _ => return Ok(false),
            }
        }
        Ok(false)
    }

    /// Processes the close comment (`ORIGINAL` or `CLOSE`) depending on whether it was encountered,
    /// and returns whether it was encountered or not.
    fn parse_maybe_close_comment(&mut self) -> Result<bool> {
        if let Some(comment_ty) = self.parse_next(opt(close_comment))? {
            #[cfg(feature = "tracing")]
            tracing::debug!(?comment_ty);
            match comment_ty {
                CommentKind::Original => {
                    self.current.set_is_passed_original();
                    let op = self.current.judge_operation();
                    // NOTE: `Op::Remove` passes through here because it needs to count the number of deletions
                    if op != Op::Remove {
                        #[cfg(feature = "tracing")]
                        {
                            tracing::debug!(?op);
                            tracing::trace!("{:#?}", self.current);
                        }
                        self.parse_next(take_till_close)?;
                        self.extend_output_patches();
                    }
                    return Ok(true);
                }
                CommentKind::Close => {
                    self.extend_output_patches();
                    return Ok(true);
                }
                _ => {}
            }
        }
        Ok(false)
    }

    /// This is the method that is called when a single differential change comment pair finishes calling.
    fn extend_output_patches(&mut self) {
        // range diff pattern
        if let Some(new_range) = self.current.seq_range.take() {
            // NOTE: If op is not calculated before taking `seq_values`,
            // it will cause a misjudgment since it will be a remove process when `seq_values` is empty.
            let op = self.current.judge_operation();

            let path = self.current.path.clone(); // needless clone? replace?
            let seq_values = mem::take(&mut self.current.seq_values);

            let value = if op == Op::Remove {
                BorrowedValue::null() // no add
            } else {
                BorrowedValue::Array(Box::new(seq_values))
            };

            // Discrete
            if let Some(prev_patch) = self.output_patches.remove(&path) {
                match prev_patch.op {
                    // twice discrete array
                    OpRangeKind::Seq(OpRange {
                        op: prev_op,
                        range: prev_range,
                    }) => {
                        let merged_op_range = OpRangeKind::Discrete(vec![
                            OpRange {
                                op: prev_op,
                                range: prev_range,
                            },
                            OpRange {
                                op,
                                range: new_range,
                            },
                        ]);

                        self.output_patches.insert(
                            path,
                            JsonPatch {
                                op: merged_op_range,
                                value: vec![prev_patch.value, value].into(),
                            },
                        );
                    }
                    // 3 times and more discrete array
                    OpRangeKind::Discrete(prev_vec_range) => {
                        let mut merged_op_range = prev_vec_range;
                        merged_op_range.push(OpRange {
                            op,
                            range: new_range,
                        });

                        let mut new_value = prev_patch.value;
                        if let Ok(array) = new_value.try_as_array_mut() {
                            array.push(value);
                        }

                        self.output_patches.insert(
                            path,
                            JsonPatch {
                                op: OpRangeKind::Discrete(merged_op_range),
                                value: new_value,
                            },
                        );
                    }
                    OpRangeKind::Pure(_) => {} // We do not consider it strange that a patch to an existing field comes twice.
                }
            } else {
                // New array patch
                self.output_patches.insert(
                    path,
                    JsonPatch {
                        op: OpRangeKind::Seq(OpRange {
                            op,
                            range: new_range,
                        }),
                        value,
                    },
                );
            }

            self.current.clear_flags(); // new patch is generated so clear flags.
            return;
        }

        // one diff pattern
        let (op, patches) = self.current.take_patches();
        self.output_patches.par_extend(
            patches
                .into_par_iter()
                .map(|CurrentJsonPatch { path, value }| {
                    (
                        path,
                        JsonPatch {
                            op: OpRangeKind::Pure(op),
                            value,
                        },
                    )
                })
                .collect::<HashMap<JsonPath<'_>, JsonPatch<'_>>>(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use json_patch::{json_path, OpRange};
    use simd_json::json_typed;

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

        let (actual, _) = parse_nemesis_patch(nemesis_xml).unwrap_or_else(|e| panic!("{e}"));
        // if map.contain_keys() {}

        let mut hash_map = HashMap::new();
        hash_map.insert(
            json_path!["#0010", "hkbProjectData", "stringData"],
            JsonPatch {
                op: OpRangeKind::Pure(Op::Replace),
                value: "$id".into(),
            },
        );

        assert_eq!(actual, hash_map);
    }

    // key: vec!["#0010", "hkbProjectData", "Array", "[100:101]"]
    // vec![ptr, ptr, ptr]
    // value: JsonPatch
    //
    //- patch1
    // key: vec!["#0010", "hkbProjectData", "Array"], range1
    // Patch1HashMap.get(Patch2HashMap.first().unwrap())
    //
    //- patch2
    // key: vec!["#0010", "hkbProjectData", "Array", range2
    // range1 : range2
    //
    // vec![ptr, ptr, ptr]
    // value: JsonPatch
    //
    // [op, path, range, value]

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
            <hkcstring>PushDummy</hkcstring>
            <hkcstring>PushDummy</hkcstring>
            <hkcstring>PushDummy</hkcstring>
            <hkcstring>PushDummy</hkcstring>
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

        let (actual, _) = parse_nemesis_patch(nemesis_xml).unwrap_or_else(|e| panic!("{e}"));
        let mut hash_map = HashMap::new();

        hash_map.insert(
            json_path!["#0009", "hkbProjectStringData", "characterFilenames"],
            JsonPatch {
                op: OpRangeKind::Seq(OpRange {
                    op: Op::Add,
                    range: 1..7,
                }),
                // path: https://crates.io/crates/jsonpath-rust
                value: json_typed!(
                    borrowed,
                    [
                        "PushDummy",
                        "PushDummy",
                        "PushDummy",
                        "PushDummy",
                        "PushDummy",
                        "PushDummy"
                    ]
                ),
            },
        );

        assert_eq!(actual, hash_map);
    }

    #[test]
    fn discrete_push_array() {
        let nemesis_xml = r###"
		<hkobject name="#0009" class="hkbProjectStringData" signature="0x76ad60a">
			<hkparam name="animationFilenames" numelements="0"></hkparam>
			<hkparam name="behaviorFilenames" numelements="0"></hkparam>
			<hkparam name="characterFilenames" numelements="1">
				<hkcstring>Characters\DefaultMale.hkx</hkcstring>
<!-- MOD_CODE ~id~ OPEN -->
                <hkcstring>PushDummy</hkcstring>
                <hkcstring>PushDummy</hkcstring>
                <hkcstring>PushDummy</hkcstring>
                <hkcstring>PushDummy</hkcstring>
                <hkcstring>PushDummy</hkcstring>
                <hkcstring>PushDummy</hkcstring>
<!-- CLOSE -->
                <hkcstring>Original</hkcstring>
                <hkcstring>Original</hkcstring>
                <hkcstring>Original</hkcstring>
                <hkcstring>Original</hkcstring>
<!-- MOD_CODE ~id~ OPEN -->

<!-- ORIGINAL -->
                <hkcstring>Original</hkcstring>
                <hkcstring>Original</hkcstring>
<!-- CLOSE -->
			</hkparam>
			<hkparam name="behaviorPath"></hkparam>
			<hkparam name="characterPath"></hkparam>
			<hkparam name="fullPathToSource"></hkparam>
		</hkobject>
"###;
        let (actual, _) = parse_nemesis_patch(nemesis_xml).unwrap_or_else(|e| panic!("{e}"));
        let mut hash_map = HashMap::new();

        let array_value = json_typed!(
            borrowed,
            [
                "PushDummy",
                "PushDummy",
                "PushDummy",
                "PushDummy",
                "PushDummy",
                "PushDummy"
            ]
        );

        hash_map.insert(
            json_path!["#0009", "hkbProjectStringData", "characterFilenames"],
            JsonPatch {
                op: OpRangeKind::Discrete(vec![
                    OpRange {
                        op: Op::Add,
                        range: 1..7,
                    },
                    OpRange {
                        op: Op::Remove,
                        range: 11..13,
                    },
                ]),
                // path: https://crates.io/crates/jsonpath-rust
                value: json_typed!(borrowed, [array_value, null]),
            },
        );

        assert_eq!(actual, hash_map);
    }

    #[cfg_attr(feature = "tracing", quick_tracing::init)]
    #[test]
    fn remove_array() {
        let nemesis_xml = r###"
		<hkobject name="#0009" class="hkbProjectStringData" signature="0x76ad60a">
			<hkparam name="animationFilenames" numelements="0"></hkparam>
			<hkparam name="behaviorFilenames" numelements="0"></hkparam>
			<hkparam name="characterFilenames" numelements="3">
				<hkcstring>Characters\DefaultMale.hkx</hkcstring>
				<hkcstring>Characters\DefaultMale.hkx</hkcstring>
				<hkcstring>Characters\DefaultMale.hkx</hkcstring>
				<hkcstring>Characters\DefaultMale.hkx</hkcstring>
				<hkcstring>Characters\DefaultMale.hkx</hkcstring>
<!-- MOD_CODE ~id~ OPEN -->

<!-- ORIGINAL -->
				<hkcstring>Characters\DefaultMale.hkx</hkcstring>
				<hkcstring>Characters\DefaultMale.hkx</hkcstring>
<!-- CLOSE -->
			</hkparam>
			<hkparam name="eventNames" numelements="0"></hkparam>
			<hkparam name="animationPath"></hkparam>
			<hkparam name="behaviorPath"></hkparam>
			<hkparam name="characterPath"></hkparam>
			<hkparam name="fullPathToSource"></hkparam>
		</hkobject>
"###;

        let (actual, _) = parse_nemesis_patch(nemesis_xml).unwrap_or_else(|e| panic!("{e}"));
        let json_path = json_path!["#0009", "hkbProjectStringData", "characterFilenames"];

        let mut hash_map = HashMap::new();

        hash_map.insert(
            json_path,
            JsonPatch {
                op: OpRangeKind::Seq(OpRange {
                    op: Op::Remove,
                    range: 5..7,
                }),
                value: Default::default(),
            },
        );
        assert_eq!(actual, hash_map);
    }

    // Todo: Consider designing for unmodified lines between patch merges
    #[cfg_attr(feature = "tracing", quick_tracing::init)]
    #[test]
    fn replace_array() {
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
        let (actual, _) = parse_nemesis_patch(nemesis_xml).unwrap_or_else(|e| panic!("{e}"));
        let json_path = json_path![
            "#0008",
            "hkRootLevelContainer",
            "namedVariants",
            "[0]",
            "hkRootLevelContainerNamedVariant",
            "name"
        ];
        let mut hash_map = HashMap::new();

        hash_map.insert(
            json_path.clone(),
            JsonPatch {
                op: OpRangeKind::Pure(Op::Replace),
                // path: https://crates.io/crates/jsonpath-rust
                value: "ReplaceDummy".into(),
            },
        );

        assert_eq!(actual, hash_map);
    }

    #[cfg_attr(
        feature = "tracing",
        quick_tracing::init(file = "./parse.log", stdio = false)
    )]
    #[ignore = "because we need external test files"]
    #[test]
    fn parse() {
        use std::fs::read_to_string;

        let nemesis_xml = {
            // let path = "zcbe/_1stperson/staggerbehavior/#0052.txt";
            // let path = "turn/1hm_behavior/#0087.txt";
            // let path = "zcbe/_1stperson/staggerbehavior/#0087.txt";
            // let path = "zcbe/_1stperson/firstperson/#0060.txt";
            let path = "turn/1hm_behavior/#2781.txt";
            let path = std::path::Path::new("../../dummy/Data/Nemesis_Engine/mod/").join(path);
            read_to_string(path).unwrap()
        };
        dbg!(parse_nemesis_patch(&nemesis_xml).unwrap_or_else(|e| panic!("{e}")));
    }
}
