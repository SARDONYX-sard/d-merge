use snafu::Snafu;

#[derive(Snafu, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(clippy::enum_variant_names)]
pub enum RangeError {
    /// Index out of bounds: Expected `1 <= index <= {len}`, but got `{index}`
    IndexOutOfBounds { index: usize, len: usize },

    /// Start index out of bounds: Expected `0 <= start < {len}`, but got `{start}`
    StartOutOfBounds { start: usize, len: usize },

    /// End index out of bounds: Expected `0 <= end <= {len}`, but got `{end}`
    EndOutOfBounds { end: usize, len: usize },

    /// `FromTo` range out of bounds: Expected `0 <= start < end <= {len}`, but got `[start..end]` where `start = {start}` and `end = {end}`
    FromToOutOfBounds {
        start: usize,
        end: usize,
        len: usize,
    },
}
