// This is an automatically generated template. Do not edit it.
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
    /// When writing, don't forget to use `set_extension(“hkx”)`.
    pub default_behavior: &'static str,
    /// Animation path registered target(`hkbCharacterStringData`) XML index e.g. `#0029`
    pub default_behavior_index: &'static str,

    /// Mod root behavior registered target template path (e.g. `behaviors/0_master.bin`)
    ///
    /// # Note
    /// When writing, don't forget to use `set_extension(“hkx”)`.
    pub master_behavior: &'static str,
    /// Mod root behavior registered target(`hkbStateMachine`) XML index e.g. `#1831`
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
        master_behavior_index: "#1831",
    },
    "character" => BehaviorEntry {
        behavior_object: "character",
        base_dir: "actors/character",
        default_behavior: "characters/defaultmale.bin", // & characters female/defaultfemale.bin"
        default_behavior_index: "#0029",
        master_behavior: "behaviors/0_master.bin",
        master_behavior_index: "#1831",
    },
};

pub static CREATURES: phf::Map<&'static str, BehaviorEntry> = phf::phf_map! {
    "atronachflame" => BehaviorEntry {
        behavior_object: "atronachflame",
        base_dir: "actors/atronachflame",
        default_behavior: "characters/atronachflame.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/atronachflamebehavior.bin",
        master_behavior_index: "#0438",
    },
    "atronachfrost" => BehaviorEntry {
        behavior_object: "atronachfrost",
        base_dir: "actors/atronachfrost",
        default_behavior: "characters/atronachfrostcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/atronachfrostbehavior.bin",
        master_behavior_index: "#0451",
    },
    "atronachstorm" => BehaviorEntry {
        behavior_object: "atronachstorm",
        base_dir: "actors/atronachstorm",
        default_behavior: "characters/atronachstormcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/atronachstormbehavior.bin",
        master_behavior_index: "#0384",
    },
    "bear" => BehaviorEntry {
        behavior_object: "bear",
        base_dir: "actors/bear",
        default_behavior: "characters/bearcharacter.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/bearbehavior.bin",
        master_behavior_index: "#0151",
    },
    "benthiclurker" => BehaviorEntry {
        behavior_object: "benthiclurker",
        base_dir: "actors/dlc02/benthiclurker",
        default_behavior: "characters/benthiclurkercharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/benthiclurkerbehavior.bin",
        master_behavior_index: "#0733",
    },
    "boarriekling" => BehaviorEntry {
        behavior_object: "boarriekling",
        base_dir: "actors/dlc02/boarriekling",
        default_behavior: "characters/boar.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/boarbehavior.bin",
        master_behavior_index: "#0584",
    },
    "chaurus" => BehaviorEntry {
        behavior_object: "chaurus",
        base_dir: "actors/chaurus",
        default_behavior: "characters/chaurus.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/chaurusbehavior.bin",
        master_behavior_index: "#0509",
    },
    "chaurusflyer" => BehaviorEntry {
        behavior_object: "chaurusflyer",
        base_dir: "actors/dlc01/chaurusflyer",
        default_behavior: "characters/chaurusflyercharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/chaurusflyerbehavior.bin",
        master_behavior_index: "#0406",
    },
    "chicken" => BehaviorEntry {
        behavior_object: "chicken",
        base_dir: "actors/ambient/chicken",
        default_behavior: "characters/chickencharater.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/chickenbehavior.bin",
        master_behavior_index: "#0332",
    },
    "cow" => BehaviorEntry {
        behavior_object: "cow",
        base_dir: "actors/cow",
        default_behavior: "characters/h_cowcharater.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/h-cowbehavior.bin",
        master_behavior_index: "#0152",
    },
    "deer" => BehaviorEntry {
        behavior_object: "deer",
        base_dir: "actors/deer",
        default_behavior: "characters/deercharater.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/deerbehavior.bin",
        master_behavior_index: "#0145",
    },
    "dog" => BehaviorEntry {
        behavior_object: "dog",
        base_dir: "actors/canine",
        default_behavior: "characters dog/dog.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/dogbehavior.bin",
        master_behavior_index: "#0144",
    },
    "dragon" => BehaviorEntry {
        behavior_object: "dragon",
        base_dir: "actors/dragon",
        default_behavior: "characters/dragontest.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/dragonbehavior.bin",
        master_behavior_index: "#1553",
    },
    "dragonpriest" => BehaviorEntry {
        behavior_object: "dragonpriest",
        base_dir: "actors/dragonpriest",
        default_behavior: "characters/dragon_priest.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/dragon_priest.bin",
        master_behavior_index: "#0796",
    },
    "draugr" => BehaviorEntry {
        behavior_object: "draugr",
        base_dir: "actors/draugr",
        default_behavior: "characters/draugr.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/draugrbehavior.bin",
        master_behavior_index: "#2026",
    },
};

