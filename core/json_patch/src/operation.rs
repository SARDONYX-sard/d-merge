/// Operation for the JSON patch
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Op {
    /// Add a new value to the JSON at the specified path.
    Add,
    /// Remove the value from the JSON at the specified path.
    Remove,
    /// Replace the value at the specified path with a new value.
    Replace,
}
