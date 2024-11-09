/// Custom error type for JSON patch operations.
#[derive(Debug, snafu::Snafu)]
pub enum PatchError {
    /// Error indicating that the specified path was not found in the JSON structure.
    #[snafu(display("Path not found: {}", path))]
    PathNotFound { path: String },

    /// Error indicating an invalid operation at the given path.
    #[snafu(display("Invalid operation for path: {}", path))]
    InvalidOperation { path: String },

    #[snafu(transparent)]
    AccessError { source: simd_json::AccessError },

    #[snafu(transparent)]
    TryTypeError { source: simd_json::TryTypeError },

    #[snafu(transparent)]
    SearchedError {
        source: crate::searcher::error::Error,
    },
}

/// Result type alias for JSON patch operations.
pub type Result<T, E = PatchError> = std::result::Result<T, E>;
