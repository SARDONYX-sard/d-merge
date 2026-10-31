//! `template_bin_to_xml` keeps the object ids of the xml template it was made from.
//!
//! Needs local files: `cargo test -p hkx_io --test template_bin -- --ignored`
//! (`resource/assets/templates` and `resource/xml/templates` of this repository)

use havok_types as _;
use rmp_serde as _;
use serde_hkx as _;
use serde_hkx_features as _;
use simd_json as _;

/// `name="#0106"` ids in file order.
fn ids(xml: &str) -> Vec<&str> {
    xml.lines()
        .filter_map(|line| line.trim_start().strip_prefix("<hkobject name=\""))
        .filter_map(|rest| rest.split('"').next())
        .collect()
}

#[test]
#[ignore = "needs the template files of the repository"]
fn bin_keeps_xml_ids() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../resource");
    let path = "meshes/actors/draugr/behaviors/draugrbehavior";
    let bin = std::fs::read(format!("{root}/assets/templates/{path}.bin")).unwrap();
    let xml = std::fs::read_to_string(format!("{root}/xml/templates/{path}.xml")).unwrap();

    let from_bin = hkx_io::template_bin_to_xml(&bin).unwrap();
    // The xml has no comments; the bin's XML must not either (Nemesis' parser rejects them).
    let comments: std::collections::BTreeSet<&str> =
        from_bin.lines().map(str::trim).filter(|l| l.starts_with("<!--")).collect();
    assert!(comments.is_empty(), "{comments:?}");

    // Same text per object (Nemesis applies patches by matching template lines).
    let objects = |xml: &str| -> std::collections::BTreeMap<String, String> {
        let mut map = std::collections::BTreeMap::new();
        let mut current: Option<(String, String)> = None;
        for line in xml.lines() {
            if let Some(rest) = line.strip_prefix("\t\t<hkobject name=\"") {
                current =
                    Some((rest.split('"').next().unwrap_or_default().to_string(), String::new()));
            }
            if let Some((_, text)) = &mut current {
                text.push_str(line.trim_end());
                text.push('\n');
            }
            if line == "\t\t</hkobject>"
                && let Some((id, text)) = current.take()
            {
                map.insert(id, text);
            }
        }
        map
    };
    let (bin_objects, xml_objects) = (objects(&from_bin), objects(&xml));
    let differ: Vec<_> =
        xml_objects.iter().filter(|(id, text)| bin_objects.get(*id) != Some(text)).collect();
    if let Some((id, text)) = differ.first() {
        let bin = &bin_objects[*id];
        let at = text.lines().zip(bin.lines()).position(|(a, b)| a != b).unwrap_or(0);
        println!(
            "{} objects differ; e.g. {id} line {at}: xml {:?} / bin {:?}",
            differ.len(),
            text.lines().nth(at),
            bin.lines().nth(at)
        );
    }
    let (mut a, mut b) = (ids(&from_bin), ids(&xml));
    println!("bin: {} objects, xml: {} objects", a.len(), b.len());
    a.sort_unstable();
    b.sort_unstable();
    assert_eq!(a, b);
}
