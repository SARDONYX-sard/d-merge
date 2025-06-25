mod global_logger;
mod status;

#[allow(unused)]
#[allow(clippy::iter_on_single_items)]
const MODS: &[&str] = &[
    "D:/GAME/ModOrganizer Skyrim SE/mods/Thu'um - Animated Shouts シャウト時のアニメ変更/Nemesis_engine/mod/thuum",
];

#[tokio::test]
#[ignore = "local test"]
#[cfg(feature = "tracing")]
async fn merge_test() {
    use crate::tests::global_logger::global_logger;
    use crate::tests::status::new_color_status_reporter;
    use crate::{behavior_gen, Config, DebugOptions, HackOptions, OutPutTarget};
    // use rayon::prelude::*;

    let log_path = "../../dummy/merge_test.log";
    global_logger(log_path, tracing::Level::TRACE).unwrap();

    let paths = std::fs::read_to_string("../../dummy/ids.txt").unwrap();
    // let mods = MODS.into_par_iter().map(|s| s.into()).collect();
    let mods = paths.split("\n").map(Into::into).collect();
    let _ = behavior_gen(
        mods,
        Config {
            resource_dir: "../../resource/assets/templates".into(),
            output_dir: "../../dummy/behavior_gen/output".into(),
            status_report: Some(new_color_status_reporter()), // +2s
            // status_report: None,
            hack_options: Some(HackOptions::enable_all()),
            debug: DebugOptions {
                output_patch_json: true,
                output_merged_json: true,
                output_merged_xml: false,
            },
            output_target: OutPutTarget::SkyrimSe,
        },
    )
    .await;
}
