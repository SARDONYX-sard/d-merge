use super::delimited_multispace0;
use crate::{
    error::Error,
    patch::class_table::{find_json_parser_by, FieldInfo},
};
use serde_hkx::xml::de::parser::{delimited_with_multispace0, tag::attr_string};
use std::str::FromStr;
use winnow::{
    ascii::digit1,
    combinator::{alt, delimited, seq},
    error::{
        ContextError, ErrMode, StrContext,
        StrContextValue::{self},
    },
    token::take_until,
    ModalResult, Parser,
};

/// Parses the start tag `<tag>`
///
/// - `tag`: e.g. `<string>`
pub fn start_tag<'a>(tag: &'static str) -> impl Parser<&'a str, (), ErrMode<ContextError>> {
    seq!(
        _: delimited_with_multispace0("<"),
        _: delimited_with_multispace0(tag),
        _: delimited_with_multispace0(">")
    )
    .context(StrContext::Label("start tag"))
    .context(StrContext::Label(tag))
}

/// Parses the end tag `</tag>`
pub fn end_tag<'a>(tag: &'static str) -> impl Parser<&'a str, (), ErrMode<ContextError>> {
    seq!(
        _: delimited_with_multispace0("<"),
        _: delimited_with_multispace0("/"),
        _: delimited_with_multispace0(tag),
        _: delimited_with_multispace0(">")
    )
    .context(StrContext::Label("end tag"))
    .context(StrContext::Label(tag))
}

/// Parses the array start tag (e.g. `<hkobject name="#0010" class="hkbProjectData" signature="0x13a39ba7">`)
///
/// # Returns
/// ([`Pointer`], ClassName) -> e.g. (`#0010`, `"hkbProjectData"`)
///
/// # Errors
/// Parse failed.
pub fn class_start_tag<'a>(input: &mut &'a str) -> ModalResult<(PointerType<'a>, &'a str)> {
    seq!(
        _: delimited_with_multispace0("<"),
        _: delimited_with_multispace0("hkobject"),
        _: delimited_with_multispace0("name"),
        _: delimited_with_multispace0("="),
        index_name_attr,
        _: delimited_with_multispace0("class"),
        _: delimited_with_multispace0("="),
        attr_string,
        _: delimited_with_multispace0("signature"),
        _: delimited_with_multispace0("="),
        _: attr_string,
        _: delimited_with_multispace0(">")
    )
    .context(StrContext::Label("Class start tag"))
    .context(StrContext::Expected(StrContextValue::Description(
        r##"e.g. `<hkobject name="#0010" class="hkbProjectData" signature="0x13a39ba7">`"##,
    )))
    .parse_next(input)
}

/// Parses the field of class start opening tag `<hkparam name=`
///
/// # Errors
/// If encountered unexpected string.
/// # Note
/// All arguments are used only for clarity of error reporting.
pub fn field_start_open_tag(input: &mut &str) -> ModalResult<()> {
    seq!(
        _: delimited_with_multispace0("<"),
        _: delimited_with_multispace0("hkparam"),
        _: delimited_with_multispace0("name"),
        _: delimited_with_multispace0("="),
    )
    .context(StrContext::Label("field of class: start opening tag"))
    .context(StrContext::Expected(StrContextValue::Description(
        "e.g. `<hkparam name=`",
    )))
    .parse_next(input)
}

/// Parses the field of class start closing tag `>` or `numelements="0">`
///
/// # Errors
/// If encountered unexpected string.
pub fn field_start_close_tag(input: &mut &str) -> ModalResult<Option<u64>> {
    seq!(
        winnow::combinator::opt(
            seq!(
                _: delimited_with_multispace0("numelements"),
                _: delimited_with_multispace0("="),
                number_in_string::<u64>, // e.g. "8"
            )
        ),
        _: delimited_with_multispace0(">")
    )
    .map(|(n,)| n.map(|n| n.0))
    .context(StrContext::Label("field of class: start closing tag"))
    .context(StrContext::Expected(StrContextValue::Description(
        "e.g. `>`",
    )))
    .parse_next(input)
}

