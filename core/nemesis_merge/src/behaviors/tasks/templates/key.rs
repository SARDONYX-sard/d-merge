use std::{borrow::Cow, path::Path};

/// Name of the template that needs to be read.
///
/// - UTF-8 path
/// - starts_with `meshes`
/// - extension: `bin` or `xml`
/// - e.g. `meshes/actors/character/_1stperson/behaviors/0_master.bin`
// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)] <- Path link manual implemented.
#[derive(Clone)]
pub struct TemplateKey<'a> {
    template_name: Cow<'a, str>,
}

impl TemplateKey<'static> {
    /// From nemesis file stem. e.g. `0_master` -> `meshes/actors/character/behaviors/0_master.bin`
    pub fn from_nemesis_file(template_file_stem: &str, is_1st_person: bool) -> Option<Self> {
        let template_name = match is_1st_person {
            true => NEMESIS_1ST_PERSON_MAP.get(template_file_stem)?,
            false => NEMESIS_3RD_PERSON_MAP.get(template_file_stem)?,
        };
        Some(Self {
            template_name: Cow::Borrowed(template_name),
        })
    }
}
impl<'a> TemplateKey<'a> {
    /// # Safety
    /// valid template path(from `meshes`)
    pub const unsafe fn new_unchecked(template_name: Cow<'a, str>) -> Self {
        Self { template_name }
    }

    /// As utf-8
    pub fn as_str(&self) -> &str {
        self.template_name.as_ref()
    }

    /// Get inner `meshes` path.
    /// - e.g. `meshes/actors/character/_1stperson/behaviors/0_master.bin`
    pub fn as_meshes_inner_path(&self) -> &Path {
        Path::new(self.template_name.as_ref())
    }

    // HACK: This is hack.
    /// This template has event names ` iState_NPCSneaking`(has space prefix) and `iState_NPCSneaking`(non space prefix). If duplicates are removed,
    /// the referenced targets will become misaligned.
    ///
    /// This is a check function to prevent that.
    ///
    /// # Note
    /// Vanilla Skyrim is configured this way. Removing duplicates causes NPCs to become immobile when casting spells.
    pub fn has_duplicate_event_names(&self) -> bool {
        matches!(
            self.as_str(),
            "meshes/actors/character/_1stperson/behaviors/magicbehavior.bin"
                | "meshes/actors/character/_1stperson/behaviors/magicmountedbehavior.bin"
                | "meshes/actors/character/behaviors/magicbehavior.bin"
                | "meshes/actors/character/behaviors/magicmountedbehavior.bin"
        )
    }
}

/* --- Path-like trait impls --- */

impl<'a> AsRef<Path> for TemplateKey<'a> {
    fn as_ref(&self) -> &Path {
        self.as_meshes_inner_path()
    }
}

impl<'a> core::borrow::Borrow<Path> for TemplateKey<'a> {
    fn borrow(&self) -> &Path {
        self.as_meshes_inner_path()
    }
}

impl<'a> std::ops::Deref for TemplateKey<'a> {
    type Target = Path;
    fn deref(&self) -> &Path {
        self.as_meshes_inner_path()
    }
}

/* Debug: delegate to Path's Debug so the output matches `&Path` derived Debug */
impl core::fmt::Debug for TemplateKey<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_meshes_inner_path(), f)
    }
}

/* Display: pretty print like a Path */
impl core::fmt::Display for TemplateKey<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_meshes_inner_path().display())
    }
}

/* Equality comparisons: behave like Path comparisons */
impl<'a, 'b> PartialEq<TemplateKey<'b>> for TemplateKey<'a> {
    fn eq(&self, other: &TemplateKey<'b>) -> bool {
        self.as_meshes_inner_path() == other.as_meshes_inner_path()
    }
}

impl<'a> PartialEq<Path> for TemplateKey<'a> {
    fn eq(&self, other: &Path) -> bool {
        self.as_meshes_inner_path() == other
    }
}

impl<'a> PartialEq<&Path> for TemplateKey<'a> {
    fn eq(&self, other: &&Path) -> bool {
        self.as_meshes_inner_path() == *other
    }
}

impl<'a> Eq for TemplateKey<'a> {}

