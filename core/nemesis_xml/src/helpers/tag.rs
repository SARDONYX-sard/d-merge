use super::delimited_multispace0;
use serde_hkx::xml::de::parser::{delimited_with_multispace0, tag::attr_string};
use std::str::FromStr;
use winnow::{
    ascii::digit1,
    combinator::{alt, delimited, preceded, seq},
    error::{
        ContextError, StrContext,
        StrContextValue::{self},
    },
    token::take_until,
    PResult, Parser,
};

/// Parses the start tag `<tag>`
///
/// - `tag`: e.g. `<string>`
pub fn start_tag<'a>(tag: &'static str) -> impl Parser<&'a str, (), ContextError> {
    seq!(
        _: delimited_with_multispace0("<"),
        _: delimited_with_multispace0(tag),
        _: delimited_with_multispace0(">")
    )
    .context(StrContext::Label("start tag"))
    .context(StrContext::Label(tag))
}

/// Parses the end tag `</tag>`
pub fn end_tag<'a>(tag: &'static str) -> impl Parser<&'a str, (), ContextError> {
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
pub fn class_start_tag<'a>(input: &mut &'a str) -> PResult<(PointerType<'a>, &'a str)> {
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
pub fn field_start_open_tag(input: &mut &str) -> PResult<()> {
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
pub fn field_start_close_tag(input: &mut &str) -> PResult<Option<u64>> {
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

/// # Errors
/// Parse failed.
pub fn field_start_tag<'a>(input: &mut &'a str) -> PResult<(&'a str, Option<u64>)> {
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
fn number_in_string<Num>(input: &mut &str) -> PResult<Num>
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
fn index_name_attr<'a>(input: &mut &'a str) -> PResult<PointerType<'a>> {
    delimited('\"', delimited_multispace0(index_name), '\"').parse_next(input)
}

/// Parse `#0001`, `#$id$2`
///
/// # Errors
/// If parsing failed.
pub fn index_name<'a>(input: &mut &'a str) -> PResult<PointerType<'a>> {
    preceded(
        "#",
        alt((
            digit1.map(PointerType::Index),
            take_until(0.., '\"').map(PointerType::Var),
        )),
    )
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index() {
        assert_eq!(
            index_name_attr.parse("\"#0002\""),
            Ok(PointerType::Index("0002"))
        );
        assert_eq!(
            index_name_attr.parse("\"#$id$2\""),
            Ok(PointerType::Var("$id$2"))
        );
    }
}
