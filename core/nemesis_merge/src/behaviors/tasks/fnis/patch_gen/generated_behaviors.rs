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
    /// `hkbBehaviorGraphStringData` XML index. e.g. `#0106`, _1stperson `#0095`
    /// Used for pushing to eventNames.
    pub master_string_data_index: &'static str,
    /// `hkbBehaviorGraphData` index. e.g. `#0108`
    /// Used for pushing to eventInfos.
    pub master_behavior_graph_index: &'static str,
    /// `hkbVariableValueSet` index.
    ///
    /// Used for pushing FNIS AnimVar.
    /// - hkbBehaviorGraphStringData.variableNames
    /// - hkbVariableValueSet.wordVariableValues
    /// - hkbBehaviorGraphData.variableInfos
    pub master_value_set_index: &'static str,

    /// `animationdatasinglefile.txt` map key.
    /// Used for generating FNIS patches with MD and RD.
    /// - e.g. `DefaultMale~1`
    ///
    /// # Note
    /// If `character` or `character/_1stperson`, patches must be
    /// applied to both `DefaultMale~1` and `DefaultFemale~1`.
    pub anim_data_key: &'static str,
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

    /// Is `character` or `character/_1stperson` patch.
    ///
    /// # Usage
    /// Regarding characters, in addition to `default_behavior: "characters/defaultmale.hkx"`,
    /// the same animation path must also be registered in `characters/female/defaultfemale.hkx`.
    /// This is the condition check for that purpose.
    #[inline]
    pub fn is_humanoid(&self) -> bool {
        HUMANOID.contains_key(self.behavior_object)
    }

    /// This patch returns whether `draugr` is the target.
    ///
    /// # Use cases for this API
    /// `draugr` shares `draugrskeleton` and skeleton, so the same Animation must be registered. This is used for that determination.
    /// Consequently, `event` and `animationdatasinglefile.txt` are also synchronized. (It's unclear if this is actually correct)
    #[inline]
    pub fn is_draugr(&self) -> bool {
        self.behavior_object == "draugr"
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
        master_string_data_index: "#0095",
        master_behavior_graph_index: "#0097",
        master_value_set_index: "#0096",
        anim_data_key: "FirstPerson~1",
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
        master_string_data_index: "#0106",
        master_behavior_graph_index: "#0108",
        master_value_set_index: "#0107",
        anim_data_key: "DefaultMale~1", // Need `DefaultFemale~1` too
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
    master_string_data_index: "#0106",
    master_behavior_graph_index: "#0108",
    master_value_set_index: "#0107",
    anim_data_key: "DefaultFemale~1",
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
    master_string_data_index: "#0092",
    master_behavior_graph_index: "#0094",
    master_value_set_index: "#0093",
    anim_data_key: "DraugrSkeletonProject~1",
};

