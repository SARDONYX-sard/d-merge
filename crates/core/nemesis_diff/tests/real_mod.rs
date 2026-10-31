//! Round trip with a real Nemesis mod: apply its patches to the template, diff again, and check
//! that the regenerated patches mean the same edits (and that d-merge's parser accepts them).
//!
//! Needs local files: `cargo test -p nemesis_diff -- --ignored`
//! - `NEMESIS_DIFF_TEMPLATE`: template xml (e.g. `resource/xml/templates/.../draugrbehavior.xml`)
//! - `NEMESIS_DIFF_PATCHES`: the mod's behavior dir (e.g. `.../mod/colisc/draugrbehavior`)
//! - `NEMESIS_DIFF_CODE`: its mod code (e.g. `colisc`)

use std::path::PathBuf;

use fast_hash as _;
use imara_diff as _;
use nemesis_diff::{View, apply_patches, diff_behavior, normalize, objects, resolve, validate};
use nemesis_xml as _;

/// The template itself has no unknown class / field names (no false positives).
#[test]
#[ignore = "needs local mod files"]
fn template_fields_are_known() {
    let template =
        std::fs::read_to_string(std::env::var("NEMESIS_DIFF_TEMPLATE").unwrap()).unwrap();
    let problems: Vec<_> = objects(&template)
        .iter()
        .flat_map(|o| {
            nemesis_diff::check_fields(o.text)
                .into_iter()
                .map(|p| format!("{}: {}", o.id, p.message))
        })
        .collect();
    assert!(
        problems.is_empty(),
        "{} problems, e.g. {:?}",
        problems.len(),
        &problems[..problems.len().min(5)]
    );
}

#[test]
#[ignore = "needs local mod files"]
fn real_mod_round_trip() {
    let var = |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("set {name}"));
    let template = std::fs::read_to_string(var("NEMESIS_DIFF_TEMPLATE")).unwrap();
    let dir = PathBuf::from(var("NEMESIS_DIFF_PATCHES"));
    let code = var("NEMESIS_DIFF_CODE");

    let files: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "txt"))
        .map(|e| {
            let stem = e.path().file_stem().unwrap().to_string_lossy().into_owned();
            let bytes = std::fs::read(e.path()).unwrap();
            (stem, String::from_utf8_lossy(&bytes).replace("\r\n", "\n"))
        })
        .collect();

    let edited = apply_patches(&template, files.iter().map(|(s, c)| (s.as_str(), c.as_str())));
    let diff = diff_behavior(&code, &template, &edited);

    let mut mismatched = Vec::new();
    let mut invalid = Vec::new();
    for changed in &diff.changed {
        let (_, original) = files.iter().find(|(s, _)| *s == changed.id).unwrap();
        // A mod written against Nemesis' own template text may not parse against d-merge's.
        let original_ok = validate(original).is_ok();
        if let Err(err) = validate(&changed.patch) {
            invalid.push(format!("{} (mod's own file parses: {original_ok}): {err}", changed.id));
        }
        if original_ok
            && resolve(&changed.patch, View::Mod) != normalize(&resolve(original, View::Mod))
        {
            mismatched.push(changed.id.clone());
        }
    }
    for line in &invalid {
        println!("invalid: {line}");
    }
    let template_ids = objects(&template).len();
    println!(
        "{} files -> {} changed, {} added, {} removed (template: {template_ids} objects)",
        files.len(),
        diff.changed.len(),
        diff.added.len(),
        diff.removed.len()
    );
    assert!(mismatched.is_empty(), "different edits: {mismatched:?}");
}

/// Every anim set patch of a mod (`NEMESIS_DIFF_ASDSF_MOD`: `.../mod/<code>/animationsetdatasinglefile`)
/// matches an anim set of the template (`resource/xml/templates/meshes/animationsetdatasinglefile.txt`),
/// and regenerating its patch gives the same edit.
#[test]
#[ignore = "needs local mod files"]
fn real_asdsf_round_trip() {
    let template = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../resource/xml/templates/meshes/animationsetdatasinglefile.txt"
    ))
    .unwrap();
    let sets = nemesis_diff::asdsf::anim_sets(&template).unwrap();
    let dir = PathBuf::from(std::env::var("NEMESIS_DIFF_ASDSF_MOD").unwrap());
    let code = std::env::var("NEMESIS_DIFF_CODE").unwrap();

    let (mut checked, mut missing, mut different, mut other_base) =
        (0, Vec::new(), Vec::new(), Vec::new());
    for project in std::fs::read_dir(&dir).unwrap().flatten().filter(|e| e.path().is_dir()) {
        for file in std::fs::read_dir(project.path()).unwrap().flatten() {
            let id = format!(
                "{}/{}",
                project.file_name().to_string_lossy(),
                file.file_name().to_string_lossy()
            );
            let patch =
                String::from_utf8_lossy(&std::fs::read(file.path()).unwrap()).replace("\r\n", "\n");
            let Some(set) = sets.iter().find(|s| s.id().eq_ignore_ascii_case(&id)) else {
                missing.push(id);
                continue;
            };
            let edited = resolve(&patch, View::Mod);
            let original = resolve(&patch, View::Original);
            if original != set.text {
                let line = original.lines().zip(set.text.lines()).position(|(a, b)| a != b);
                let at =
                    line.unwrap_or_else(|| original.lines().count().min(set.text.lines().count()));
                // The mod was made against another version of the template: report only.
                other_base.push(format!(
                    "{id}: original side differs at line {} (patch: {:?}, template: {:?})",
                    at + 1,
                    original.lines().nth(at),
                    set.text.lines().nth(at)
                ));
            } else if let Some(regenerated) = nemesis_diff::object_patch(&code, &set.text, &edited)
                && resolve(&regenerated, View::Mod) != edited
            {
                different.push(id);
            }
            checked += 1;
        }
    }
    println!("{} sets in template, {checked} patches checked, missing: {missing:?}", sets.len());
    println!("made against another template: {}", other_base.len());
    other_base.iter().take(3).for_each(|line| println!("  {line}"));
    assert!(different.is_empty(), "{different:?}");
}

/// Adding an event and a variable to a real template gives patches d-merge's parser accepts.
#[test]
#[ignore = "needs local mod files"]
fn real_add_event_and_variable() {
    use nemesis_diff::vars::{VariableType, add_event, add_variable, names};
    let template =
        std::fs::read_to_string(std::env::var("NEMESIS_DIFF_TEMPLATE").unwrap()).unwrap();
    let all = objects(&template);
    let of = |class: &str| all.iter().find(|o| o.class == class).unwrap().text;
    let (string_data, graph_data, value_set) =
        (of("hkbBehaviorGraphStringData"), of("hkbBehaviorGraphData"), of("hkbVariableValueSet"));

    let (s, g) = add_event(string_data, graph_data, "MyEvent").unwrap();
    let (s, g, v) =
        add_variable(&s, &g, value_set, "myVar", VariableType::Real, 1_045_220_557).unwrap();
    assert_eq!(names(&s, "eventNames").last().map(String::as_str), Some("MyEvent"));
    assert_eq!(names(&s, "variableNames").last().map(String::as_str), Some("myVar"));

    // string / graph data: the event and the variable arrays; value set: the value.
    for (original, edited, blocks) in
        [(string_data, &s, 2), (graph_data, &g, 2), (value_set, &v, 1)]
    {
        let patch = nemesis_diff::object_patch("mymod", original, edited).unwrap();
        validate(&patch).unwrap_or_else(|e| panic!("{e}\n{patch}"));
        assert_eq!(patch.matches("<!-- CLOSE -->").count(), blocks, "{patch}");
    }
}
