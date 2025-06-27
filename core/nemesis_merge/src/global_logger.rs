use std::{fs::File, path::Path};

use tracing::{level_filters::LevelFilter, Level};
use tracing_subscriber::{fmt, layer::SubscriberExt as _, reload, util::SubscriberInitExt as _};

/// Init tracing as global
///
/// # Errors
/// logging failed.
pub fn global_logger<I, P>(path: P, level: I) -> std::io::Result<()>
where
    I: Into<Level>,
    P: AsRef<Path>,
{
    // Unable `pretty()` & `with_ansi(false)` combination in `#[tracing::instrument]`
    // ref: https://github.com/tokio-rs/tracing/issues/1310
    let fmt_layer = fmt::layer()
        .compact()
        .with_ansi(false)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_writer(File::create(path)?);

    let (filter, _reload_handle) = reload::Layer::new(LevelFilter::from_level(level.into()));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    Ok(())
}