fn find_type(field_name: &str, field_info: &FieldInfo) -> Result<&'static str, Error> {
    find_json_parser_by(field_name, field_info).ok_or_else(|| {
        let acceptable_fields: Vec<&'static str> = field_info.keys().copied().collect();

        Error::UnknownField {
            field_name: field_name.to_string(),
            acceptable_fields,
        }
    })
}

/// Parses the start tag of a field element (`<hkparam name="..." numelements="...">`).
///
/// This parser extracts the field name, type kind, and an optional array length (if specified
/// via `numelements="..."`). It performs type lookup using the provided [`FieldInfo`] and may fail
/// if the type is unknown.
///
/// # Returns
/// A tuple of:
/// - The field name as a string slice
/// - The type kind as a static string
/// - An optional array length (e.g., from `numelements`)
///
/// # Errors
/// Returns a [`ContextError`] if the input does not match the expected structure or if the type cannot be resolved.
pub fn field_start_tag<'a>(
    field_info: &'a FieldInfo,
) -> impl Parser<&'a str, (&'a str, &'static str, Option<u64>), ErrMode<ContextError>> {
    move |input: &mut &'a str| {
        any_field_start_tag
            .try_map(|(field_name, array_len)| {
                find_type(field_name, field_info)
                    .map(|field_type| (field_name, field_type, array_len))
            })
            .parse_next(input)
    }
}

/// Parses the start tag of a field element (`<hkparam name="..." numelements="...">`).
///
/// # Returns
/// A tuple of:
/// - The field name as a string slice
/// - An optional array length (e.g., from `numelements`)
///
/// # Errors
/// Returns a [`ContextError`] if the input does not match the expected structure or if the type cannot be resolved.
#[inline]
pub fn any_field_start_tag<'a>(input: &mut &'a str) -> ModalResult<(&'a str, Option<u64>)> {
    field_start_open_tag.parse_next(input)?; // <hkparam name=
    let field_name = attr_string.parse_next(input)?; // "name"
    let array_len = field_start_close_tag.parse_next(input)?; // > or numelements="">
    Ok((field_name, array_len))
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

// There are support functions that exists only to parse the attributes in the tag.

/// Parses a number inside a string, e.g., `"64"`
///
/// # Errors
/// Parse failed.
fn number_in_string<Num>(input: &mut &str) -> ModalResult<Num>
where
    Num: FromStr,
{
    attr_string
        .parse_to()
        .context(StrContext::Label("number in string"))
        .context(StrContext::Expected(StrContextValue::Description(
            r#"Number(e.g. `"64"`)"#,
        )))
        .parse_next(input)
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PointerType<'a> {
    /// Official representation
    Index(&'a str),
    /// # Nemesis mod representation
    /// Originally, only numbers like #0001 come, but in the case of addition, the mod creator cannot determine the number,
    /// so Nemesis declares it as a variable (e.g. `$id$2`).
    Var(&'a str),
}

/// Parse `"#0001"`, `"#$id$2"`
///
/// # Errors
/// If parsing failed.
fn index_name_attr<'a>(input: &mut &'a str) -> ModalResult<PointerType<'a>> {
    delimited('\"', delimited_multispace0(index_name), '\"').parse_next(input)
}

/// Parse `#0001`, `#$id$2`
///
/// # Errors
/// If parsing failed.
pub fn index_name<'a>(input: &mut &'a str) -> ModalResult<PointerType<'a>> {
    alt((
        ("#", digit1).take().map(PointerType::Index),
        take_until(0.., '\"').map(PointerType::Var),
    ))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index() {
        assert_eq!(
            index_name_attr.parse("\"#0002\""),
            Ok(PointerType::Index("#0002"))
        );
        assert_eq!(
            index_name_attr.parse("\"$id$2\""),
            Ok(PointerType::Var("$id$2"))
        );
    }
}
