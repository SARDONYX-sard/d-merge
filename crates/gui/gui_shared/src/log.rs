//! Start a log file watcher thread.
//!
//! This continuously updates `log_lines` with the latest contents of the log file.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::Warn => "Warn",
            Self::Info => "Info",
            Self::Debug => "Debug",
            Self::Trace => "Trace",
        }
    }

    #[inline]
    pub const fn all() -> [Self; 5] {
        [Self::Error, Self::Warn, Self::Info, Self::Debug, Self::Trace]
    }
}

impl core::fmt::Display for LogLevel {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl core::str::FromStr for LogLevel {
    type Err = String;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            _ if s.eq_ignore_ascii_case("error") => Self::Error,
            _ if s.eq_ignore_ascii_case("warn") => Self::Warn,
            _ if s.eq_ignore_ascii_case("info") => Self::Info,
            _ if s.eq_ignore_ascii_case("debug") => Self::Debug,
            _ if s.eq_ignore_ascii_case("trace") => Self::Trace,
            unknown => return Err(format!("Invalid log level: {unknown}")),
        })
    }
}

impl From<LogLevel> for String {
    #[inline]
    fn from(value: LogLevel) -> Self {
        value.as_str().to_string()
    }
}

#[cfg(feature = "tracing")]
impl From<LogLevel> for tracing::Level {
    #[inline]
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => Self::ERROR,
            LogLevel::Warn => Self::WARN,
            LogLevel::Info => Self::INFO,
            LogLevel::Debug => Self::DEBUG,
            LogLevel::Trace => Self::TRACE,
        }
    }
}

pub const LOG_DIR: &str = ".d_merge/logs";
pub const LOG_FILENAME: &str = "d_merge.log";
