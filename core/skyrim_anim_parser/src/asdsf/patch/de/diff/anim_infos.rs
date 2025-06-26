use crate::common_parser::lines::Str;
use json_patch::ValueWithPriority;
use std::collections::HashMap;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AnimInfosDiff<'a> {
    /// - key: replace target index
    /// - value: Partial change request
    pub one: HashMap<usize, AnimInfoDiff<'a>>,

    /// A request to change all elements of an array.
    ///
    /// This is processed after partial patching is complete.
    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "Vec<ValueWithPriority<'a>>: serde::Deserialize<'de>")
        )
    )]
    pub seq: Vec<ValueWithPriority<'a>>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AnimInfoDiff<'a> {
    /// CRC32 representation path
    /// - type: [`u32`]
    pub hashed_path: Option<Str<'a>>,
    /// CRC32 representation file name
    /// - type: [`u32`]
    pub hashed_file_name: Option<Str<'a>>,
    /// Always `7891816`
    /// - type: [`u32`]
    pub ascii_extension: Option<Str<'a>>,
}

// impl<'a> TryFrom<AnimInfoDiff<'a>> for Value<'a> {
//     type Error = DiffCastError;

//     #[inline]
//     fn try_from(value: AnimInfoDiff<'a>) -> Result<Self, Self::Error> {
//         let mut obj = Object::new();

//         let hashed_path = value.hashed_path.ok_or(DiffCastError::MissingField {
//             field: "hashed_path",
//         })?;
//         obj.insert("hashed_path".into(), Value::String(hashed_path.into()));

//         let hashed_file_name = value.hashed_file_name.ok_or(DiffCastError::MissingField {
//             field: "hashed_file_name",
//         })?;
//         obj.insert(
//             "hashed_file_name".into(),
//             Value::String(hashed_file_name.into()),
//         );

//         let ascii_extension = value.ascii_extension.ok_or(DiffCastError::MissingField {
//             field: "ascii_extension",
//         })?;
//         obj.insert(
//             "ascii_extension".into(),
//             Value::String(ascii_extension.into()),
//         );

//         Ok(Value::Object(Box::new(obj)))
//     }
// }
