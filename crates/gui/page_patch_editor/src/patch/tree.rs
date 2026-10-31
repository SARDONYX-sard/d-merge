//! Object tree: from the root (`hkRootLevelContainer`) down the `#XXXX` pointers in each object.
//!
//! ```text
//! ▼ #0090 hkRootLevelContainer
//!   ▼ #0091 hkbBehaviorGraph  BehaviorGraph
//!     ▶ #0094 hkbBehaviorGraphData
//!     ▶ #0095 hkbStateMachine  Root
//!       ...  #0097 (↻ shown above)
//! ```
//! Children are read from the current text (edits and new objects included). An object shown
//! once is a (dimmed) leaf at its next places, so shared nodes and cycles cannot blow up the tree.
//!
//! Right click a node: expand / collapse everything below it.

use fast_hash::HashSet;

use super::workspace::Behavior;

/// A pending "expand / collapse everything below" request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExpandAll {
    /// Path (ids from the root) of the node it was asked on.
    path: Vec<String>,
    open: bool,
    /// Children appear only once their parent is open, so it runs for a few frames.
    frames_left: u8,
}

/// Tree state kept between frames.
#[derive(Debug, Default)]
pub(super) struct TreeState {
    expand_all: Option<ExpandAll>,
}

/// Draws the tree; a click selects an object (`selected`).
pub(super) fn tree_ui(
    ui: &mut egui::Ui,
    behavior: &Behavior,
    selected: &mut Option<String>,
    state: &mut TreeState,
) {
    let root = behavior
        .object_of_class("hkRootLevelContainer")
        .or_else(|| behavior.objects.first().map(|o| o.id.as_str()));
    let Some(root) = root else { return };
    let mut walk = Walk { behavior, selected, state, path: Vec::new(), shown: HashSet::default() };
    walk.node(ui, root);

    if let Some(request) = &mut walk.state.expand_all {
        request.frames_left = request.frames_left.saturating_sub(1);
        if request.frames_left == 0 {
            walk.state.expand_all = None;
        } else {
            ui.ctx().request_repaint();
        }
    }
}

struct Walk<'a> {
    behavior: &'a Behavior,
    selected: &'a mut Option<String>,
    state: &'a mut TreeState,
    /// Ids from the root to the current node.
    path: Vec<String>,
    /// Nodes already drawn this frame.
    shown: HashSet<String>,
}

impl Walk<'_> {
    fn node(&mut self, ui: &mut egui::Ui, id: &str) {
        let behavior = self.behavior;
        let text = behavior.text(id).unwrap_or_default();
        let label = egui::RichText::new(label(behavior, id, text)).monospace();
        let is_selected = self.selected.as_deref() == Some(id);

        let children: Vec<String> =
            pointers(text, id).into_iter().filter(|child| behavior.text(child).is_some()).collect();
        let seen = !self.shown.insert(id.to_string());
        if children.is_empty() || seen {
            let label = if seen { label.weak() } else { label };
            ui.horizontal(|ui| {
                ui.add_space(ui.spacing().indent);
                if ui.add(egui::Button::selectable(is_selected, label)).clicked() {
                    *self.selected = Some(id.to_string());
                }
            });
            return;
        }

        // The id path makes the open state unique per position (an object can have several parents).
        let state_id = ui.make_persistent_id((self.path.as_slice(), id));
        let mut collapsing = egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            state_id,
            self.path.is_empty(),
        );
        if let Some(request) = &self.state.expand_all
            && self.path.len() >= request.path.len().saturating_sub(1)
            && is_below(&self.path, id, &request.path)
        {
            collapsing.set_open(request.open);
        }

        let mut request = None;
        collapsing
            .show_header(ui, |ui| {
                let response = ui.add(egui::Button::selectable(is_selected, label));
                if response.clicked() {
                    *self.selected = Some(id.to_string());
                }
                response.context_menu(|ui| {
                    if ui.button("⊞ Expand all below").clicked() {
                        request = Some(true);
                    }
                    if ui.button("⊟ Collapse all below").clicked() {
                        request = Some(false);
                    }
                });
            })
            .body(|ui| {
                self.path.push(id.to_string());
                for child in &children {
                    self.node(ui, child);
                }
                self.path.pop();
            });
        if let Some(open) = request {
            let mut path = self.path.clone();
            path.push(id.to_string());
            self.state.expand_all = Some(ExpandAll { path, open, frames_left: 60 });
        }
    }
}

/// Is the node `path` + `id` the requested node or below it?
fn is_below(path: &[String], id: &str, requested: &[String]) -> bool {
    let full = path.iter().map(String::as_str).chain(std::iter::once(id));
    requested.len() <= path.len() + 1 && full.zip(requested).all(|(a, b)| a == b)
}
/// `● #0106 hkbModifierList GetUpModList`
fn label(behavior: &Behavior, id: &str, text: &str) -> String {
    let marker = if behavior.added.contains_key(id) {
        '+'
    } else if behavior.edits.contains_key(id) {
        '●'
    } else {
        ' '
    };
    let class = attribute(text, "class").unwrap_or_default();
    let name = text
        .split_once("<hkparam name=\"name\">")
        .and_then(|(_, rest)| rest.split_once("</hkparam>"))
        .map_or("", |(name, _)| name);
    format!("{marker} {id}  {class}  {name}")
}

fn attribute<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let start = text.find(&key)? + key.len();
    text[start..].split('"').next()
}

/// The `#XXXX` / `#mod$N` pointers of an object (not its own name), first occurrence order.
fn pointers(text: &str, own: &str) -> Vec<String> {
    let is_id_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    let mut found: Vec<String> = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find('#') {
        let len = rest[at + 1..].find(|c: char| !is_id_char(c)).unwrap_or(rest.len() - at - 1);
        let token = &rest[at..at + 1 + len];
        if len > 0 && token != own && !found.iter().any(|f| f == token) {
            found.push(token.to_string());
        }
        rest = &rest[at + 1 + len..];
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_pointers() {
        let text =
            "<hkobject name=\"#0106\" class=\"x\">\n#0105 #0109\n#mymod$0 #0105\n</hkobject>\n";
        assert_eq!(pointers(text, "#0106"), ["#0105", "#0109", "#mymod$0"]);
    }
}
