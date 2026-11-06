//! Pins what the patch generators produce for each FNIS list syntax.
//!
//! # Why
//! The generators are long tables of Havok classes with few branches. A wrong index in one of them
//! compiles, and shows up only in the game. These tests make any change to the output visible, as a
//! diff of `snapshots/*.txt`.
//!
//! # When a test fails
//! Read the diff. If the change is what you meant, write the new snapshots and commit them:
//! ```text
//! $env:UPDATE_FNIS_SNAPSHOTS = 1; cargo test -p fnis_patch_gen snapshot
//! ```
//!
//! # Format
//! Everything in `dump` is sorted or written in a fixed order, as the maps have none. Numbers keep
//! their type (`I64(-1)`, `U64(1)`, `F64(1.0)`), as a change of type is a change of what is generated.
use std::{fmt::Write as _, path::Path};

use json_patch::ValueWithPriority;
use simd_json::BorrowedValue;

use super::{
    collect_borrowed_patches,
    generated_behaviors::{CREATURES, HUMANOID},
};
use crate::{
    adsf::AdsfPatch, asdsf::AsdsfPatch, collect::owned::OwnedFnisInjection, config::Config, patches::PatchCollection,
};

/// Writes `value` with its object keys sorted, one field per line.
fn write_value(out: &mut String, value: &BorrowedValue<'_>, indent: usize) {
    const STEP: usize = 2;
    match value {
        BorrowedValue::Static(node) => write!(out, "{node:?}").unwrap(),
        BorrowedValue::String(text) => write!(out, "{:?}", text.as_ref()).unwrap(),
        BorrowedValue::Array(items) if items.is_empty() => out.push_str("[]"),
        BorrowedValue::Array(items) => {
            out.push_str("[\n");
            for item in items.iter() {
                write!(out, "{:indent$}", "", indent = indent + STEP).unwrap();
                write_value(out, item, indent + STEP);
                out.push('\n');
            }
            write!(out, "{:indent$}]", "").unwrap();
        }
        BorrowedValue::Object(fields) => {
            let mut fields: Vec<_> = fields.iter().collect();
            fields.sort_unstable_by(|a, b| a.0.cmp(b.0));
            out.push_str("{\n");
            for (key, field) in fields {
                write!(out, "{:indent$}{key}: ", "", indent = indent + STEP).unwrap();
                write_value(out, field, indent + STEP);
                out.push('\n');
            }
            write!(out, "{:indent$}}}", "").unwrap();
        }
    }
}

fn write_patch(out: &mut String, patch: &ValueWithPriority<'_>) {
    write!(out, "  {:?} priority={} ", patch.patch.action, patch.priority).unwrap();
    write_value(out, &patch.patch.value, 2);
    out.push('\n');
}

