mod status;

#[tokio::test]
#[ignore = "local test"]
async fn merge_test() -> Result<(), Box<dyn std::error::Error>> {
    use self::status::*;
    use crate::{PatchMaps, PriorityMap, behavior_gen};
    use rayon::prelude::*;

    std::fs::remove_dir_all("../../dummy/behavior_gen/output")?; // remove prev output

    tracing_rotation::init("../../dummy/behavior_gen", "merge_test.log")?;

    let patches = {
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
            "FNISSexyMove",
            "XPMSE",
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
