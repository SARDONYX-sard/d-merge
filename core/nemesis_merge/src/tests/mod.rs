mod status;

#[tokio::test]
#[ignore = "local test"]
#[cfg(feature = "tracing")]
async fn merge_test() -> Result<(), Box<dyn std::error::Error>> {
    use crate::behavior_gen;
    use crate::global_logger::global_logger;
    use crate::tests::status::*;
    use rayon::prelude::*;

    global_logger("../../dummy/merge_test.log", tracing::Level::TRACE)?;

    let mods = {
        let paths = std::fs::read_to_string("../../dummy/ids.txt")?;
        paths
            .par_lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .map(Into::into)
            .collect::<Vec<_>>()
    };
    behavior_gen(mods, slow_debug_config()).await?;
    // behavior_gen(mods, fastest_config()).await?;
    Ok(())
}
