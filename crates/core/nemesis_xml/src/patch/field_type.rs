//! What the patch parser needs to know about a field to read its value.

use super::class_schema::{FieldSchema, TypeKind};

/// A field's type: how to read its value (see `PatchDeserializer::parse_value`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldType {
    pub kind: TypeKind,
    /// Element type of an array.
    pub sub_kind: TypeKind,
    /// Class of a struct (or of the elements of an array of structs).
    pub class: Option<&'static str>,
}

impl FieldType {
    /// A struct field of `class`, e.g. `hkbEventProperty`.
    pub const fn object(class: &'static str) -> Self {
        Self { kind: TypeKind::Struct, sub_kind: TypeKind::Void, class: Some(class) }
    }

    /// A single pointer.
    pub const POINTER: Self =
        Self { kind: TypeKind::Pointer, sub_kind: TypeKind::Void, class: None };

    /// The type of one element of this array.
    pub const fn element(self) -> Self {
        Self { kind: self.sub_kind, sub_kind: TypeKind::Void, class: self.class }
    }

    /// Read as a string (`<hkcstring>` in arrays): text, enums and flags.
    pub const fn is_string(self) -> bool {
        matches!(
            self.kind,
            TypeKind::Char
                | TypeKind::CString
                | TypeKind::StringPtr
                | TypeKind::Enum
                | TypeKind::Flags
        )
    }
}

impl From<&FieldSchema> for FieldType {
    fn from(field: &FieldSchema) -> Self {
        Self { kind: field.kind, sub_kind: field.sub_kind, class: field.class }
    }
}
