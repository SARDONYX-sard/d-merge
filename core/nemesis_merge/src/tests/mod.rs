mod status;

#[tokio::test]
#[ignore = "local test"]
#[cfg(feature = "tracing")]
async fn merge_test() -> Result<(), Box<dyn std::error::Error>> {
    use crate::global_logger::global_logger;
    use crate::tests::status::*;
    use crate::{behavior_gen, PatchMaps};
    use rayon::prelude::*;

    global_logger("../../dummy/merge_test.log", tracing::Level::TRACE)?;

    let mods = {
        use crate::behaviors::PriorityMap;

        let string = std::fs::read_to_string("../../dummy/ids.txt")?;

        let lines: Vec<_> = string
            .par_lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .collect();

        lines
            .into_par_iter()
            .enumerate()
            .map(|(idx, line)| (line.to_string(), idx))
            .collect::<PriorityMap>()
    };

    behavior_gen(
        PatchMaps {
            nemesis_entries: mods,
            ..Default::default()
        },
        slow_debug_config(),
    )
    .await?;
    // behavior_gen(mods, fastest_config()).await?;
    Ok(())
}
