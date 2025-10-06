// TODO: This should be separated into a different crate. Otherwise, if the generated Rust code encounters a compilation error,
// it won't even be able to output anything.
#[cfg(test)]
mod tests {
    #[derive(Debug, serde::Deserialize)]
    pub struct BehaviorEntry {
        pub behavior_object: String,
        pub base_folder: String,
        pub default_behavior: String,
        pub default_behavior_index: String,
        pub master_behavior: String,
        pub master_behavior_index: String,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct Root {
        pub creatures: Vec<BehaviorEntry>,
        pub skeletons: Vec<BehaviorEntry>,
        pub auxbones: Vec<BehaviorEntry>,
        #[serde(rename = "plants/activators")]
        pub plants_activators: Vec<BehaviorEntry>,
    }

    /// phf_map!
    fn generate_map_code(category: &str, entries: &[BehaviorEntry]) -> String {
        let static_name = category.to_ascii_uppercase().replace('/', "_");

        let mut map_entries = String::new();
        for entry in entries {
            map_entries.push_str(&format!(
                r###"    "{bo}" => BehaviorEntry {{
        behavior_object: "{bo}",
        base_dir: "{bf}",
        default_behavior: "{db}",
        default_behavior_index: "{dbi}",
        master_behavior: "{mb}",
        master_behavior_index: "{mbi}",
    }},
"###,
                bo = entry.behavior_object,
                bf = entry.base_folder,
                db = entry.default_behavior,
                dbi = entry.default_behavior_index,
                mb = entry.master_behavior,
                mbi = entry.master_behavior_index,
            ));
        }

        format!(
            r###"
pub static {name}: phf::Map<&'static str, BehaviorEntry> = phf::phf_map! {{
{entries}}};
"###,
            name = static_name,
            entries = map_entries
        )
    }

    #[test]
    #[ignore = "local only"]
    fn generate_const_behavior_table() -> Result<(), Box<dyn std::error::Error>> {
        use std::fs::File;
        use std::io::Write as _;
        use std::path::Path;

        let mut data =
            std::fs::read_to_string("../../dummy/debug/FNIS_output/behaviors_table.json")?;
        let root: Root = simd_json::from_slice(unsafe { data.as_bytes_mut() })?;

        let rs_code = r###"// This is an automatically generated template. Do not edit it.
use crate::behaviors::tasks::templates::key::TemplateKey;
use std::borrow::Cow;

/// `meshes/<base_dir>/<default_behavior/master_behavior>`
///
/// # Notes
/// - According to FNIS debug output, if you register an animation path for the Creature `draugr`, you must also register the same animation in `draugrskelton.hkx`.
/// - CREATURE and SKELETON keys overlap only for `draugr`.
///
/// - For characters, in addition to `default_behavior: "characters/defaultmale.hkx"`, register the same animation path in `characters/female/defaultfemale.hkx`.
///
/// # Intend
/// By joining these and using `/` as the path separator, the `TemplateKey` is intended to be generated.
/// This integrates FNIS Patch Maps into Nemesis patchMaps.
#[derive(Debug)]
pub struct BehaviorEntry {
    /// e.g. `character`
    pub behavior_object: &'static str,
    /// e.g. `actors/character`
    pub base_dir: &'static str,

    /// Animation path registered target template path (e.g. `character/firstperson.bin`)
    ///
    /// # Note
    /// When writing, don't forget to use `set_extension("hkx")`.
    pub default_behavior: &'static str,
    /// Animation path registered target(`hkbCharacterStringData`) XML index e.g. `#0029`
    pub default_behavior_index: &'static str,

    /// Mod root behavior registered target template path (e.g. `behaviors/0_master.bin`)
    ///
    /// # Note
    /// When writing, don't forget to use `set_extension("hkx")`.
    pub master_behavior: &'static str,
    /// Mod root behavior registered target(`hkbStateMachine`) XML index e.g. `#0340`
    pub master_behavior_index: &'static str,
}

impl BehaviorEntry {
    /// Generate TemplateKey from default_behavior
    /// - e.g. `meshes/actors/character/characters/defaultmale.bin`
    ///
    /// FNIS registers the animation paths specified in FNIS_*_List.txt in this template.
    pub fn to_default_behavior_template_key(&self) -> TemplateKey<'static> {
        let path = format!("meshes/{}/{}", self.base_dir, self.default_behavior);
        // Safety: caller guarantees the path is a valid TemplateKey
        unsafe { TemplateKey::new_unchecked(Cow::Owned(path)) }
    }

    /// Generate TemplateKey from master_behavior
    /// - e.g. `meshes/actors/character/behaviors/0_master.bin`
    ///
    /// FNIS registers the Mod Root behavior in this template.
    pub fn to_master_behavior_template_key(&self) -> TemplateKey<'static> {
        let path = format!("meshes/{}/{}", self.base_dir, self.master_behavior);
        // Safety: caller guarantees the path is a valid TemplateKey
        unsafe { TemplateKey::new_unchecked(Cow::Owned(path)) }
    }

    /// Is humanoid patch.
    ///
    /// # Usage
    /// Regarding characters, in addition to `default_behavior: "characters/defaultmale.hkx"`,
    /// the same animation path must also be registered in `characters/female/defaultfemale.hkx`.
    /// This is the condition check for that purpose.
    pub fn is_humanoid(&self) -> bool {
        HUMANOID.contains_key(self.behavior_object)
    }
}