/* Ordering: delegate to Path ordering */
impl<'a, 'b> PartialOrd<TemplateKey<'b>> for TemplateKey<'a> {
    fn partial_cmp(&self, other: &TemplateKey<'b>) -> Option<core::cmp::Ordering> {
        self.as_meshes_inner_path()
            .partial_cmp(other.as_meshes_inner_path())
    }
}

impl<'a> Ord for TemplateKey<'a> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_meshes_inner_path()
            .cmp(other.as_meshes_inner_path())
    }
}

/* Hash: delegate to Path's Hash (so it hashes like Path would) */
impl<'a> core::hash::Hash for TemplateKey<'a> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_meshes_inner_path().hash(state);
    }
}

pub const THREAD_PERSON_0_MASTER_KEY: TemplateKey<'static> = unsafe {
    TemplateKey::new_unchecked(Cow::Borrowed(
        "meshes/actors/character/behaviors/0_master.bin",
    ))
};
pub const THREAD_PERSON_MT_BEHAVIOR_KEY: TemplateKey<'static> = unsafe {
    TemplateKey::new_unchecked(Cow::Borrowed(
        "meshes/actors/character/behaviors/mt_behavior.bin",
    ))
};

/// Nemesis 1st person to meshes rel template .bin path
#[rustfmt::skip]
pub(crate) static NEMESIS_1ST_PERSON_MAP: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "0_master" => "meshes/actors/character/_1stperson/behaviors/0_master.bin",
    "1hm_behavior" => "meshes/actors/character/_1stperson/behaviors/1hm_behavior.bin",
    "1hm_locomotion" => "meshes/actors/character/_1stperson/behaviors/1hm_locomotion.bin",
    "bashbehavior" => "meshes/actors/character/_1stperson/behaviors/bashbehavior.bin",
    "blockbehavior" => "meshes/actors/character/_1stperson/behaviors/blockbehavior.bin",
    "bow_direction_behavior" => "meshes/actors/character/_1stperson/behaviors/bow_direction_behavior.bin",
    "crossbow_direction_behavior" => "meshes/actors/character/_1stperson/behaviors/crossbow_direction_behavior.bin",
    "firstperson" => "meshes/actors/character/_1stperson/characters/firstperson.bin",
    "firstperson_Project" => "meshes/actors/character/_1stperson/firstperson_Project.bin",
    "horsebehavior" => "meshes/actors/character/_1stperson/behaviors/horsebehavior.bin",
    "idlebehavior" => "meshes/actors/character/_1stperson/behaviors/idlebehavior.bin",
    "magicbehavior" => "meshes/actors/character/_1stperson/behaviors/magicbehavior.bin",
    "magicmountedbehavior" => "meshes/actors/character/_1stperson/behaviors/magicmountedbehavior.bin",
    "mt_behavior" => "meshes/actors/character/_1stperson/behaviors/mt_behavior.bin",
    "shout_behavior" => "meshes/actors/character/_1stperson/behaviors/shout_behavior.bin",
    "shoutmounted_behavior" => "meshes/actors/character/_1stperson/behaviors/shoutmounted_behavior.bin",
    "sprintbehavior" => "meshes/actors/character/_1stperson/behaviors/sprintbehavior.bin",
    "staggerbehavior" => "meshes/actors/character/_1stperson/behaviors/staggerbehavior.bin",
    "weapequip" => "meshes/actors/character/_1stperson/behaviors/weapequip.bin",
};

