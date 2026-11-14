//! A small VS Code made of this crate and `egui_widgets`: run it to see how the parts fit, without
//! reading them.
//!
//! ```text
//! cargo run -p egui_code_editor --example vscode
//! ```
//!
//! - the activity bar (`ActivityBar`): Explorer / Search
//! - the explorer: a click opens a file in the preview tab (`PreviewTabs`), a double click keeps it
//! - the tabs (`EditorTab` + `action`): click, double click, `✖`, middle click, the right-click menu
//! - the editor (`CodeEditor`) and its status line; `Compare` opens a read-only diff tab (`DiffView`)
//! - the search of every file (`SearchView`): a click opens that line

use egui_code_editor::{
    CodeEditor, CodeEditorState, DiffView, Keymap, SearchLabels, SearchView, languages::Xml, text_pos::LinePos,
};
use egui_widgets::{ActivityBar, ActivityItem, EditorTab, TabAction, TabMenu, preview_tabs::PreviewTabs, shadcn::Icon};

/// A file: its text as opened (to compare with) and as edited.
struct Doc {
    name: &'static str,
    original: String,
    text: String,
    state: CodeEditorState,
}

/// What a tab shows.
enum Shown {
    Doc(usize),
    Diff { doc: usize, view: Box<DiffView> },
}

struct App {
    docs: Vec<Doc>,
    tabs: PreviewTabs<Shown>,
    search: SearchView,
    searching: bool,
}

fn main() -> eframe::Result {
    let doc = |name, text: &str| Doc {
        name,
        original: text.to_owned(),
        text: text.to_owned(),
        state: CodeEditorState::default(),
    };
    let mut app = App {
        docs: vec![
            doc("root.xml", "<hkobject name=\"#0090\">\n  <hkparam name=\"variant\">#0091</hkparam>\n</hkobject>\n"),
            doc(
                "graph.xml",
                "<hkobject name=\"#0091\">\n  <hkparam name=\"name\">BehaviorGraph</hkparam>\n</hkobject>\n",
            ),
        ],
        tabs: PreviewTabs::default(),
        search: SearchView::default(),
        searching: false,
    };
    eframe::run_ui_native("VS Code-like", eframe::NativeOptions::default(), move |ui, _| app.ui(ui))
}

impl App {
    fn ui(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("activity").resizable(false).exact_size(ActivityBar::SIZE).show(ui, |ui| {
            let items = [(Icon::FolderOpen, "Explorer", false), (Icon::Search, "Search", true)]
                .map(|(icon, hover, search)| ActivityItem { icon, hover, shown: self.searching == search, badge: 0 });
            if let Some(at) = ActivityBar::new().show(ui, items) {
                self.searching = at == 1;
            }
        });
        egui::Panel::left("side").default_size(220.0).show(ui, |ui| self.side_bar(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            self.tab_bar(ui);
            self.editor(ui);
        });
    }

    fn side_bar(&mut self, ui: &mut egui::Ui) {
        if self.searching {
            egui_widgets::side_bar_title(ui, "Search");
            let docs = self.docs.iter().map(|doc| (doc.name, doc.text.as_str()));
            self.search.refresh(self.docs.iter().map(|doc| doc.text.len()).sum::<usize>(), docs);
            let labels =
                SearchLabels { hint: "Search", regex_hover: "Regex", results_in: "results in", documents: "files" };
            let label = |job: &mut egui::text::LayoutJob, name: &str, visuals: &egui::Visuals| {
                job.append(name, 0.0, egui::TextFormat::simple(egui::FontId::monospace(12.0), visuals.text_color()));
            };
            if let Some((name, line)) = self.search.show(ui, &labels, &Xml, label) {
                let doc = self.docs.iter().position(|doc| *doc.name == *name).unwrap_or_default();
                self.open(doc, true);
                let id = egui_code_editor::editor_id(("doc", doc));
                let at = LinePos { line, column: 0 }.to_char_index(&self.docs[doc].text);
                egui_code_editor::set_cursor(ui.ctx(), id, at);
            }
            return;
        }
        egui_widgets::side_bar_title(ui, "Explorer");
        for at in 0..self.docs.len() {
            let response = egui_widgets::fitted_button(ui, false, self.docs[at].name);
            if response.clicked() || response.double_clicked() {
                self.open(at, response.double_clicked());
            }
        }
    }

    /// Shows `doc` in its tab, or the preview tab; `keep`: pinned.
    fn open(&mut self, doc: usize, keep: bool) {
        let at = self.tabs.open(0, Shown::Doc(doc), |shown| matches!(shown, Shown::Doc(d) if *d == doc), None);
        if keep {
            self.tabs.pin(at);
        }
        self.tabs.active = Some(at);
    }

    fn tab_bar(&mut self, ui: &mut egui::Ui) {
        let menu = TabMenu::from_texts(["Keep Open", "Close", "Close Others", "Close to the Right", "Close All"]);
        let mut picked = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (at, tab) in self.tabs.of(0) {
                let (name, read_only) = match &tab.item {
                    Shown::Doc(doc) => (self.docs[*doc].name, false),
                    Shown::Diff { doc, .. } => (self.docs[*doc].name, true),
                };
                let doc = match &tab.item {
                    Shown::Doc(doc) | Shown::Diff { doc, .. } => &self.docs[*doc],
                };
                let shown = EditorTab::new(name, self.tabs.active == Some(at))
                    .modified(!read_only && doc.text != doc.original)
                    .preview(!tab.pinned)
                    .read_only(read_only)
                    .show(ui);
                if let Some(action) = shown.action(&menu, |action| (at, action), |_| None) {
                    picked = Some(action);
                }
            }
        });
        match picked {
            Some((at, TabAction::Show)) => self.tabs.active = Some(at),
            Some((at, TabAction::KeepOpen)) => self.tabs.pin(at),
            Some((at, TabAction::Close)) => self.tabs.active = self.tabs.close(at),
            Some((at, TabAction::CloseOthers)) => self.tabs.active = self.tabs.close_others(at),
            Some((at, TabAction::CloseToTheRight)) => self.tabs.close_to_the_right(at),
            Some((_, TabAction::CloseAll)) => self.tabs.close_all(0),
            None => {}
        }
    }

    fn editor(&mut self, ui: &mut egui::Ui) {
        let Some(at) = self.tabs.active else {
            ui.weak("Open a file from the explorer.");
            return;
        };
        let mut compare = None;
        match &mut self.tabs[at].item {
            Shown::Doc(doc) => {
                let doc_at = *doc;
                let doc = &mut self.docs[doc_at];
                if ui.button("Compare with the file as opened").clicked() {
                    compare = Some(doc_at);
                }
                egui::Panel::bottom("status").show(ui, |ui| {
                    egui_code_editor::status_line(ui, &doc.state, Keymap::Normal, &doc.text);
                });
                egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                    CodeEditor::new(("doc", doc_at), &mut doc.text).language(&Xml).show(ui, &mut doc.state);
                });
            }
            Shown::Diff { doc, view } => {
                if let Some(line) = view.nav(ui, "Previous / next change") {
                    view.go_to(line);
                }
                view.show(ui, ("diff", *doc), &Xml, Keymap::Normal);
            }
        }
        if let Some(doc) = compare {
            let view = Box::new(DiffView::diff(&self.docs[doc].original, self.docs[doc].text.clone()));
            let at = self.tabs.open(0, Shown::Diff { doc, view }, |_| false, None);
            self.tabs.active = Some(at);
        }
    }
}
