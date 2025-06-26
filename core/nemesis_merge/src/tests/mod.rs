mod global_logger;
mod status;

#[tokio::test]
#[ignore = "local test"]
#[cfg(feature = "tracing")]
async fn merge_test() -> Result<(), Box<dyn std::error::Error>> {
    use crate::behavior_gen;
    use crate::tests::{global_logger::global_logger, status::*};

    global_logger("../../dummy/merge_test.log", tracing::Level::TRACE)?;

    let mods = {
        let paths = std::fs::read_to_string("../../dummy/ids.txt")?;
        paths.split("\n").map(Into::into).collect()
    };
    behavior_gen(mods, slow_debug_config()).await?;
    // behavior_gen(mods, fastest_config()).await?;
    Ok(())
}
