use winnow::ascii::Caseless;
use winnow::combinator::{alt, delimited};
use winnow::{ModalResult, Parser};

use crate::helpers::tag::{field_start_close_tag, field_start_open_tag};

/// Parser hack for misnamed or invalid fields in the `BSRagdollContactListenerModifier` class.
///
/// This hack corrects known field name issues that appear in some community-created patches:
/// - If `<hkparam name="event">` is found inside the `BSRagdollContactListenerModifier` class,
///   it is treated as `contactEvent` of type `Object|hkbEventProperty`.
/// - If `<hkparam name="anotherBoneIndex">` is encountered, it is replaced with `bones` of type `Pointer`.
///
/// These substitutions help ensure compatibility with patches that use invalid or outdated field names,
/// even though those fields do not exist in the official format.
///
/// This function parses the start of a `<hkparam>` tag and returns corrected field metadata as a tuple:
/// - field name (corrected)
/// - type string
/// - array length (None for singular fields)
///
/// # Returns
/// A tuple representing the fixed field metadata.
///
/// # Errors
/// Returns an error if the expected malformed field tag is not present at the current input.
pub fn do_hack_cast_ragdoll_event<'a>(
    input: &mut &'a str,
) -> ModalResult<(&'a str, &'static str, Option<u64>)> {
    field_start_open_tag.parse_next(input)?; // <hkparam name=
    let (field_name, field_type) = delimited(
        "\"",
        alt((
            Caseless("event").map(|_| ("contactEvent", "Object|hkbEventProperty")),
            Caseless("anotherBoneIndex").map(|_| ("bones", "Pointer")),
        )),
        "\"",
    )
    .parse_next(input)?; // "name"
    let array_len = field_start_close_tag.parse_next(input)?; // > or numelements="">

    Ok((field_name, field_type, array_len)) // `contactEvent` isn't array(So use `None`)
}