/// Nemesis third person to meshes rel template .bin path
#[rustfmt::skip]
pub(crate) static NEMESIS_3RD_PERSON_MAP: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "0_master" => "meshes/actors/character/behaviors/0_master.bin",
    "1hm_behavior" => "meshes/actors/character/behaviors/1hm_behavior.bin",
    "1hm_locomotion" => "meshes/actors/character/behaviors/1hm_locomotion.bin",
    "atronachflamebehavior" => "meshes/actors/atronachflame/behaviors/atronachflamebehavior.bin",
    "atronachfrostbehavior" => "meshes/actors/atronachfrost/behaviors/atronachfrostbehavior.bin",
    "atronachstormbehavior" => "meshes/actors/atronachstorm/behaviors/atronachstormbehavior.bin",
    "bashbehavior" => "meshes/actors/character/behaviors/bashbehavior.bin",
    "bcbehavior" => "meshes/actors/dlc02/dwarvenballistacenturion/behaviors/bcbehavior.bin",
    "bearbehavior" => "meshes/actors/bear/behaviors/bearbehavior.bin",
    "benthiclurkerbehavior" => "meshes/actors/dlc02/benthiclurker/behaviors/benthiclurkerbehavior.bin",
    "blockbehavior" => "meshes/actors/character/behaviors/blockbehavior.bin",
    "boarbehavior" => "meshes/actors/dlc02/boarriekling/behaviors/boarbehavior.bin",
    "bow_direction_behavior" => "meshes/actors/character/behaviors/bow_direction_behavior.bin",
    "chaurusbehavior" => "meshes/actors/chaurus/behaviors/chaurusbehavior.bin",
    "chaurusflyerbehavior" => "meshes/actors/dlc01/chaurusflyer/behaviors/chaurusflyerbehavior.bin",
    "chickenbehavior" => "meshes/actors/ambient/chicken/behaviors/chickenbehavior.bin",
    "crossbow_direction_behavior" => "meshes/actors/character/behaviors/crossbow_direction_behavior.bin",
    "deerbehavior" => "meshes/actors/deer/behaviors/deerbehavior.bin",
    "defaultfemale" => "meshes/actors/character/characters female/defaultfemale.bin",
    "defaultmale" => "meshes/actors/character/characters/defaultmale.bin",
    "dogbehavior" => "meshes/actors/canine/behaviors/dogbehavior.bin",
    "dragon_priest" => "meshes/actors/dragonpriest/behaviors/dragon_priest.bin",
    "dragonbehavior" => "meshes/actors/dragon/behaviors/dragonbehavior.bin",
    "draugr" => "meshes/actors/draugr/characters/draugr.bin", // To Support `Draugr MCO-DXP` Endge case
    "draugr_skeleton" => "meshes/actors/draugr/characterskeleton/draugr_skeleton.bin", // To support `Draugr MCO-DXP` Endge case
    "draugrbehavior" => "meshes/actors/draugr/behaviors/draugrbehavior.bin",
    "dwarvenspiderbehavior" => "meshes/actors/dwarvenspider/behaviors/dwarvenspiderbehavior.bin",
    "falmerbehavior" => "meshes/actors/falmer/behaviors/falmerbehavior.bin",
    "frostbitespiderbehavior" => "meshes/actors/frostbitespider/behaviors/frostbitespiderbehavior.bin",
    "giantbehavior" => "meshes/actors/giant/behaviors/giantbehavior.bin",
    "goatbehavior" => "meshes/actors/goat/behaviors/goatbehavior.bin",
    "h-cowbehavior" => "meshes/actors/cow/behaviors/h-cowbehavior.bin",
    "h-cowbehavior_lod" => "meshes/actors/cow/behaviors/h-cowbehavior_lod.bin",
    "harebehavior" => "meshes/actors/ambient/hare/behaviors/harebehavior.bin",
    "havgravenbehavior" => "meshes/actors/hagraven/behaviors/havgravenbehavior.bin",
    "hmdaedra" => "meshes/actors/dlc02/hmdaedra/behaviors/hmdaedra.bin",
    "horkerbehavior" => "meshes/actors/horker/behaviors/horkerbehavior.bin",
    "horsebehavior" => "meshes/actors/character/behaviors/horsebehavior.bin",
    "icewraithbehavior" => "meshes/actors/icewraith/behaviors/icewraithbehavior.bin",
    "idlebehavior" => "meshes/actors/character/behaviors/idlebehavior.bin",
    "magic_readied_direction_behavior" => "meshes/actors/character/behaviors/magic_readied_direction_behavior.bin",
    "magicbehavior" => "meshes/actors/character/behaviors/magicbehavior.bin",
    "magicmountedbehavior" => "meshes/actors/character/behaviors/magicmountedbehavior.bin",
    "mammothbehavior" => "meshes/actors/mammoth/behaviors/mammothbehavior.bin",
    "mt_behavior" => "meshes/actors/character/behaviors/mt_behavior.bin",
    "mudcrabbehavior" => "meshes/actors/mudcrab/behaviors/mudcrabbehavior.bin",
    "netchbehavior" => "meshes/actors/dlc02/netch/behaviors/netchbehavior.bin",
    "rieklingbehavior" => "meshes/actors/dlc02/riekling/behaviors/rieklingbehavior.bin",
    "sabrecatbehavior" => "meshes/actors/sabrecat/behaviors/sabrecatbehavior.bin",
    "scbehavior" => "meshes/actors/dwarvenspherecenturion/behaviors/scbehavior.bin",
    "scribbehavior" => "meshes/actors/dlc02/scrib/behaviors/scribbehavior.bin",
    "shout_behavior" => "meshes/actors/character/behaviors/shout_behavior.bin",
    "shoutmounted_behavior" => "meshes/actors/character/behaviors/shoutmounted_behavior.bin",
    "skeeverbehavior" => "meshes/actors/skeever/behaviors/skeeverbehavior.bin",
    "slaughterfishbehavior" => "meshes/actors/slaughterfish/behaviors/slaughterfishbehavior.bin",
    "sprigganbehavior" => "meshes/actors/spriggan/behaviors/sprigganbehavior.bin",
    "sprintbehavior" => "meshes/actors/character/behaviors/sprintbehavior.bin",
    "staggerbehavior" => "meshes/actors/character/behaviors/staggerbehavior.bin",
    "steambehavior" => "meshes/actors/dwarvensteamcenturion/behaviors/steambehavior.bin",
    "trollbehavior" => "meshes/actors/troll/behaviors/trollbehavior.bin",
    "vampirebrutebehavior" => "meshes/actors/dlc01/vampirebrute/behaviors/vampirebrutebehavior.bin",
    "weapequip" => "meshes/actors/character/behaviors/weapequip.bin",
    "werewolf" => "meshes/actors/werewolfbeast/characters/werewolf.bin",
    "werewolfbeastproject" => "meshes/actors/werewolfbeast/werewolfbeastproject.bin",
    "werewolfbehavior" => "meshes/actors/werewolfbeast/behaviors/werewolfbehavior.bin",
    "wispbehavior" => "meshes/actors/wisp/behaviors/wispbehavior.bin",
    "witchlightbehavior" => "meshes/actors/witchlight/behaviors/witchlightbehavior.bin",
    "wolfbehavior" => "meshes/actors/canine/behaviors wolf/wolfbehavior.bin",
};

