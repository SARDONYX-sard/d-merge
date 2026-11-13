//! A tree driven by the keys, as VS Code's explorer: run it to see what this crate does.
//!
//! ```text
//! cargo run -p egui_tree --example tree
//! ```
//! `j` `k` `g` `G` move, `h` `l` close / open (or go up / in), `o` toggles; a click moves the cursor.
//! The host keeps its own rows; it gives the crate their depth and open state ([`NavRow`]), the lines
//! between them ([`guides`]), and draws each in a [`list::fixed_row`].

use egui_tree::{Nav, NavRow, TreeKey, guides, list};

/// A node of the host's own data.
struct Node {
    name: &'static str,
    open: bool,
    children: Vec<Self>,
}

/// One row on screen: a path of child indices to its node, and what the crate needs of it.
struct Row {
    path: Vec<usize>,
    nav: NavRow,
    lines: guides::GuideRow,
}

const fn node(name: &'static str, children: Vec<Node>) -> Node {
    Node { name, open: true, children }
}

fn main() -> eframe::Result {
    let leaf = |name| node(name, Vec::new());
    let mut root = vec![
        node("behaviors", vec![leaf("0_master.xml"), node("characters", vec![leaf("defaultmale.xml")])]),
        node("animations", vec![leaf("idle.hkx"), leaf("run.hkx")]),
        leaf("readme.txt"),
    ];
    let mut cursor = 0;
    eframe::run_ui_native("egui_tree", eframe::NativeOptions::default(), move |ui, _| {
        egui::CentralPanel::default().show(ui, |ui| {
            let mut rows = Vec::new();
            push_rows(&root, &mut Vec::new(), 0, &mut rows);
            for key in keys(ui) {
                let nav: Vec<NavRow> = rows.iter().map(|row| row.nav).collect();
                match egui_tree::navigate(key, &nav, cursor) {
                    Nav::Cursor(at) => cursor = at,
                    Nav::SetOpen(at, open) => node_at(&mut root, &rows[at].path).open = open,
                    Nav::Leave | Nav::None => {}
                }
                rows.clear();
                push_rows(&root, &mut Vec::new(), 0, &mut rows);
            }
            let height = list::row_height(ui);
            egui::ScrollArea::vertical().auto_shrink(false).show_rows(ui, height, rows.len(), |ui, range| {
                for at in range {
                    guides::paint(ui, rows[at].lines, at, (height, 12.0), &[]);
                    list::fixed_row(ui, height, |ui| {
                        ui.add_space(f32::from(rows[at].lines.depth) * 12.0);
                        let glyph = match rows[at].nav.open {
                            Some(true) => "▼ ",
                            Some(false) => "▶ ",
                            None => "  ",
                        };
                        let name = node_at(&mut root, &rows[at].path).name;
                        let response = ui.add(egui::Button::selectable(at == cursor, format!("{glyph}{name}")));
                        if response.clicked() {
                            cursor = at;
                        }
                        if at == cursor {
                            list::cursor_frame(ui, response.rect);
                        }
                    });
                }
            });
        });
    })
}

/// The keys pressed this frame, as the tree's. (An app maps its key bindings instead.)
fn keys(ui: &egui::Ui) -> Vec<TreeKey> {
    let map = [
        (egui::Key::J, TreeKey::Next),
        (egui::Key::K, TreeKey::Previous),
        (egui::Key::H, TreeKey::Close),
        (egui::Key::L, TreeKey::Open),
        (egui::Key::O, TreeKey::Toggle),
    ];
    ui.input(|input| {
        let mut keys: Vec<TreeKey> =
            map.iter().filter(|(key, _)| input.key_pressed(*key)).map(|(_, key)| *key).collect();
        if input.key_pressed(egui::Key::G) {
            keys.push(if input.modifiers.shift { TreeKey::Last } else { TreeKey::First });
        }
        keys
    })
}

/// The rows of `nodes` and, under the open ones, of their children.
fn push_rows(nodes: &[Node], path: &mut Vec<usize>, guides: u64, rows: &mut Vec<Row>) {
    let depth = u16::try_from(path.len()).unwrap_or(u16::MAX);
    for (at, node) in nodes.iter().enumerate() {
        let last_child = at + 1 == nodes.len();
        path.push(at);
        let open = (!node.children.is_empty()).then_some(node.open);
        let lines = guides::GuideRow { depth, last_child, guides };
        rows.push(Row { path: path.clone(), nav: NavRow { depth: usize::from(depth), open }, lines });
        if node.open {
            // The line of this level goes on past the children unless this is the last node.
            let below = if last_child { guides } else { guides | guides::bit(depth) };
            push_rows(&node.children, path, below, rows);
        }
        path.pop();
    }
}

/// The node of a row: `path` is never empty.
fn node_at<'a>(nodes: &'a mut [Node], path: &[usize]) -> &'a mut Node {
    let mut node = &mut nodes[path[0]];
    for &at in &path[1..] {
        node = &mut node.children[at];
    }
    node
}