pub static CREATURES: phf::Map<&'static str, BehaviorEntry> = phf::phf_map! {
    "atronachflame" => BehaviorEntry {
        behavior_object: "atronachflame",
        base_dir: "actors/atronachflame",
        default_behavior: "characters/atronachflame.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/atronachflamebehavior.bin",
        master_behavior_index: "#0438",
        master_string_data_index: "#0086",
        master_behavior_graph_index: "#0088",
        master_value_set_index: "#0087",
        anim_data_key: "AtronachFlame~1",
    },
    "atronachfrost" => BehaviorEntry {
        behavior_object: "atronachfrost",
        base_dir: "actors/atronachfrost",
        default_behavior: "characters/atronachfrostcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/atronachfrostbehavior.bin",
        master_behavior_index: "#0451",
        master_string_data_index: "#0088",
        master_behavior_graph_index: "#0090",
        master_value_set_index: "#0089",
        anim_data_key: "AtronachFrostProject~1",
    },
    "atronachstorm" => BehaviorEntry {
        behavior_object: "atronachstorm",
        base_dir: "actors/atronachstorm",
        default_behavior: "characters/atronachstormcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/atronachstormbehavior.bin",
        master_behavior_index: "#0384",
        master_string_data_index: "#0083",
        master_behavior_graph_index: "#0085",
        master_value_set_index: "#0084",
        anim_data_key: "AtronachStormProject~1",
    },
    "bear" => BehaviorEntry {
        behavior_object: "bear",
        base_dir: "actors/bear",
        default_behavior: "characters/bearcharacter.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/bearbehavior.bin",
        master_behavior_index: "#0151",
        master_string_data_index: "#0078",
        master_behavior_graph_index: "#0080",
        master_value_set_index: "#0079",
        anim_data_key: "BearProject~1",
    },
    "benthiclurker" => BehaviorEntry {
        behavior_object: "benthiclurker",
        base_dir: "actors/dlc02/benthiclurker",
        default_behavior: "characters/benthiclurkercharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/benthiclurkerbehavior.bin",
        master_behavior_index: "#0733",
        master_string_data_index: "#0093",
        master_behavior_graph_index: "#0095",
        master_value_set_index: "#0094",
        anim_data_key: "BenthicLurkerProject~1",
    },
    "boarriekling" => BehaviorEntry {
        behavior_object: "boarriekling",
        base_dir: "actors/dlc02/boarriekling",
        default_behavior: "characters/boar.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/boarbehavior.bin",
        master_behavior_index: "#0584",
        master_string_data_index: "#0093",
        master_behavior_graph_index: "#0095",
        master_value_set_index: "#0094",
        anim_data_key: "BoarProject~1",
    },
    "chaurus" => BehaviorEntry {
        behavior_object: "chaurus",
        base_dir: "actors/chaurus",
        default_behavior: "characters/chaurus.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/chaurusbehavior.bin",
        master_behavior_index: "#0509",
        master_string_data_index: "#0091",
        master_behavior_graph_index: "#0093",
        master_value_set_index: "#0092",
        anim_data_key: "ChickenProject~1",
    },
    "chaurusflyer" => BehaviorEntry {
        behavior_object: "chaurusflyer",
        base_dir: "actors/dlc01/chaurusflyer",
        default_behavior: "characters/chaurusflyercharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/chaurusflyerbehavior.bin",
        master_behavior_index: "#0406",
        master_string_data_index: "#0085",
        master_behavior_graph_index: "#0087",
        master_value_set_index: "#0086",
        anim_data_key: "ChaurusFlyer~1",
    },
    "chicken" => BehaviorEntry {
        behavior_object: "chicken",
        base_dir: "actors/ambient/chicken",
        default_behavior: "characters/chickencharater.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/chickenbehavior.bin",
        master_behavior_index: "#0332",
        master_string_data_index: "#0085",
        master_behavior_graph_index: "#0087",
        master_value_set_index: "#0086",
        anim_data_key: "ChickenProject~1",
    },
    "cow" => BehaviorEntry {
        behavior_object: "cow",
        base_dir: "actors/cow",
        default_behavior: "characters/h_cowcharater.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/h-cowbehavior.bin",
        master_behavior_index: "#0152",
        master_string_data_index: "#0078",
        master_behavior_graph_index: "#0080",
        master_value_set_index: "#0079",
        anim_data_key: "HighlandCowProject~1",
    },
    "deer" => BehaviorEntry {
        behavior_object: "deer",
        base_dir: "actors/deer",
        default_behavior: "characters/deercharater.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/deerbehavior.bin",
        master_behavior_index: "#0145",
        master_string_data_index: "#0078",
        master_behavior_graph_index: "#0080",
        master_value_set_index: "#0079",
        anim_data_key: "DeerProject~1",
    },
    "dog" => BehaviorEntry {
        behavior_object: "dog",
        base_dir: "actors/canine",
        default_behavior: "characters dog/dog.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/dogbehavior.bin",
        master_behavior_index: "#0144",
        master_string_data_index: "#0078",
        master_behavior_graph_index: "#0080",
        master_value_set_index: "#0079",
        anim_data_key: "DogProject~1",
    },
    "dragon" => BehaviorEntry {
        behavior_object: "dragon",
        base_dir: "actors/dragon",
        default_behavior: "characters/dragontest.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/dragonbehavior.bin",
        master_behavior_index: "#1610",
        master_string_data_index: "#0101",
        master_behavior_graph_index: "#0103",
        master_value_set_index: "#0102",
        anim_data_key: "DragonProject~1",
    },
    "dragonpriest" => BehaviorEntry {
        behavior_object: "dragonpriest",
        base_dir: "actors/dragonpriest",
        default_behavior: "characters/dragon_priest.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/dragon_priest.bin",
        master_behavior_index: "#0796",
        master_string_data_index: "#0088",
        master_behavior_graph_index: "#0090",
        master_value_set_index: "#0089",
        anim_data_key: "Dragon_Priest~1",
    },
    "draugr" => BehaviorEntry {
        behavior_object: "draugr",
        base_dir: "actors/draugr",
        default_behavior: "characters/draugr.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/draugrbehavior.bin",
        master_behavior_index: "#2026",
        master_string_data_index: "#0092",
        master_behavior_graph_index: "#0094",
        master_value_set_index: "#0093",
        anim_data_key: "DraugrProject~1",
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
        master_string_data_index: "#0092",
        master_behavior_graph_index: "#0094",
        master_value_set_index: "#0093",
        anim_data_key: "DraugrSkeletonProject~1",
    },
    "dwarvenballistacenturion" => BehaviorEntry {
        behavior_object: "dwarvenballistacenturion",
        base_dir: "actors/dlc02/dwarvenballistacenturion",
        default_behavior: "characters/ballistacenturion.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/bcbehavior.bin",
        master_behavior_index: "#0492",
        master_string_data_index: "#0088",
        master_behavior_graph_index: "#0090",
        master_value_set_index: "#0089",
        anim_data_key: "BallistaCenturion~1",
    },
    "dwarvenspherecenturion" => BehaviorEntry {
        behavior_object: "dwarvenspherecenturion",
        base_dir: "actors/dwarvenspherecenturion",
        default_behavior: "characters/spherecenturion.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/scbehavior.bin",
        master_behavior_index: "#0797",
        master_string_data_index: "#0085",
        master_behavior_graph_index: "#0087",
        master_value_set_index: "#0086",
        anim_data_key: "SphereCenturion~1",
    },
    "dwarvenspider" => BehaviorEntry {
        behavior_object: "dwarvenspider",
        base_dir: "actors/dwarvenspider",
        default_behavior: "characters/dwarvenspidercenturion.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/dwarvenspiderbehavior.bin",
        master_behavior_index: "#0404",
        master_string_data_index: "#0082",
        master_behavior_graph_index: "#0084",
        master_value_set_index: "#0083",
        anim_data_key: "DwarvenSpiderCenturionProject~1",
    },
    "dwarvensteamcenturion" => BehaviorEntry {
        behavior_object: "dwarvensteamcenturion",
        base_dir: "actors/dwarvensteamcenturion",
        default_behavior: "characters/dwarvensteam.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/steambehavior.bin",
        master_behavior_index: "#0552",
        master_string_data_index: "#0085",
        master_behavior_graph_index: "#0087",
        master_value_set_index: "#0086",
        anim_data_key: "SteamProject~1",
    },
    "falmer" => BehaviorEntry {
        behavior_object: "falmer",
        base_dir: "actors/falmer",
        default_behavior: "characters/falmer.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/falmerbehavior.bin",
        master_behavior_index: "#1314",
        master_string_data_index: "#0099",
        master_behavior_graph_index: "#0101",
        master_value_set_index: "#0100",
        anim_data_key: "FalmerProject~1",
    },
    "frostbitespider" => BehaviorEntry {
        behavior_object: "frostbitespider",
        base_dir: "actors/frostbitespider",
        default_behavior: "characters/frostbitespidercharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/frostbitespiderbehavior.bin",
        master_behavior_index: "#0412",
        master_string_data_index: "#0084",
        master_behavior_graph_index: "#0086",
        master_value_set_index: "#0085",
        anim_data_key: "FrostbiteSpiderProject~1",
    },
    "giant" => BehaviorEntry {
        behavior_object: "giant",
        base_dir: "actors/giant",
        default_behavior: "characters/giantcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/giantbehavior.bin",
        master_behavior_index: "#0822",
        master_string_data_index: "#0093",
        master_behavior_graph_index: "#0095",
        master_value_set_index: "#0094",
        anim_data_key: "GiantProject~1",
    },
    "goat" => BehaviorEntry {
        behavior_object: "goat",
        base_dir: "actors/goat",
        default_behavior: "characters/goatcharater.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/goatbehavior.bin",
        master_behavior_index: "#0140",
        master_string_data_index: "#0077",
        master_behavior_graph_index: "#0079",
        master_value_set_index: "#0078",
        anim_data_key: "GoatProject~1",
    },
    "hagraven" => BehaviorEntry {
        behavior_object: "hagraven",
        base_dir: "actors/hagraven",
        default_behavior: "characters/hagravencharacter.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/havgravenbehavior.bin",
        master_behavior_index: "#0634",
        master_string_data_index: "#0088",
        master_behavior_graph_index: "#0090",
        master_value_set_index: "#0089",
        anim_data_key: "HagravenProject~1",
    },
    "hare" => BehaviorEntry {
        behavior_object: "hare",
        base_dir: "actors/ambient/hare",
        default_behavior: "characters/harecharater.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/harebehavior.bin",
        master_behavior_index: "#0309",
        master_string_data_index: "#0085",
        master_behavior_graph_index: "#0087",
        master_value_set_index: "#0086",
        anim_data_key: "HareProject~1",
    },
    "hmdaedra" => BehaviorEntry {
        behavior_object: "hmdaedra",
        base_dir: "actors/dlc02/hmdaedra",
        default_behavior: "characters/hmdaedracharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/hmdaedra.bin",
        master_behavior_index: "#0504",
        master_string_data_index: "#0086",
        master_behavior_graph_index: "#0088",
        master_value_set_index: "#0087",
        anim_data_key: "HMDaedra~1",
    },
    "horker" => BehaviorEntry {
        behavior_object: "horker",
        base_dir: "actors/horker",
        default_behavior: "characters/horker.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/horkerbehavior.bin",
        master_behavior_index: "#0161",
        master_string_data_index: "#0078",
        master_behavior_graph_index: "#0080",
        master_value_set_index: "#0079",
        anim_data_key: "HorkerProject~1",
    },
    "horse" => BehaviorEntry {
        behavior_object: "horse",
        base_dir: "actors/horse",
        default_behavior: "characters/horse.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/horsebehavior.bin",
        master_behavior_index: "#0760",
        master_string_data_index: "#0087",
        master_behavior_graph_index: "#0089",
        master_value_set_index: "#0088",
        anim_data_key: "HorseProject~1",
    },
    "icewraith" => BehaviorEntry {
        behavior_object: "icewraith",
        base_dir: "actors/icewraith",
        default_behavior: "characters/icewraithcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/icewraithbehavior.bin",
        master_behavior_index: "#0266",
        master_string_data_index: "#0079",
        master_behavior_graph_index: "#0081",
        master_value_set_index: "#0080",
        anim_data_key: "IceWraithProject~1",
    },
    "mammoth" => BehaviorEntry {
        behavior_object: "mammoth",
        base_dir: "actors/mammoth",
        default_behavior: "characters/mammothcharacter.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/mammothbehavior.bin",
        master_behavior_index: "#0155",
        master_string_data_index: "#0078",
        master_behavior_graph_index: "#0080",
        master_value_set_index: "#0079",
        anim_data_key: "MammothProject~1",
    },
    "mudcrab" => BehaviorEntry {
        behavior_object: "mudcrab",
        base_dir: "actors/mudcrab",
        default_behavior: "characters/mudcrabcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/mudcrabbehavior.bin",
        master_behavior_index: "#0495",
        master_string_data_index: "#0086",
        master_behavior_graph_index: "#0088",
        master_value_set_index: "#0087",
        anim_data_key: "MudcrabProject~1",
    },
    "netch" => BehaviorEntry {
        behavior_object: "netch",
        base_dir: "actors/dlc02/netch",
        default_behavior: "characters/netchcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/netchbehavior.bin",
        master_behavior_index: "#0279",
        master_string_data_index: "#0081",
        master_behavior_graph_index: "#0083",
        master_value_set_index: "#0082",
        anim_data_key: "NetchProject~1",
    },
    "riekling" => BehaviorEntry {
        behavior_object: "riekling",
        base_dir: "actors/dlc02/riekling",
        default_behavior: "characters/rieklingcharacter.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/rieklingbehavior.bin",
        master_behavior_index: "#0769",
        master_string_data_index: "#0095",
        master_behavior_graph_index: "#0097",
        master_value_set_index: "#0096",
        anim_data_key: "RieklingProject~1",
    },
    "sabrecat" => BehaviorEntry {
        behavior_object: "sabrecat",
        base_dir: "actors/sabrecat",
        default_behavior: "characters/sabrecat.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/sabrecatbehavior.bin",
        master_behavior_index: "#0140",
        master_string_data_index: "#0077",
        master_behavior_graph_index: "#0079",
        master_value_set_index: "#0078",
        anim_data_key: "SabreCatProject~1",
    },
    "scrib" => BehaviorEntry {
        behavior_object: "scrib",
        base_dir: "actors/dlc02/scrib",
        default_behavior: "characters/scrib.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/scribbehavior.bin",
        master_behavior_index: "#0605",
        master_string_data_index: "#0094",
        master_behavior_graph_index: "#0096",
        master_value_set_index: "#0095",
        anim_data_key: "ScribProject~1",
    },
    "skeever" => BehaviorEntry {
        behavior_object: "skeever",
        base_dir: "actors/skeever",
        default_behavior: "characters/skeevercharacter.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/skeeverbehavior.bin",
        master_behavior_index: "#0132",
        master_string_data_index: "#0077",
        master_behavior_graph_index: "#0079",
        master_value_set_index: "#0078",
        anim_data_key: "SkeeverProject~1",
    },
    "slaughterfish" => BehaviorEntry {
        behavior_object: "slaughterfish",
        base_dir: "actors/slaughterfish",
        default_behavior: "characters/slaughterfish.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/slaughterfishbehavior.bin",
        master_behavior_index: "#0286",
        master_string_data_index: "#0081",
        master_behavior_graph_index: "#0083",
        master_value_set_index: "#0082",
        anim_data_key: "SlaughterfishProject~1",
    },
    "spriggan" => BehaviorEntry {
        behavior_object: "spriggan",
        base_dir: "actors/spriggan",
        default_behavior: "characters/spriggan.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/sprigganbehavior.bin",
        master_behavior_index: "#0635",
        master_string_data_index: "#0090",
        master_behavior_graph_index: "#0092",
        master_value_set_index: "#0091",
        anim_data_key: "Spriggan~1",
    },
    "troll" => BehaviorEntry {
        behavior_object: "troll",
        base_dir: "actors/troll",
        default_behavior: "characters/troll.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/trollbehavior.bin",
        master_behavior_index: "#0724",
        master_string_data_index: "#0089",
        master_behavior_graph_index: "#0091",
        master_value_set_index: "#0090",
        anim_data_key: "TrollProject~1",
    },
    "vampirebrute" => BehaviorEntry {
        behavior_object: "vampirebrute",
        base_dir: "actors/dlc01/vampirebrute",
        default_behavior: "characters/vampirebrutecharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/vampirebrutebehavior.bin",
        master_behavior_index: "#0527",
        master_string_data_index: "#0093",
        master_behavior_graph_index: "#0095",
        master_value_set_index: "#0094",
        anim_data_key: "VampireBruteProject~1",
    },
    "vampirelord" => BehaviorEntry {
        behavior_object: "vampirelord",
        base_dir: "actors/vampirelord",
        default_behavior: "characters/vampirelord.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/vampirelord.bin",
        master_behavior_index: "#1114",
        master_string_data_index: "#0101",
        master_behavior_graph_index: "#0103",
        master_value_set_index: "#0102",
        anim_data_key: "VampireLord~1",
    },
    "werewolfbeast" => BehaviorEntry {
        behavior_object: "werewolfbeast",
        base_dir: "actors/werewolfbeast",
        default_behavior: "characters/werewolf.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors/werewolfbehavior.bin",
        master_behavior_index: "#1207",
        master_string_data_index: "#0096",
        master_behavior_graph_index: "#0098",
        master_value_set_index: "#0097",
        anim_data_key: "WerewolfBeastProject~1",
    },
    "wisp" => BehaviorEntry {
        behavior_object: "wisp",
        base_dir: "actors/wisp",
        default_behavior: "characters/wispcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/wispbehavior.bin",
        master_behavior_index: "#0410",
        master_string_data_index: "#0086",
        master_behavior_graph_index: "#0088",
        master_value_set_index: "#0087",
        anim_data_key: "WispProject~1",
    },
    "witchlight" => BehaviorEntry {
        behavior_object: "witchlight",
        base_dir: "actors/witchlight",
        default_behavior: "characters/witchlightcharacter.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/witchlightbehavior.bin",
        master_behavior_index: "#0154",
        master_string_data_index: "#0064",
        master_behavior_graph_index: "#0066",
        master_value_set_index: "#0065",
        anim_data_key: "WitchlightProject~1",
    },
    "wolf" => BehaviorEntry {
        behavior_object: "wolf",
        base_dir: "actors/canine",
        default_behavior: "characters wolf/wolf.bin",
        default_behavior_index: "#0029",
        master_behavior: "behaviors wolf/wolfbehavior.bin",
        master_behavior_index: "#0169",
        master_string_data_index: "#0078",
        master_behavior_graph_index: "#0080",
        master_value_set_index: "#0079",
        anim_data_key: "WolfProject~1",
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
        master_string_data_index: "#0062",
        master_behavior_graph_index: "#0064",
        master_value_set_index: "#0063",
        anim_data_key: "TailProject~1",
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
        master_string_data_index: "#0052",
        master_behavior_graph_index: "#0054",
        master_value_set_index: "#0053",
        anim_data_key: "Caveworm~1",
    },
    "cavewormgroup" => BehaviorEntry {
        behavior_object: "cavewormgroup",
        base_dir: "dlc01/plants/cavewormgroup",
        default_behavior: "characters/character01.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/behavior00.bin",
        master_behavior_index: "#0073",
        master_string_data_index: "#0052",
        master_behavior_graph_index: "#0054",
        master_value_set_index: "#0053",
        anim_data_key: "Caveworm3Group~1",
    },
    "cavewormsmall" => BehaviorEntry {
        behavior_object: "cavewormsmall",
        base_dir: "dlc01/plants/cavewormsmall",
        default_behavior: "characters/character01.bin",
        default_behavior_index: "#0024",
        master_behavior: "behaviors/behavior00.bin",
        master_behavior_index: "#0073",
        master_string_data_index: "#0052",
        master_behavior_graph_index: "#0054",
        master_value_set_index: "#0053",
        anim_data_key: "CavewormSmall~1",
    },
};