/// Writes all that `collect_borrowed_patches` returned, in an order that does not depend on the maps.
#[expect(clippy::significant_drop_tightening, reason = "A test: nothing else wants the locks of the maps")]
pub(super) fn dump(
    patches: &PatchCollection<'_>,
    adsf_patches: &[AdsfPatch<'_>],
    asdsf_patches: &[AsdsfPatch<'_>],
    errors: &[crate::Error],
) -> String {
    let mut out = String::new();

    let mut templates: Vec<_> = patches.borrowed_patches.0.iter().collect();
    templates.sort_unstable_by_key(|template| template.key().to_string());
    for template in templates {
        writeln!(out, "# template {}", template.key()).unwrap();

        let one = template.value().one.clone().into_inner();
        let mut one: Vec<_> = one.iter().collect();
        one.sort_unstable_by(|a, b| a.key().cmp(b.key()));
        for field in one {
            writeln!(out, "one {}", field.key().join("/")).unwrap();
            write_patch(&mut out, field.value());
        }

        // The order inside of a `seq` is what is generated, and it comes first to last: not sorted.
        let mut seq: Vec<_> = template.value().seq.0.iter().collect();
        seq.sort_unstable_by(|a, b| a.key().cmp(b.key()));
        for field in seq {
            writeln!(out, "seq {}", field.key().join("/")).unwrap();
            for patch in field.value() {
                write_patch(&mut out, patch);
            }
        }
    }

    let mut indexes: Vec<_> = patches.behavior_index_map.0.iter().collect();
    indexes.sort_unstable_by_key(|index| index.key().to_string());
    for index in indexes {
        writeln!(out, "# index {} {:?}", index.key(), index.value()).unwrap();
    }

    for patch in adsf_patches {
        writeln!(out, "# adsf {patch:?}").unwrap();
    }
    for patch in asdsf_patches {
        writeln!(out, "# asdsf {patch:?}").unwrap();
    }
    for error in errors {
        writeln!(out, "# error {error}").unwrap();
    }
    out
}

fn injection(list_content: &str, behavior_object: &str, namespace: &str) -> OwnedFnisInjection {
    let behavior_entry = HUMANOID
        .get(behavior_object)
        .or_else(|| CREATURES.get(behavior_object))
        .unwrap_or_else(|| panic!("unknown behavior object: {behavior_object}"));
    OwnedFnisInjection::new(
        Path::new("meshes/actors/character/animations").join(namespace),
        behavior_entry,
        namespace.to_string(),
        7,
        list_content.to_string(),
        format!("Behaviors\\FNIS_{namespace}_Behavior.hkx"),
    )
}

fn generated(list_content: &str, behavior_object: &str) -> String {
    let injections = [injection(list_content, behavior_object, "Sample")];
    let config = Config::default();
    let (patches, adsf_patches, asdsf_patches, _jobs, errors) = collect_borrowed_patches(&injections, &config);
    let mut out = dump(&patches, &adsf_patches, &asdsf_patches, &errors);
    // How many classes were generated: the next index has to stay the same as well.
    writeln!(out, "# next class {}", injections[0].next_class_name_attribute()).unwrap();
    out
}

#[track_caller]
fn check(name: &str, list_content: &str, behavior_object: &str) {
    let actual = generated(list_content, behavior_object);

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/patch_gen/snapshots").join(name).with_extension("txt");
    if std::env::var_os("UPDATE_FNIS_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    // git may have checked it out with CRLF.
    let expected = expected.replace("\r\n", "\n");
    assert!(
        actual == expected,
        "snapshot {name} differs: see the head of this file\n{}",
        first_difference(&actual, &expected)
    );
}

/// The first line that differs: the snapshots are too long to print whole.
fn first_difference(actual: &str, expected: &str) -> String {
    let different = actual.lines().zip(expected.lines()).enumerate().find(|(_, (a, b))| a != b);
    match different {
        Some((line, (a, b))) => format!("line {}:\n  actual:   {a}\n  expected: {b}", line + 1),
        None => format!("line counts: actual {}, expected {}", actual.lines().count(), expected.lines().count()),
    }
}

#[test]
fn snapshot_basic() {
    check("basic", "Version V1.0\nb -a,Tsound.play/1.5 sample_idle sample_idle.hkx\n", "character");
}

#[test]
fn snapshot_basic_with_motion_data() {
    check(
        "basic_motion",
        "b sample_walk sample_walk.hkx\nMD 2.9333 0 0 30\nMD 3.5 1 2 40\nRD 2.9333 0\nRD 3.5 0 0 0.7 0.7\n",
        "character",
    );
}

#[test]
fn snapshot_basic_of_the_first_person() {
    check("basic_1stperson", "b sample_idle sample_idle.hkx\n", "character/_1stperson");
}

#[test]
fn snapshot_basic_of_a_creature() {
    check("basic_creature", "b sample_idle sample_idle.hkx\n", "draugr");
}

#[test]
fn snapshot_anim_object() {
    check("anim_object", "o -a,Tsound.play/4.5 sample_eat sample_eat.hkx _AnimObjectBread\n", "character");
}

#[test]
fn snapshot_anim_var() {
    check(
        "anim_var",
        "AnimVar SampleBool BOOL 1\nAnimVar SampleInt INT32 42\nAnimVar SampleReal REAL 1.5\n",
        "character",
    );
}

#[test]
fn snapshot_sequenced() {
    check(
        "sequenced",
        "s -a,Tsound.play/1.7 sample_first sample_first.hkx\n+ sample_second sample_second.hkx\n+ -a sample_last sample_last.hkx\n",
        "character",
    );
}

#[test]
fn snapshot_offset_arm() {
    check("offset_arm", "ofa -a,o sample_drink sample_drink.hkx _AnimObjectMead\n", "character");
}

#[test]
fn snapshot_chair() {
    check(
        "chair",
        "ch -o,Tsound.play/2.4 sample_sit sample_sit.hkx _AnimObjectStew\n+ dummy2 sample_sit2.hkx\n+ dummy3 sample_sit3.hkx\n+ dummy4 sample_sit4.hkx\n",
        "character",
    );
}

#[test]
fn snapshot_furniture() {
    check(
        "furniture",
        "fu -a sample_enter sample_enter.hkx\n+ -Tsound.play/1.5 sample_loop sample_loop.hkx\n+ -a sample_exit sample_exit.hkx\n",
        "character",
    );
}

#[test]
fn snapshot_furniture_with_anim_objects() {
    check(
        "furniture_objects",
        "fuo -a,o sample_enter sample_enter.hkx _AnimObjectMead\n+ -o sample_loop sample_loop.hkx _AnimObjectMead\n+ -a,o sample_exit sample_exit.hkx _AnimObjectMead\n",
        "character",
    );
}

#[test]
fn snapshot_paired() {
    check(
        "paired",
        "pa -o,D3.466667,Tnpcsoundplay.npckillchop/2.555,T2_killactor/3.333 sample_hug sample_hug.hkx AnimObjectSword/1 AnimObjectAxe/2\n",
        "character",
    );
}

#[test]
fn snapshot_kill_move() {
    check(
        "kill_move",
        "km -o,D6.266666,TSoundPlay.NPCKillClutch/0.900,TNPCHitFrame/4.633,T2_KillActor/5.867 sample_kill sample_kill.hkx AnimObjectSword/1\n",
        "character",
    );
}

#[test]
fn snapshot_all_in_one_list() {
    check(
        "all_in_one_list",
        "Version V1.0\n\
AnimVar SampleBool BOOL 1\n\
b sample_idle sample_idle.hkx\n\
o sample_eat sample_eat.hkx _AnimObjectBread\n\
s sample_first sample_first.hkx\n+ sample_last sample_last.hkx\n\
ofa -a,o sample_drink sample_drink.hkx _AnimObjectMead\n\
ch sample_sit sample_sit.hkx\n+ dummy2 sample_sit2.hkx\n+ dummy3 sample_sit3.hkx\n+ dummy4 sample_sit4.hkx\n\
fu -a sample_enter sample_enter.hkx\n+ sample_loop sample_loop.hkx\n+ -a sample_exit sample_exit.hkx\n\
pa -D3.4,Tsample_trigger/1.0 sample_hug sample_hug.hkx\n\
km -D6.2 sample_kill sample_kill.hkx\n",
        "character",
    );
}
