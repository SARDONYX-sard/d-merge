#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("Failed to read file: {}", path.display()))]
    ReadFile { path: std::path::PathBuf, source: std::io::Error },

    #[snafu(display("Failed to write file: {}", path.display()))]
    WriteFile { path: std::path::PathBuf, source: std::io::Error },

    #[snafu(display("Failed to parse json: {}", path.display()))]
    ParseJson { path: std::path::PathBuf, source: sonic_rs::Error },

    #[snafu(display("Failed to serialize json: {}", path.display()))]
    SerializeJson { path: std::path::PathBuf, source: sonic_rs::Error },
}
