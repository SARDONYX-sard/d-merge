use std::path::Path;

/// Name of the template that needs to be read.
///
/// - UTF-8 path
/// - starts_with `meshes`
/// - extension: `bin` or `xml`
/// - e.g. `meshes/actors/character/_1stperson/behaviors/0_master.bin`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemplateKey<'a> {
    template_name: &'a Path,
}

impl<'a> TemplateKey<'a> {
    /// # Safety
    /// valid template path(from `meshes`)
    pub unsafe fn new_unchecked(template_name: &'a str) -> Self {
        Self {
            template_name: Path::new(template_name),
        }
    }

    /// From nemesis file stem. e.g. `0_master` -> `meshes/actors/character/behaviors/0_master.bin`
    pub fn from_nemesis_file(template_file_stem: &'a str, is_1st_person: bool) -> Option<Self> {
        let template_name = match is_1st_person {
            true => NEMESIS_1ST_PERSON_MAP.get(template_file_stem)?,
            false => NEMESIS_3RD_PERSON_MAP.get(template_file_stem)?,
        };
        Some(Self {
            template_name: Path::new(template_name),
        })
    }

    /// As utf-8
    pub fn as_str(&self) -> &str {
        // Safety: 100% utf-8 path
        unsafe { self.template_name.to_str().unwrap_unchecked() }
    }

    /// Get inner `meshes` path.
    /// - e.g. `meshes/actors/character/_1stperson/behaviors/0_master.bin`
    pub const fn as_meshes_inner_path(&self) -> &Path {
        self.template_name
    }
}

impl core::fmt::Display for TemplateKey<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.template_name.display())
    }
}

/// Nemesis 1st person to meshes rel template .bin path
#[rustfmt::skip]
static NEMESIS_1ST_PERSON_MAP: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "0_master"=> "meshes/actors/character/_1stperson/behaviors/0_master.bin",
    "1hm_behavior"=> "meshes/actors/character/_1stperson/behaviors/1hm_behavior.bin",
    "1hm_locomotion"=> "meshes/actors/character/_1stperson/behaviors/1hm_locomotion.bin",
    "bashbehavior"=> "meshes/actors/character/_1stperson/behaviors/bashbehavior.bin",
    "blockbehavior"=> "meshes/actors/character/_1stperson/behaviors/blockbehavior.bin",
    "bow_direction_behavior"=> "meshes/actors/character/_1stperson/behaviors/bow_direction_behavior.bin",
    "crossbow_direction_behavior"=> "meshes/actors/character/_1stperson/behaviors/crossbow_direction_behavior.bin",
    "firstperson"=> "meshes/actors/character/_1stperson/characters/firstperson.bin",
    "firstperson_Project"=> "meshes/actors/character/_1stperson/firstperson_Project.bin",
    "horsebehavior"=> "meshes/actors/character/_1stperson/behaviors/horsebehavior.bin",
    "idlebehavior"=> "meshes/actors/character/_1stperson/behaviors/idlebehavior.bin",
    "magicbehavior"=> "meshes/actors/character/_1stperson/behaviors/magicbehavior.bin",
    "magicmountedbehavior"=> "meshes/actors/character/_1stperson/behaviors/magicmountedbehavior.bin",
    "mt_behavior"=> "meshes/actors/character/_1stperson/behaviors/mt_behavior.bin",
    "shout_behavior"=> "meshes/actors/character/_1stperson/behaviors/shout_behavior.bin",
    "shoutmounted_behavior"=> "meshes/actors/character/_1stperson/behaviors/shoutmounted_behavior.bin",
    "sprintbehavior"=> "meshes/actors/character/_1stperson/behaviors/sprintbehavior.bin",
    "staggerbehavior"=> "meshes/actors/character/_1stperson/behaviors/staggerbehavior.bin",
    "weapequip"=> "meshes/actors/character/_1stperson/behaviors/weapequip.bin",
};

/// Nemesis third person to meshes rel template .bin path
#[rustfmt::skip]
static NEMESIS_3RD_PERSON_MAP: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "0_master" => "meshes/actors/character/behaviors/0_master.bin",
    "1hm_behavior" => "meshes/actors/character/behaviors/1hm_behavior.bin",
    "1hm_locomotion" => "meshes/actors/character/behaviors/1hm_locomotion.bin",
    "AttackBehavior" => "meshes/actors/character/behaviors/AttackBehavior.bin",
    "WeapUnequip" => "meshes/actors/character/behaviors/WeapUnequip.bin",
    "animationdatasinglefile" => "meshes/animationdatasinglefile.bin",
    "animationsetdatasinglefile" => "meshes/animationsetdatasinglefile.bin",
    "atronachflamebehavior" => "meshes/actors/atronachflame/behaviors/atronachflamebehavior.bin",
    "atronachfrostbehavior" => "meshes/actors/atronachfrost/behaviors/atronachfrostbehavior.bin",
    "atronachstormbehavior" => "meshes/actors/atronachstorm/behaviors/atronachstormbehavior.bin",
    "bashbehavior" => "meshes/actors/character/behaviors/bashbehavior.bin",
    "bcbehavior" => "meshes/actors/dlc02/dwarvenballistacenturion/behaviors/bcbehavior.bin",
    "bearbehavior" => "meshes/actors/bear/behaviors/bearbehavior.bin",
    "behavior" => "meshes/actors/character/behaviors/behavior.bin",
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
    "defaultfemale_Project" => "meshes/actors/character/defaultfemale_Project.bin",
    "defaultmale" => "meshes/actors/character/characters/defaultmale.bin",
    "defaultmale_Project" => "meshes/actors/character/defaultmale_Project.bin",
    "dogbehavior" => "meshes/actors/canine/behaviors/dogbehavior.bin",
    "dragon_priest" => "meshes/actors/dragonpriest/behaviors/dragon_priest.bin",
    "dragonbehavior" => "meshes/actors/dragon/behaviors/dragonbehavior.bin",
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