pub static HUMANOID: phf::Map<&'static str, BehaviorEntry> = phf::phf_map! {
    "character/_1stperson" => BehaviorEntry {
        behavior_object: "character/_1stperson",
        base_dir: "actors/character/_1stperson",
        default_behavior: "characters/firstperson.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/0_master.bin",
        master_behavior_index: "#0167",
    },
    "character" => BehaviorEntry {
        behavior_object: "character",
        base_dir: "actors/character",
        default_behavior: "characters/defaultmale.bin", // & characters female/defaultfemale.bin"
        default_behavior_index: "#0029", // defaultmale.xml & defaultfemale.xml same index
        master_behavior: "behaviors/0_master.bin",
        // Basically, hkRootLevelContainer.m_namedVariants[0] -> hkbBehaviorGraph.m_rootGenerator
        // However, for some reason, only the humanoid 0_master seems to push to a different index.
        master_behavior_index: "#0340",
    },
};

/// # Usage
/// Regarding characters, in addition to `default_behavior: "characters/defaultmale.hkx"`,
/// the same animation path must also be registered in `characters/female/defaultfemale.hkx`.
pub const DEFAULT_FEMALE: BehaviorEntry = BehaviorEntry {
    behavior_object: "character",
    base_dir: "actors/character",
    default_behavior: "characters female/defaultfemale.bin",
    default_behavior_index: "#0029", // defaultmale.xml & defaultfemale.xml same index
    master_behavior: "behaviors/0_master.bin",
    master_behavior_index: "#2521",
};

/// # Why need this?
/// It seems draugr must have the animations path added to both draugr.xml and
/// draugr_skeleton.xml (information from the FNIS Creature pack's behavior object).
pub const DRAUGR_SKELETON: BehaviorEntry = BehaviorEntry {
    behavior_object: "draugr",
    base_dir: "actors/draugr",
    default_behavior: "characterskeleton/draugr_skeleton.bin",
    default_behavior_index: "#0024",
    master_behavior: "behaviors/draugrbehavior.bin",
    master_behavior_index: "#2026",
};
"###
        .to_string();

        let mut rs_code = rs_code;
        rs_code.push_str(&generate_map_code("creatures", &root.creatures));
        rs_code.push_str(&generate_map_code("skeletons", &root.skeletons));
        rs_code.push_str(&generate_map_code("auxbones", &root.auxbones));
        rs_code.push_str(&generate_map_code(
            "plants/activators",
            &root.plants_activators,
        ));

        let path = Path::new("./src/behaviors/tasks/fnis/patch_gen/generated_behaviors.rs");
        let mut f = File::create(path)?;
        f.write_all(rs_code.as_bytes())?;

        println!("generated: {}", path.display());
        Ok(())
    }
}
