//! Error types for `tracing-rotation`.

use std::io;

use snafu::Snafu;
use tracing_subscriber::reload;

/// Alias for `std::result::Result<T, Error>`.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// All errors that can be produced by this crate.
#[derive(Debug, Snafu)]
pub enum Error {
    /// An I/O error while creating, renaming, or deleting log files.
    #[snafu(transparent)]
    Io { source: io::Error },

    /// The reload handle's subscriber was dropped before the modify call.
    #[snafu(display("failed to reload log level: {source}"))]
    Reload { source: reload::Error },

    /// The `SwappableWriter`'s mutex was poisoned.
    #[snafu(display("internal log writer lock is poisoned"))]
    LockPoisoned,

    /// [`global::init`](crate::global::init) was called more than once.
    #[snafu(display("global logger is already initialized"))]
    AlreadyInit,

    /// A [`global`](crate::global) function was called before
    /// [`global::init`](crate::global::init).
    #[snafu(display("global logger has not been initialized yet"))]
    NotInit,
}
