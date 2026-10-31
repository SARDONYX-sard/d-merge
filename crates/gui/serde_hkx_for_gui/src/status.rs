#[derive(Debug, Copy, Clone, serde_repr::Serialize_repr, serde_repr::Deserialize_repr)]
#[repr(u8)]
/// Represents the progress status of a conversion task.
///
/// The numeric representation (`u8`) is serialized and deserialized directly,
/// which is convenient for frontend communication.
pub enum Status {
    /// Task is pending and has not started yet.
    Pending = 0,

    /// Task is currently being processed.
    Processing = 1,

    /// Task completed successfully.
    Done = 2,

    /// Task encountered an error during processing.
    Error = 3,
}

/// Payload for progress reporting
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    /// Hashed identifier of the file path.
    ///
    /// Using a hash ensures that the frontend can track tasks reliably,
    /// even if items are removed or reordered.
    ///
    /// - conversion input path to `djb2` hashed -> id
    pub path_id: u32,

    /// Current progress status of this task.
    pub status: Status,
}