/// Map from full template path to actor name
#[derive(Debug, Clone)]
pub struct MasterIndex {
    #[allow(unused)]
    pub master_string_data_index: &'static str,
    pub master_behavior_graph_index: &'static str,
}

impl MasterIndex {
    /// From nemesis file stem. e.g. `0_master` -> MasterIndex
    pub fn from_nemesis_file(
        template_file_stem: &str,
        is_1st_person: bool,
    ) -> Option<&'static Self> {
        if is_1st_person {
            FIRST_PERSON_INDEX_MAP.get(template_file_stem)
        } else {
            MASTER_INDEX_MAP.get(template_file_stem)
        }
    }
}

static FIRST_PERSON_INDEX_MAP: phf::Map<&'static str, MasterIndex> = phf::phf_map! {
    "0_master" => MasterIndex { master_string_data_index: "#0095", master_behavior_graph_index: "#0097" },
    "1hm_behavior" => MasterIndex { master_string_data_index: "#0085", master_behavior_graph_index: "#0087" },
    "1hm_locomotion" => MasterIndex { master_string_data_index: "#0063", master_behavior_graph_index: "#0065" },
    "bashbehavior" => MasterIndex { master_string_data_index: "#0056", master_behavior_graph_index: "#0058" },
    "blockbehavior" => MasterIndex { master_string_data_index: "#0076", master_behavior_graph_index: "#0078" },
    "bow_direction_behavior" => MasterIndex { master_string_data_index: "#0060", master_behavior_graph_index: "#0062" },
    "crossbow_direction_behavior" => MasterIndex { master_string_data_index: "#0060", master_behavior_graph_index: "#0062" },
    "horsebehavior" => MasterIndex { master_string_data_index: "#0087", master_behavior_graph_index: "#0089" },
    "idlebehavior" => MasterIndex { master_string_data_index: "#0055", master_behavior_graph_index: "#0057" },
    "magicbehavior" => MasterIndex { master_string_data_index: "#0077", master_behavior_graph_index: "#0079" },
    "magicmountedbehavior" => MasterIndex { master_string_data_index: "#0071", master_behavior_graph_index: "#0073" },
    "mt_behavior" => MasterIndex { master_string_data_index: "#0076", master_behavior_graph_index: "#0078" },
    "shout_behavior" => MasterIndex { master_string_data_index: "#0063", master_behavior_graph_index: "#0065" },
    "shoutmounted_behavior" => MasterIndex { master_string_data_index: "#0057", master_behavior_graph_index: "#0059" },
    "sprintbehavior" => MasterIndex { master_string_data_index: "#0057", master_behavior_graph_index: "#0059" },
    "staggerbehavior" => MasterIndex { master_string_data_index: "#0052", master_behavior_graph_index: "#0054" },
    "weapequip" => MasterIndex { master_string_data_index: "#0057", master_behavior_graph_index: "#0059" },
};

