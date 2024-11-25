//! The Nemesis path consists of the following
//!
//! - Format: `Nemesis_Engine/mod/<id>/[1st_person/]<template name>/<patch index>.txt`
//! - e.g.: `/some/path/to/Nemesis_Engine/mod/flinch/1st_person/0_master/#0106.txt`.
//!
//! From here, we need to determine and load the xml to which the patch will be applied, so we use a table to load this.
//! Note that the dir paths above `mesh` can be optionally specified and concatenated as resource paths later.
use phf::{phf_ordered_map, OrderedMap};

/// hkx to XML
pub type XmlTemplateTable = OrderedMap<&'static str, &'static str>;

/// NOTE: `shoutmounted_behavior` is x64 only
#[rustfmt::skip]
pub const THIRD_PERSON_BEHAVIORS: XmlTemplateTable = phf_ordered_map! {
    "0_master"                         => "meshes/actors/character/behaviors/0_master.xml"                        ,
    "1hm_behavior"                     => "meshes/actors/character/behaviors/1hm_behavior.xml"                    ,
    "1hm_locomotion"                   => "meshes/actors/character/behaviors/1hm_locomotion.xml"                  ,
    "bashbehavior"                     => "meshes/actors/character/behaviors/bashbehavior.xml"                    ,
    "blockbehavior"                    => "meshes/actors/character/behaviors/blockbehavior.xml"                   ,
    "bow_direction_behavior"           => "meshes/actors/character/behaviors/bow_direction_behavior.xml"          ,
    "horsebehavior"                    => "meshes/actors/character/behaviors/horsebehavior.xml"                   ,
    "idlebehavior"                     => "meshes/actors/character/behaviors/idlebehavior.xml"                    ,
    "magic_readied_direction_behavior" => "meshes/actors/character/behaviors/magic_readied_direction_behavior.xml",
    "magicbehavior"                    => "meshes/actors/character/behaviors/magicbehavior.xml"                   ,
    "mt_behavior"                      => "meshes/actors/character/behaviors/mt_behavior.xml"                     ,
    "shout_behavior"                   => "meshes/actors/character/behaviors/shout_behavior.xml"                  ,
    "shoutmounted_behavior.hkx"        => "meshes/actors/character/behaviors/shoutmounted_behavior.xml",
    "sprintbehavior"                   => "meshes/actors/character/behaviors/sprintbehavior.xml"                  ,
    "staggerbehavior"                  => "meshes/actors/character/behaviors/staggerbehavior.xml"                 ,
    "weapequip"                        => "meshes/actors/character/behaviors/weapequip.xml"                       ,
};

#[rustfmt::skip]
pub const FIRST_PERSON_BEHAVIORS: XmlTemplateTable = phf_ordered_map! {
    "0_master"               => "meshes/actors/character/_1stperson/behaviors/0_master.xml"              ,
    "1hm_behavior"           => "meshes/actors/character/_1stperson/behaviors/1hm_behavior.xml"          ,
    "1hm_locomotion"         => "meshes/actors/character/_1stperson/behaviors/1hm_locomotion.xml"        ,
    "bashbehavior"           => "meshes/actors/character/_1stperson/behaviors/bashbehavior.xml"          ,
    "blockbehavior"          => "meshes/actors/character/_1stperson/behaviors/blockbehavior.xml"         ,
    "bow_direction_behavior" => "meshes/actors/character/_1stperson/behaviors/bow_direction_behavior.xml",
    "horsebehavior"          => "meshes/actors/character/_1stperson/behaviors/horsebehavior.xml"         ,
    "idlebehavior"           => "meshes/actors/character/_1stperson/behaviors/idlebehavior.xml"          ,
    "magicbehavior"          => "meshes/actors/character/_1stperson/behaviors/magicbehavior.xml"         ,
    "mt_behavior"            => "meshes/actors/character/_1stperson/behaviors/mt_behavior.xml"           ,
    "shout_behavior"         => "meshes/actors/character/_1stperson/behaviors/shout_behavior.xml"        ,
    "sprintbehavior"         => "meshes/actors/character/_1stperson/behaviors/sprintbehavior.xml"        ,
    "staggerbehavior"        => "meshes/actors/character/_1stperson/behaviors/staggerbehavior.xml"       ,
    "weapequip"              => "meshes/actors/character/_1stperson/behaviors/weapequip.xml"             ,
};
