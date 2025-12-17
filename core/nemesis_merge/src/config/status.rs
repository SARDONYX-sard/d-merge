use core::fmt;

pub type StatusReporterFn = Option<Box<dyn Fn(Status) + Send + Sync>>;

/// An enum representing various statuses during a process.
///
/// This enum is used to track and report the current state of an ongoing process, such as
/// reading templates, applying patches, generating files, or completing the task.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts_serde", serde(tag = "type", content = "content"))]
pub enum Status {
    /// Status when generating FNIS patches.
    GeneratingFnisPatches {
        /// 0 based index
        index: usize,
        total: usize,
    },

    /// Status when reading patches.
    ReadingPatches {
        /// 0 based index
        index: usize,
        total: usize,
    },

    /// Status when Parsing patches.
    ParsingPatches {
        /// 0 based index
        index: usize,
        total: usize,
    },

    /// Status when applying patches.
    ApplyingPatches {
        /// 0 based index
        index: usize,
        total: usize,
    },

    /// Status when generating HKX files.
    GeneratingHkxFiles {
        /// 0 based index
        index: usize,
        total: usize,
    },

    /// Status when the process is completed.
    Done,

    Error(String),
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GeneratingFnisPatches { index, total } => {
                write!(f, "[1/6] Generating FNIS patches...({index}/{total})")
            }
            Self::ReadingPatches { index, total } => {
                write!(f, "[2/6] Reading templates and patches...({index}/{total})")
            }
            Self::ParsingPatches { index, total } => {
                write!(f, "[3/6] Parsing patches...({index}/{total})")
            }
            Self::ApplyingPatches { index, total } => {
                write!(f, "[4/6] Applying patches...({index}/{total})")
            }
            Self::GeneratingHkxFiles { index, total } => {
                write!(f, "[5/6] Generating .hkx files...({index}/{total})")
            }
            Self::Done => write!(f, "[6/6] Done."),
            Self::Error(msg) => write!(f, "[Error] {msg}"),
        }
    }
}

#[cfg(feature = "ts_serde")]
#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "ts_serde")]
    #[test]
    fn serialize_status() {
        let status = Status::Error("Error message".to_string());
        let serialized = simd_json::to_string(&status).unwrap();
        assert_eq!(serialized, r#"{"type":"Error","content":"Error message"}"#);

        let status = Status::ReadingPatches {
            index: 0,
            total: 100,
        };
        let serialized = simd_json::to_string(&status).unwrap();
        assert_eq!(
            serialized,
            r#"{"type":"ReadingPatches","content":{"index":0,"total":100}}"#
        );
    }
}
