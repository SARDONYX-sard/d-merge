mod global_logger;
mod status;

#[tokio::test]
#[ignore = "local test"]
#[cfg(feature = "tracing")]
async fn merge_test() -> Result<(), Box<dyn std::error::Error>> {
    use crate::tests::{global_logger::global_logger, status::*};
    use crate::{behavior_gen, PatchMaps};
    use rayon::prelude::*;

    global_logger("../../dummy/merge_test.log", tracing::Level::TRACE)?;

    let patches = {
        use crate::behaviors::PriorityMap;

        let string = std::fs::read_to_string("../../dummy/ids.ini")?;

        let lines: Vec<_> = string
            .par_lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with(";"))
            .collect();
        let nemesis_mods_count = lines.len();

        let nemesis_entries = lines
            .into_par_iter()
            .enumerate()
            .map(|(idx, line)| (line.to_string(), idx))
            .collect::<PriorityMap>();

        let fnis_entries = [
            "FNISBase",
            "FNISCreatureVersion",
            "FNISZoo",
            "P1FlyingRing",
            // "XPMSE",
            "backgrab",
            "backgrabnosneak",
            "backstabnosneak",
            "cbbackcutandkick",
            "cbbackcutandkicknosneak",
            "cbbackstompsneak",
            "cbbackuppercutandslash",
            "Fightcb4kickandkill",
            "cbstabback",
            "cbstompback1",
            "fightcbsitkick",
            "Slidekill",
            "fightcbstamp",
            "frontgrab",
            "newfightcb1",
        ]
        .into_par_iter()
        .enumerate()
        .map(|(idx, namespace)| (namespace.to_string(), nemesis_mods_count + idx))
        .collect();

        PatchMaps {
            nemesis_entries,
            fnis_entries,
        }
    };

    behavior_gen(patches, slow_debug_config()).await?;
    // behavior_gen(mods, fastest_config()).await?;
    Ok(())
}
