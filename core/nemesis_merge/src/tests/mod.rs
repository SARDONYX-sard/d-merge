pub(crate) mod patches_builder;
mod status;

use crate::tests::patches_builder::{PatchMapsConfig, build_patch_maps};

#[tokio::test]
#[ignore = "local test"]
async fn merge_test() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::remove_dir_all("../../dummy/behavior_gen/output")?; // remove prev output
    tracing_rotation::global::init("../../dummy/behavior_gen", "merge_test.log", 5)?;

    let patches = build_patch_maps(PatchMapsConfig {
        pattern: "D:/GAME/ModOrganizer Skyrim SE/mods/*",
        nemesis_excludes: &[
            "dmco", // Avoid conflicts between SE and AE.
            "tkuc", // This is because TkDodge and RE attempt to add to the same pointer, causing an error.
                   //
        ],
        ..Default::default()
    });
    let config = self::status::slow_debug_config();
    // let config = self::status::fastest_config();

    crate::behavior_gen(patches, config).await?;
    Ok(())
}
