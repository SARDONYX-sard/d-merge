pub type JsonPath<'a> = Vec<std::borrow::Cow<'a, str>>;

/// A macro to create a JSON path as a vector of string slices.
///
/// This macro helps construct a `Vec<Cow<'static, str>>` representing a JSON path.
/// It can take zero or more string literals as arguments and builds the path accordingly.
///
/// # Examples
///
/// Creating an empty JSON path:
/// ```
/// use json_patch::json_path;
/// let path = json_path!();
/// assert!(path.is_empty());
/// ```
///
/// Creating a JSON path with multiple components:
/// ```
/// use json_patch::json_path;
/// let path = json_path!("foo", "bar", "baz");
/// assert_eq!(path, vec!["foo", "bar", "baz"]);
/// ```
#[macro_export]
macro_rules! json_path {
    // Match when no arguments are provided.
    () => {
        std::vec::Vec::<std::vec::Vec<std::borrow::Cow<'static, str>>>::new()
    };

    // Match when one or more string literal arguments are provided.
    ( $( $arg:expr ),* $(,)? ) => {
        {
            std::vec![
                $(
                    std::borrow::Cow::Borrowed($arg),
                )*
            ]
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_empty_json_path() {
        let path = json_path!();
        assert!(path.is_empty(), "Expected an empty path");
    }

    #[test]
    fn test_single_component_path() {
        let path = json_path!("foo");
        assert_eq!(path, vec!["foo"]);
    }

    #[test]
    fn test_multiple_component_path() {
        let path = json_path!("foo", "bar", "baz");
        assert_eq!(path, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn test_trailing_comma() {
        let path = json_path!("foo", "bar", "baz",);
        assert_eq!(path, vec!["foo", "bar", "baz"]);
    }
}
