mod adsf;
mod behavior;
mod config;
pub mod errors;
mod types;

mod hkx;
mod patches;
mod path_id;
mod results;
mod templates;

pub use crate::behavior::generate::behavior_gen;
pub use crate::config::{Config, Status};
pub use crate::templates::gen_bin::create_bin_templates;
pub use nemesis_xml::hack::HackOptions;

#[cfg(all(feature = "tracing", test))]
pub(crate) mod global_logger;

#[cfg(test)]
mod tests {
    use super::*;
    use rayon::prelude::*;

    #[allow(clippy::iter_on_single_items)]
    const MODS: &[&str] = &[
            // "../../dummy/Data/Nemesis_Engine/mod/aaaaa",
            // "../../dummy/Data/Nemesis_Engine/mod/bcbi",
            // "../../dummy/Data/Nemesis_Engine/mod/cbbi",
            // "../../dummy/Data/Nemesis_Engine/mod/gender",
            // "../../dummy/Data/Nemesis_Engine/mod/hmce",
            // "../../dummy/Data/Nemesis_Engine/mod/momo",
            // "../../dummy/Data/Nemesis_Engine/mod/na1w",
            // "../../dummy/Data/Nemesis_Engine/mod/nemesis",
            // "../../dummy/Data/Nemesis_Engine/mod/pscd",
            // "../../dummy/Data/Nemesis_Engine/mod/rthf",
            // "../../dummy/Data/Nemesis_Engine/mod/skice",
            // "../../dummy/Data/Nemesis_Engine/mod/sscb",
            // "../../dummy/Data/Nemesis_Engine/mod/tkuc",
            // "../../dummy/Data/Nemesis_Engine/mod/tudm",
            // "../../dummy/Data/Nemesis_Engine/mod/turn",
            // "../../dummy/Data/Nemesis_Engine/mod/zcbe",
            // "D:/GAME/ModOrganizer Skyrim SE/mods/FlinchingSSE やられモーションを追加(要OnHit/Nemesis_Engine/mod/flinch",
            "D:/GAME/ModOrganizer Skyrim SE/mods/Crouch Sliding スプリント→しゃがみでスライディング/Nemesis_Engine/mod/slide",
            "D:/GAME/ModOrganizer Skyrim SE/mods/Eating Animations And Sounds SE 歩行しながら食べるモーション/Nemesis_Engine/mod/eaas"
    ];

    #[tokio::test]
    #[ignore = "unimplemented yet"]
    #[cfg(feature = "tracing")]
    async fn merge_test() {
        let log_path = "../../dummy/merge_test.log";
        crate::global_logger::global_logger(log_path, tracing::Level::TRACE).unwrap();

        let _ = behavior_gen(
            MODS.into_par_iter().map(|s| s.into()).collect(),
            Config {
                resource_dir: "../../resource/assets/templates".into(),
                output_dir: "../../dummy/behavior_gen/output".into(),
                status_report: Some(crate::config::new_color_status_reporter()),
                hack_options: Some(crate::HackOptions::enable_all()),
            },
        )
        .await;
    }
}
