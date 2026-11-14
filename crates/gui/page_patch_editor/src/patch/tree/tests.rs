//! The object tree's tests: its graph and its rows together.

use super::*;

/// How long the walk over all the nodes takes on the largest template: what a pointer edit costs.
#[test]
#[ignore = "needs the template files of the repository; prints timings"]
fn time_the_walk_on_the_largest_template() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../resource/assets/templates/meshes/actors/character/behaviors/0_master.bin");
    let behavior = Behavior::open(&path).unwrap();

    let start = std::time::Instant::now();
    let mut graph = Graph::new(&behavior);
    println!("{} nodes, whole build: {:?}", graph.entries.len(), start.elapsed());

    const RUNS: u32 = 100;
    let start = std::time::Instant::now();
    for _ in 0..RUNS {
        graph.links_dirty = true;
        graph.finish_updates();
    }
    println!("walk: {:?} each", start.elapsed() / RUNS);
}

fn template<'a>(id: &'a str, text: &'a str) -> ObjectText<'a> {
    ObjectText { id, class: "c", name: "", text, is_new: false, is_edited: false }
}

fn new_object<'a>(id: &'a str, text: &'a str) -> ObjectText<'a> {
    ObjectText { id, class: "c", name: "", text, is_new: true, is_edited: false }
}

/// `#new0` is reached through `#b`, `#new1` through nothing. `#c` is shared by `#a` and `#b`.
fn sample() -> Graph {
    let objects = [
        new_object("#new0", ""),
        new_object("#new1", ""),
        template("#root", "#a #b"),
        template("#a", "#c"),
        template("#b", "#c #new0"),
        template("#c", ""),
    ];
    Graph::from_objects(&objects, Some("#root"))
}

/// The tree of `graph` as the page shows it.
const fn source(graph: &Graph, changed_only: bool) -> ObjectTree<'_> {
    ObjectTree { graph, changed_only }
}

fn ids<'a>(graph: &'a Graph, state: &TreeState) -> Vec<&'a str> {
    state.rows().iter().map(|row| graph.id(row.node)).collect()
}

fn unlinked(graph: &Graph) -> Vec<&str> {
    graph.unlinked().iter().map(|&node| graph.id(node)).collect()
}

/// Everything open: the whole tree as `(id, dimmed)`.
fn whole_tree(graph: &Graph) -> Vec<(String, bool)> {
    let mut state = TreeState::default();
    state.prepare(&source(graph, false), None);
    if let Some(root) = graph.root {
        state.set_open_below(&source(graph, false), root, true);
    }
    state.prepare(&source(graph, false), None);
    state.rows().iter().map(|row| (graph.id(row.node).to_owned(), row.dimmed)).collect()
}

#[test]
fn finds_pointers() {
    let text = "<hkobject name=\"#0106\" class=\"x\">\n#0105 #0109\n#mymod$0 #0105\n</hkobject>\n";
    assert_eq!(pointers(text, "#0106"), ["#0105", "#0109", "#mymod$0"]);
}

/// The "not linked" count must not depend on what is expanded.
#[test]
fn unlinked_does_not_depend_on_open_nodes() {
    let graph = sample();
    assert_eq!(unlinked(&graph), ["#new1"]);

    let mut state = TreeState::default();
    state.prepare(&source(&graph, false), None);
    assert_eq!(ids(&graph, &state), ["#root", "#a", "#b"]); // only the root starts open
    assert_eq!(unlinked(&graph), ["#new1"]);
}

/// The list's "unlinked only" asks each node: it must agree with the list of them.
#[test]
fn is_unlinked_agrees_with_the_unlinked_list() {
    let graph = sample();
    let asked: Vec<&str> = graph.nodes().filter(|&node| graph.is_unlinked(node)).map(|node| graph.id(node)).collect();
    assert_eq!(asked, unlinked(&graph));
}

#[test]
fn labels_tell_where_new_objects_are_linked() {
    let graph = sample();
    let new0 = graph.node_of("#new0").unwrap();
    assert_eq!(graph.label(new0), "+ #new0  c   ← #b");
    assert_eq!(graph.doc(new0), "c   ← #b");
    assert_eq!(graph.label(graph.node_of("#new1").unwrap()), "+ #new1  c   ⚠ not linked");
}

/// `#c` has two parents: it expands under the first one and is dimmed under the other.
#[test]
fn shared_objects_expand_once() {
    let tree = whole_tree(&sample());
    let expected = [("#root", false), ("#a", false), ("#c", false), ("#b", false), ("#c", true), ("#new0", false)];
    assert_eq!(tree, expected.map(|(id, dimmed)| (id.to_owned(), dimmed)));
}

