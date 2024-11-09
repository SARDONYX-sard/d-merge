use simd_json::TryTypeError;

#[derive(snafu::Snafu, Debug, Clone)]
pub enum Error {
    #[snafu(display("Pointer is empty, cannot add"))]
    EmptyPointer,

    #[snafu(display("Invalid index: {}", index))]
    InvalidIndex { index: String },

    #[snafu(display("Cannot go deeper in a String"))]
    InvalidString,

    /// Can't go deeper in a static node
    InvalidTarget,

    #[snafu(transparent)]
    TryType { source: TryTypeError },
}
