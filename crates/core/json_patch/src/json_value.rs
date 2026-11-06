//! [`json_value!`](crate::json_value): builds a [`BorrowedValue`](simd_json::BorrowedValue) without
//! copying what can be borrowed.
//!
//! # Why not `simd_json::json_typed!(borrowed, ...)`?
//! With simd-json's `serde_impl` feature, it passes each value through serde, and serde copies each
//! string (`Value::from(value.to_owned())`). A table of Havok classes has hundreds of them, nearly all
//! literals. This macro calls `Value::from` instead, which keeps a `&str` as it is.

/// Builds a [`BorrowedValue`](simd_json::BorrowedValue) from JSON-like syntax. Keys are string literals.
///
/// A value that isn't JSON syntax is an expression, that goes to `Value::from`:
/// - `&'a str` is borrowed. A `String` is moved. A `Vec` of what `Value::from` takes becomes an array.
/// - `&String` isn't accepted: write `text.as_str()` when `text` lives long enough, or `text.clone()`.
///
/// # Example
/// ```
/// use json_patch::json_value;
/// use simd_json::{BorrowedValue, derived::ValueObjectAccess as _};
///
/// let name = "Player_Behavior";
/// let value: BorrowedValue<'_> = json_value!({
///     "__ptr": String::from("#Sample$1"),
///     "name": name,
///     "bindings": [{ "bitIndex": -1 }],
///     "enable": true,
/// });
/// assert_eq!(value.get("name"), Some(&BorrowedValue::from("Player_Behavior")));
/// ```
#[macro_export]
macro_rules! json_value {
    (null) => { $crate::__private::Value::Static($crate::__private::StaticNode::Null) };
    (true) => { $crate::__private::Value::Static($crate::__private::StaticNode::Bool(true)) };
    (false) => { $crate::__private::Value::Static($crate::__private::StaticNode::Bool(false)) };
    ([ $($tt:tt)* ]) => { $crate::json_value!(@array [] $($tt)*) };
    ({ $($tt:tt)* }) => { $crate::json_value!(@object [] $($tt)*) };
    ($other:expr) => { $crate::__private::Value::from($other) };

    // Array: elements are collected in `[...]` until there are no tokens left.
    (@array [$($elements:expr,)*]) => {
        $crate::__private::Value::Array(::std::boxed::Box::new(::std::vec![$($elements),*]))
    };
    // `null` has to be matched here: once parsed as an `expr`, it would no longer match `(null)`.
    (@array [$($elements:expr,)*] null $(, $($rest:tt)*)?) => {
        $crate::json_value!(@array [$($elements,)* $crate::json_value!(null),] $($($rest)*)?)
    };
    (@array [$($elements:expr,)*] [$($inner:tt)*] $(, $($rest:tt)*)?) => {
        $crate::json_value!(@array [$($elements,)* $crate::json_value!([$($inner)*]),] $($($rest)*)?)
    };
    (@array [$($elements:expr,)*] {$($inner:tt)*} $(, $($rest:tt)*)?) => {
        $crate::json_value!(@array [$($elements,)* $crate::json_value!({$($inner)*}),] $($($rest)*)?)
    };
    (@array [$($elements:expr,)*] $next:expr $(, $($rest:tt)*)?) => {
        $crate::json_value!(@array [$($elements,)* $crate::json_value!($next),] $($($rest)*)?)
    };

    // Object: `"key" value` pairs are collected in `[...]`, to know the capacity at the end.
    (@object [$(($key:literal $value:expr))*]) => {{
        #[allow(unused_mut, reason = "an empty object is not written to")]
        let mut object = $crate::__private::Object::with_capacity_and_hasher(
            <[()]>::len(&[$($crate::json_value!(@unit $key)),*]),
            ::std::default::Default::default(),
        );
        $(object.insert(::std::borrow::Cow::Borrowed($key), $value);)*
        $crate::__private::Value::from(object)
    }};
    (@unit $key:literal) => { () };
    (@object [$($entries:tt)*] $key:literal : null $(, $($rest:tt)*)?) => {
        $crate::json_value!(@object [$($entries)* ($key $crate::json_value!(null))] $($($rest)*)?)
    };
    (@object [$($entries:tt)*] $key:literal : [$($inner:tt)*] $(, $($rest:tt)*)?) => {
        $crate::json_value!(@object [$($entries)* ($key $crate::json_value!([$($inner)*]))] $($($rest)*)?)
    };
    (@object [$($entries:tt)*] $key:literal : {$($inner:tt)*} $(, $($rest:tt)*)?) => {
        $crate::json_value!(@object [$($entries)* ($key $crate::json_value!({$($inner)*}))] $($($rest)*)?)
    };
    (@object [$($entries:tt)*] $key:literal : $value:expr $(, $($rest:tt)*)?) => {
        $crate::json_value!(@object [$($entries)* ($key $crate::json_value!($value))] $($($rest)*)?)
    };
}

/// What the macro refers to from other crates, which may have no `simd_json` dependency.
#[doc(hidden)]
pub mod __private {
    pub use simd_json::{
        StaticNode,
        borrowed::{Object, Value},
    };
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use simd_json::{BorrowedValue, StaticNode, derived::ValueObjectAccess as _};

    #[test]
    fn a_literal_is_borrowed_and_not_copied() {
        let value = json_value!({ "name": "literal" });
        let Some(BorrowedValue::String(name)) = value.get("name") else { panic!("{value:?}") };
        assert!(matches!(name, Cow::Borrowed("literal")));
    }

    #[test]
    fn it_builds_the_same_as_simd_json() {
        let moved = String::from("#Sample$1");
        let ours = json_value!({
            "__ptr": moved.clone(),
            "bindings": [
                { "memberPath": "isActive", "variableIndex": 51, "bitIndex": -1 },
                { "memberPath": "other", "variableIndex": 52, "bitIndex": -1 },
            ],
            "empty": [],
            "emptyObject": {},
            "nothing": null,
            "enable": true,
            "disable": false,
            "probability": 1.0,
            "formatted": format!("Player_{}", 1 + 1),
            "nested": [[1, 2], [true, "a"]]
        });
        let theirs = simd_json::json_typed!(borrowed, {
            "__ptr": moved,
            "bindings": [
                { "memberPath": "isActive", "variableIndex": 51, "bitIndex": -1 },
                { "memberPath": "other", "variableIndex": 52, "bitIndex": -1 },
            ],
            "empty": [],
            "emptyObject": {},
            "nothing": null,
            "enable": true,
            "disable": false,
            "probability": 1.0,
            "formatted": format!("Player_{}", 1 + 1),
            "nested": [[1, 2], [true, "a"]]
        });
        assert_eq!(ours, theirs);
    }

    #[test]
    fn numbers_keep_the_type_they_have_through_serde() {
        assert_eq!(json_value!(-1), simd_json::json_typed!(borrowed, -1));
        assert_eq!(json_value!(51), BorrowedValue::Static(StaticNode::I64(51)));
        assert_eq!(json_value!(51_u64), simd_json::json_typed!(borrowed, 51_u64));
        assert_eq!(json_value!(1.5), simd_json::json_typed!(borrowed, 1.5));
    }

    #[test]
    fn the_last_field_and_element_may_have_a_trailing_comma() {
        assert_eq!(json_value!({ "a": 1, }), json_value!({ "a": 1 }));
        assert_eq!(json_value!([1, 2,]), json_value!([1, 2]));
    }
}