/// For 3rd person
static MASTER_INDEX_MAP: phf::Map<&'static str, MasterIndex> = phf::phf_map! {
    "0_master" => MasterIndex { master_string_data_index: "#0106", master_behavior_graph_index: "#0108" },
    "1hm_behavior" => MasterIndex { master_string_data_index: "#0085", master_behavior_graph_index: "#0087" },
    "1hm_locomotion" => MasterIndex { master_string_data_index: "#0063", master_behavior_graph_index: "#0065" },
    "atronachflamebehavior" => MasterIndex { master_string_data_index: "#0086", master_behavior_graph_index: "#0088" },
    "atronachfrostbehavior" => MasterIndex { master_string_data_index: "#0088", master_behavior_graph_index: "#0090" },
    "atronachstormbehavior" => MasterIndex { master_string_data_index: "#0083", master_behavior_graph_index: "#0085" },
    "bashbehavior" => MasterIndex { master_string_data_index: "#0056", master_behavior_graph_index: "#0058" },
    "bcbehavior" => MasterIndex { master_string_data_index: "#0088", master_behavior_graph_index: "#0090" },
    "bearbehavior" => MasterIndex { master_string_data_index: "#0078", master_behavior_graph_index: "#0080" },
    "benthiclurkerbehavior" => MasterIndex { master_string_data_index: "#0093", master_behavior_graph_index: "#0095" },
    "blockbehavior" => MasterIndex { master_string_data_index: "#0076", master_behavior_graph_index: "#0078" },
    "boarbehavior" => MasterIndex { master_string_data_index: "#0093", master_behavior_graph_index: "#0095" },
    "bow_direction_behavior" => MasterIndex { master_string_data_index: "#0060", master_behavior_graph_index: "#0062" },
    "chaurusbehavior" => MasterIndex { master_string_data_index: "#0091", master_behavior_graph_index: "#0093" },
    "chaurusflyerbehavior" => MasterIndex { master_string_data_index: "#0085", master_behavior_graph_index: "#0087" },
    "chickenbehavior" => MasterIndex { master_string_data_index: "#0085", master_behavior_graph_index: "#0087" },
    "crossbow_direction_behavior" => MasterIndex { master_string_data_index: "#0060", master_behavior_graph_index: "#0062" },
    "deerbehavior" => MasterIndex { master_string_data_index: "#0078", master_behavior_graph_index: "#0080" },
    "dogbehavior" => MasterIndex { master_string_data_index: "#0078", master_behavior_graph_index: "#0080" },
    "dragon_priest" => MasterIndex { master_string_data_index: "#0088", master_behavior_graph_index: "#0090" },
    "dragonbehavior" => MasterIndex { master_string_data_index: "#0101", master_behavior_graph_index: "#0103" },
    "draugrbehavior" => MasterIndex { master_string_data_index: "#0092", master_behavior_graph_index: "#0094" },
    "dwarvenspiderbehavior" => MasterIndex { master_string_data_index: "#0082", master_behavior_graph_index: "#0084" },
    "falmerbehavior" => MasterIndex { master_string_data_index: "#0099", master_behavior_graph_index: "#0101" },
    "frostbitespiderbehavior" => MasterIndex { master_string_data_index: "#0084", master_behavior_graph_index: "#0086" },
    "giantbehavior" => MasterIndex { master_string_data_index: "#0093", master_behavior_graph_index: "#0095" },
    "goatbehavior" => MasterIndex { master_string_data_index: "#0077", master_behavior_graph_index: "#0079" },
    "h-cowbehavior" => MasterIndex { master_string_data_index: "#0078", master_behavior_graph_index: "#0080" },
    "h-cowbehavior_lod" => MasterIndex { master_string_data_index: "#0078", master_behavior_graph_index: "#0080" },
    "harebehavior" => MasterIndex { master_string_data_index: "#0085", master_behavior_graph_index: "#0087" },
    "havgravenbehavior" => MasterIndex { master_string_data_index: "#0088", master_behavior_graph_index: "#0090" },
    "hmdaedra" => MasterIndex { master_string_data_index: "#0086", master_behavior_graph_index: "#0088" },
    "horkerbehavior" => MasterIndex { master_string_data_index: "#0078", master_behavior_graph_index: "#0080" },
    "horsebehavior" => MasterIndex { master_string_data_index: "#0087", master_behavior_graph_index: "#0089" },
    "icewraithbehavior" => MasterIndex { master_string_data_index: "#0079", master_behavior_graph_index: "#0081" },
    "idlebehavior" => MasterIndex { master_string_data_index: "#0062", master_behavior_graph_index: "#0064" },
    "magic_readied_direction_behavior" => MasterIndex { master_string_data_index: "#0054", master_behavior_graph_index: "#0056" },
    "magicbehavior" => MasterIndex { master_string_data_index: "#0077", master_behavior_graph_index: "#0079" },
    "magicmountedbehavior" => MasterIndex { master_string_data_index: "#0071", master_behavior_graph_index: "#0073" },
    "mammothbehavior" => MasterIndex { master_string_data_index: "#0078", master_behavior_graph_index: "#0080" },
    "mt_behavior" => MasterIndex { master_string_data_index: "#0083", master_behavior_graph_index: "#0085" },
    "mudcrabbehavior" => MasterIndex { master_string_data_index: "#0086", master_behavior_graph_index: "#0088" },
    "netchbehavior" => MasterIndex { master_string_data_index: "#0081", master_behavior_graph_index: "#0083" },
    "rieklingbehavior" => MasterIndex { master_string_data_index: "#0095", master_behavior_graph_index: "#0097" },
    "sabrecatbehavior" => MasterIndex { master_string_data_index: "#0077", master_behavior_graph_index: "#0079" },
    "scbehavior" => MasterIndex { master_string_data_index: "#0085", master_behavior_graph_index: "#0087" },
    "scribbehavior" => MasterIndex { master_string_data_index: "#0094", master_behavior_graph_index: "#0096" },
    "shout_behavior" => MasterIndex { master_string_data_index: "#0063", master_behavior_graph_index: "#0065" },
    "shoutmounted_behavior" => MasterIndex { master_string_data_index: "#0057", master_behavior_graph_index: "#0059" },
    "skeeverbehavior" => MasterIndex { master_string_data_index: "#0077", master_behavior_graph_index: "#0079" },
    "slaughterfishbehavior" => MasterIndex { master_string_data_index: "#0081", master_behavior_graph_index: "#0083" },
    "sprigganbehavior" => MasterIndex { master_string_data_index: "#0090", master_behavior_graph_index: "#0092" },
    "sprintbehavior" => MasterIndex { master_string_data_index: "#0057", master_behavior_graph_index: "#0059" },
    "staggerbehavior" => MasterIndex { master_string_data_index: "#0052", master_behavior_graph_index: "#0054" },
    "steambehavior" => MasterIndex { master_string_data_index: "#0085", master_behavior_graph_index: "#0087" },
    "trollbehavior" => MasterIndex { master_string_data_index: "#0089", master_behavior_graph_index: "#0091" },
    "vampirebrutebehavior" => MasterIndex { master_string_data_index: "#0093", master_behavior_graph_index: "#0095" },
    "weapequip" => MasterIndex { master_string_data_index: "#0057", master_behavior_graph_index: "#0059" },
    "werewolfbehavior" => MasterIndex { master_string_data_index: "#0096", master_behavior_graph_index: "#0098" },
    "wispbehavior" => MasterIndex { master_string_data_index: "#0086", master_behavior_graph_index: "#0088" },
    "witchlightbehavior" => MasterIndex { master_string_data_index: "#0064", master_behavior_graph_index: "#0066" },
    "wolfbehavior" => MasterIndex { master_string_data_index: "#0078", master_behavior_graph_index: "#0080" },
};
