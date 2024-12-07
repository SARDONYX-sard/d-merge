#[derive(Debug, Clone, serde_repr::Serialize_repr, serde_repr::Deserialize_repr)]
#[repr(u8)]
pub(super) enum Status {
    Pending = 0,
    Processing = 1,
    Done = 2,
    Error = 3,
}

/// # Progress report for progress bar
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Payload {
    /// hashed path
    pub(super) path_id: u32,
    /// Current progress status
    pub(super) status: Status,
}