pub static SKELETONS: phf::Map<&'static str, BehaviorEntry> = phf::phf_map! {
    "draugr" => BehaviorEntry {
        behavior_object: "draugr",
        base_dir: "actors/draugr",
        default_behavior: "characterskeleton/draugr_skeleton.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/draugrbehavior.bin",
        master_behavior_index: "#2026",
    },
    "dwarvenballistacenturion" => BehaviorEntry {
        behavior_object: "dwarvenballistacenturion",
        base_dir: "actors/dlc02/dwarvenballistacenturion",
        default_behavior: "characters/ballistacenturion.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/bcbehavior.bin",
        master_behavior_index: "#0492",
    },
    "dwarvenspherecenturion" => BehaviorEntry {
        behavior_object: "dwarvenspherecenturion",
        base_dir: "actors/dwarvenspherecenturion",
        default_behavior: "characters/spherecenturion.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/scbehavior.bin",
        master_behavior_index: "#0797",
    },
    "dwarvenspider" => BehaviorEntry {
        behavior_object: "dwarvenspider",
        base_dir: "actors/dwarvenspider",
        default_behavior: "characters/dwarvenspidercenturion.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/dwarvenspiderbehavior.bin",
        master_behavior_index: "#0404",
    },
    "dwarvensteamcenturion" => BehaviorEntry {
        behavior_object: "dwarvensteamcenturion",
        base_dir: "actors/dwarvensteamcenturion",
        default_behavior: "characters/dwarvensteam.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/steambehavior.bin",
        master_behavior_index: "#0552",
    },
    "falmer" => BehaviorEntry {
        behavior_object: "falmer",
        base_dir: "actors/falmer",
        default_behavior: "characters/falmer.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/falmerbehavior.bin",
        master_behavior_index: "#1314",
    },
    "frostbitespider" => BehaviorEntry {
        behavior_object: "frostbitespider",
        base_dir: "actors/frostbitespider",
        default_behavior: "characters/frostbitespidercharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/frostbitespiderbehavior.bin",
        master_behavior_index: "#0412",
    },
    "giant" => BehaviorEntry {
        behavior_object: "giant",
        base_dir: "actors/giant",
        default_behavior: "characters/giantcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/giantbehavior.bin",
        master_behavior_index: "#0822",
    },
    "goat" => BehaviorEntry {
        behavior_object: "goat",
        base_dir: "actors/goat",
        default_behavior: "characters/goatcharater.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/goatbehavior.bin",
        master_behavior_index: "#0140",
    },
    "hagraven" => BehaviorEntry {
        behavior_object: "hagraven",
        base_dir: "actors/hagraven",
        default_behavior: "characters/hagravencharacter.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/havgravenbehavior.bin",
        master_behavior_index: "#0634",
    },
    "hare" => BehaviorEntry {
        behavior_object: "hare",
        base_dir: "actors/ambient/hare",
        default_behavior: "characters/harecharater.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/harebehavior.bin",
        master_behavior_index: "#0309",
    },
    "hmdaedra" => BehaviorEntry {
        behavior_object: "hmdaedra",
        base_dir: "actors/dlc02/hmdaedra",
        default_behavior: "characters/hmdaedracharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/hmdaedra.bin",
        master_behavior_index: "#0504",
    },
    "horker" => BehaviorEntry {
        behavior_object: "horker",
        base_dir: "actors/horker",
        default_behavior: "characters/horker.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/horkerbehavior.bin",
        master_behavior_index: "#0161",
    },
    "horse" => BehaviorEntry {
        behavior_object: "horse",
        base_dir: "actors/horse",
        default_behavior: "characters/horse.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/horsebehavior.bin",
        master_behavior_index: "#0537",
    },
    "icewraith" => BehaviorEntry {
        behavior_object: "icewraith",
        base_dir: "actors/icewraith",
        default_behavior: "characters/icewraithcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/icewraithbehavior.bin",
        master_behavior_index: "#0266",
    },
    "mammoth" => BehaviorEntry {
        behavior_object: "mammoth",
        base_dir: "actors/mammoth",
        default_behavior: "characters/mammothcharacter.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/mammothbehavior.bin",
        master_behavior_index: "#0155",
    },
    "mudcrab" => BehaviorEntry {
        behavior_object: "mudcrab",
        base_dir: "actors/mudcrab",
        default_behavior: "characters/mudcrabcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/mudcrabbehavior.bin",
        master_behavior_index: "#0495",
    },
    "netch" => BehaviorEntry {
        behavior_object: "netch",
        base_dir: "actors/dlc02/netch",
        default_behavior: "characters/netchcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/netchbehavior.bin",
        master_behavior_index: "#0279",
    },
    "riekling" => BehaviorEntry {
        behavior_object: "riekling",
        base_dir: "actors/dlc02/riekling",
        default_behavior: "characters/rieklingcharacter.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/rieklingbehavior.bin",
        master_behavior_index: "#0769",
    },
    "sabrecat" => BehaviorEntry {
        behavior_object: "sabrecat",
        base_dir: "actors/sabrecat",
        default_behavior: "characters/sabrecat.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/sabrecatbehavior.bin",
        master_behavior_index: "#0140",
    },
    "scrib" => BehaviorEntry {
        behavior_object: "scrib",
        base_dir: "actors/dlc02/scrib",
        default_behavior: "characters/scrib.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/scribbehavior.bin",
        master_behavior_index: "#0605",
    },
    "skeever" => BehaviorEntry {
        behavior_object: "skeever",
        base_dir: "actors/skeever",
        default_behavior: "characters/skeevercharacter.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/skeeverbehavior.bin",
        master_behavior_index: "#0132",
    },
    "slaughterfish" => BehaviorEntry {
        behavior_object: "slaughterfish",
        base_dir: "actors/slaughterfish",
        default_behavior: "characters/slaughterfish.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/slaughterfishbehavior.bin",
        master_behavior_index: "#0286",
    },
    "spriggan" => BehaviorEntry {
        behavior_object: "spriggan",
        base_dir: "actors/spriggan",
        default_behavior: "characters/spriggan.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/sprigganbehavior.bin",
        master_behavior_index: "#0635",
    },
    "troll" => BehaviorEntry {
        behavior_object: "troll",
        base_dir: "actors/troll",
        default_behavior: "characters/troll.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/trollbehavior.bin",
        master_behavior_index: "#0724",
    },
    "vampirebrute" => BehaviorEntry {
        behavior_object: "vampirebrute",
        base_dir: "actors/dlc01/vampirebrute",
        default_behavior: "characters/vampirebrutecharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/vampirebrutebehavior.bin",
        master_behavior_index: "#0527",
    },
    "vampirelord" => BehaviorEntry {
        behavior_object: "vampirelord",
        base_dir: "actors/vampirelord",
        default_behavior: "characters/vampirelord.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/vampirelord.bin",
        master_behavior_index: "#1114",
    },
    "werewolfbeast" => BehaviorEntry {
        behavior_object: "werewolfbeast",
        base_dir: "actors/werewolfbeast",
        default_behavior: "characters/werewolf.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/werewolfbehavior.bin",
        master_behavior_index: "#1207",
    },
    "wisp" => BehaviorEntry {
        behavior_object: "wisp",
        base_dir: "actors/wisp",
        default_behavior: "characters/wispcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/wispbehavior.bin",
        master_behavior_index: "#0410",
    },
    "witchlight" => BehaviorEntry {
        behavior_object: "witchlight",
        base_dir: "actors/witchlight",
        default_behavior: "characters/witchlightcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/witchlightbehavior.bin",
        master_behavior_index: "#0154",
    },
    "wolf" => BehaviorEntry {
        behavior_object: "wolf",
        base_dir: "actors/canine",
        default_behavior: "characters wolf/wolf.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors wolf/wolfbehavior.bin",
        master_behavior_index: "#0169",
    },
};

pub static AUXBONES: phf::Map<&'static str, BehaviorEntry> = phf::phf_map! {
    "tail" => BehaviorEntry {
        behavior_object: "tail",
        base_dir: "auxbones/tail",
        default_behavior: "characters/tailcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/tailbehavior.bin",
        master_behavior_index: "#0506",
    },
};

pub static PLANTS_ACTIVATORS: phf::Map<&'static str, BehaviorEntry> = phf::phf_map! {
    "caveworm" => BehaviorEntry {
        behavior_object: "caveworm",
        base_dir: "dlc01/plants/caveworm",
        default_behavior: "characters/character01.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/behavior00.bin",
        master_behavior_index: "#0073",
    },
    "cavewormgroup" => BehaviorEntry {
        behavior_object: "cavewormgroup",
        base_dir: "dlc01/plants/cavewormgroup",
        default_behavior: "characters/character01.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/behavior00.bin",
        master_behavior_index: "#0073",
    },
    "cavewormsmall" => BehaviorEntry {
        behavior_object: "cavewormsmall",
        base_dir: "dlc01/plants/cavewormsmall",
        default_behavior: "characters/character01.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/behavior00.bin",
        master_behavior_index: "#0073",
    },
};