/// Updating one object at a time must give what building from scratch gives.
#[test]
fn updates_match_a_full_build() {
    let mut graph = sample();

    // Link the unlinked object.
    graph.update("#a", Some(template("#a", "#c #new1")));
    graph.finish_updates();
    assert!(unlinked(&graph).is_empty());
    assert_eq!(graph.label(graph.node_of("#new1").unwrap()), "+ #new1  c   ← #a");

    // Add one that was mentioned before it existed.
    graph.update("#c", Some(template("#c", "#new2")));
    graph.update("#new2", Some(new_object("#new2", "")));
    graph.finish_updates();
    assert_eq!(graph.label(graph.node_of("#new2").unwrap()), "+ #new2  c   ← #c");

    // Delete one: it leaves the list and the tree.
    graph.update("#new0", None);
    graph.finish_updates();
    assert!(graph.node_of("#new0").is_none());

    let fresh = Graph::from_objects(
        &[
            new_object("#new1", ""),
            new_object("#new2", ""),
            template("#root", "#a #b"),
            template("#a", "#c #new1"),
            template("#b", "#c #new0"),
            template("#c", "#new2"),
        ],
        Some("#root"),
    );
    assert_eq!(whole_tree(&graph), whole_tree(&fresh));
    assert_eq!(unlinked(&graph), unlinked(&fresh));
    let ids_of = |graph: &Graph| graph.nodes().map(|node| graph.id(node).to_owned()).collect::<Vec<_>>();
    assert_eq!(ids_of(&graph)[..2], ["#new1", "#new2"]);
    assert_eq!(ids_of(&graph).len(), ids_of(&fresh).len());
}

/// Typing that leaves the pointers alone must not cost a walk or a rebuild of the rows.
#[test]
fn typing_keeps_the_tree_generation() {
    let mut graph = sample();
    let tree_generation = graph.tree_generation();
    let generation = graph.generation();

    graph.update("#a", Some(template("#a", "#c <hkparam name=\"name\">x</hkparam>")));
    graph.finish_updates();

    assert_eq!(graph.tree_generation(), tree_generation);
    assert_ne!(graph.generation(), generation);
    assert_eq!(graph.label(graph.node_of("#a").unwrap()), "  #a  c  x");

    // The first edit of an object does change the tree: it joins "changed only".
    let edited = ObjectText { is_edited: true, ..template("#a", "#c") };
    graph.update("#a", Some(edited));
    graph.finish_updates();
    assert_ne!(graph.tree_generation(), tree_generation);
}

#[test]
fn changed_only_shows_the_way_to_changes() {
    let mut graph = sample();
    let edited = ObjectText { is_edited: true, ..template("#c", "") };
    graph.update("#c", Some(edited));
    graph.finish_updates();
    let mut state = TreeState::default();
    state.prepare(&source(&graph, true), None);
    // `#new0` is new, `#c` edited; `#a`, `#b` lead to them.
    assert_eq!(ids(&graph, &state), ["#root", "#a", "#c", "#b", "#c", "#new0"]);
}

#[test]
fn knows_the_way_to_a_row() {
    let graph = sample();
    let mut state = TreeState::default();
    state.prepare(&source(&graph, false), None);
    state.set_open_below(&source(&graph, false), graph.node_of("#root").unwrap(), true);
    state.prepare(&source(&graph, false), None);
    // 0 #root, 1 #a, 2 #c, 3 #b, 4 #c (dimmed), 5 #new0
    assert_eq!(state.ancestors(5), [0, 3]);
    assert_eq!(state.ancestors(4), [0, 3]);
    assert!(state.ancestors(0).is_empty());
    assert!(state.ancestors(99).is_empty());

    state.cursor = 5;
    assert_eq!(state.active_guides(), [Some(1..=3), Some(4..=5)]);

    // Scrolled so that `#new0` is the top row: its parents get pinned.
    state.scroll.offset = 50.0;
    assert_eq!(state.sticky_rows(10.0, 5), [0, 3]);
    assert_eq!(state.sticky_rows(10.0, 1), [3]); // only the nearest fit
    state.scroll.offset = 0.0;
    assert_eq!(state.sticky_rows(10.0, 5), Vec::<usize>::new());
}

#[test]
fn rows_know_their_connecting_lines() {
    let graph = sample();
    let mut state = TreeState::default();
    state.prepare(&source(&graph, false), None);
    state.set_open_below(&source(&graph, false), graph.node_of("#root").unwrap(), true);
    state.prepare(&source(&graph, false), None);
    let rows = state.rows();

    assert_eq!(rows.iter().map(|row| row.last_child).collect::<Vec<_>>(), [true, false, true, true, false, true]);
    // `#c` under `#a`: the line of level 0 goes on to `#b`.
    assert!(egui_tree::guides::has(rows[2].guides, 0));
    // `#c` under `#b`: `#b` is the last child, so level 0 has ended; level 1 goes on to `#new0`.
    assert!(!egui_tree::guides::has(rows[4].guides, 0));
    assert!(egui_tree::guides::has(rows[4].guides, 1));
}
