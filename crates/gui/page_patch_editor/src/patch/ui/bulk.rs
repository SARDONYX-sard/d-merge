//! Above the user patch's editor: the conflicts that can be decided without picking one mod's value,
//! all at once. Nothing is shown while there is none, so it is in the way of nothing.
//!
//! ```text
//! [⚡ Resolve at once  12]
//!   ┌──────────────────────────────────────────────────┐
//!   │ [Select All] [Select None]           [Apply (12)] │
//!   │ ☑ 0_master  #1821/…/enterNotifyEvents            │
//!   │       Keep all events: Flinching, Impact          │
//!   │ ✓ 0_master  #1902/…/exitNotifyEvents   (decided)  │
//!   └──────────────────────────────────────────────────┘
//! ```
//!
//! The candidates come from the patch page's conflict check (`PatchPage::bulk_resolutions`). What
//! the user unchecked stays unchecked when they are found again.

use egui_widgets::{
    HoverMarkdown as _,
    shadcn::{Button, Checkbox},
};
use fast_hash::HashSet;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    patch::{BulkKind, BulkResolution, ResolveRequest, template_name},
};

/// How high the list gets before it scrolls.
const LIST_HEIGHT: f32 = 360.0;

/// The candidates, and which are unchecked.
#[derive(Debug, Default)]
pub(in crate::patch) struct BulkResolutions {
    list: Vec<BulkResolution>,
    /// By `(target, path)`: kept when the list is found again.
    unchecked: HashSet<(String, String)>,
}

impl BulkResolutions {
    pub(in crate::patch) fn set(&mut self, list: Vec<BulkResolution>) {
        self.list = list;
    }

    /// `requests` were taken in: they are decided, until the conflict check says so too.
    pub(in crate::patch) fn mark_resolved<'a>(&mut self, requests: impl Iterator<Item = &'a ResolveRequest>) {
        for request in requests {
            let same =
                |item: &&mut BulkResolution| item.request.target == request.target && item.request.path == request.path;
            if let Some(item) = self.list.iter_mut().find(same) {
                item.resolved = true;
            }
        }
    }

    /// Draws the button, if a candidate is left, and its list when open. Returns what to decide.
    pub(in crate::patch) fn bar_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) -> Option<Vec<ResolveRequest>> {
        let left = self.list.iter().filter(|item| !item.resolved).count();
        if left == 0 {
            return None;
        }

        let text = format!("⚡ {}  {left}", i18n.t(I18nKey::ConflictBulkButton));
        let button = ui.add(Button::new(text).small()).on_hover_markdown(i18n.t(I18nKey::ConflictBulkHover));
        let popup = egui::Popup::from_toggle_button_response(&button)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
        let picked = popup.show(|ui| self.list_ui(ui, i18n)).and_then(|shown| shown.inner);
        if picked.is_some() {
            egui::Popup::close_all(ui.ctx());
        }
        picked
    }

    /// `[Select All] [Select None] [Apply (N)]`, then a row per candidate.
    fn list_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) -> Option<Vec<ResolveRequest>> {
        let mut picked = None;
        ui.horizontal(|ui| {
            if ui.add(Button::new(i18n.t(I18nKey::SelectAll)).small()).clicked() {
                self.unchecked.clear();
            }
            if ui.add(Button::new(i18n.t(I18nKey::SelectNone)).small()).clicked() {
                self.unchecked = self.list.iter().map(key_of).collect();
            }
            let checked = self.checked().count();
            let apply = Button::new(format!("{} ({checked})", i18n.t(I18nKey::ApplyButton))).small();
            if ui.add_enabled(checked > 0, apply).clicked() {
                picked = Some(self.checked().map(|item| item.request.clone()).collect());
            }
        });
        ui.separator();

        egui::ScrollArea::vertical().max_height(LIST_HEIGHT).show(ui, |ui| {
            for item in &self.list {
                row_ui(ui, i18n, item, &mut self.unchecked);
            }
        });
        picked
    }

    /// The candidates to decide: checked, and not decided already.
    fn checked(&self) -> impl Iterator<Item = &BulkResolution> {
        self.list.iter().filter(|item| !item.resolved && !self.unchecked.contains(&key_of(item)))
    }
}

/// `☑ 0_master  #1821/…/enterNotifyEvents`, then what it does and with which mods.
fn row_ui(ui: &mut egui::Ui, i18n: &I18nMap, item: &BulkResolution, unchecked: &mut HashSet<(String, String)>) {
    let place = format!("{}  {}", template_name(&item.request.target), item.request.path);
    if item.resolved {
        ui.weak(format!("✓ {place}"));
        return;
    }

    let key = key_of(item);
    let mut on = !unchecked.contains(&key);
    if ui.add(Checkbox::new(&mut on, egui::RichText::new(place).monospace())).changed() {
        if on {
            unchecked.remove(&key);
        } else {
            unchecked.insert(key);
        }
    }
    let mods: Vec<&str> = item.request.sides.iter().map(|side| side.name.as_str()).collect();
    ui.indent(("bulk_row", &item.request.path), |ui| {
        ui.weak(format!("{}: {}", kind_label(i18n, item.kind), mods.join(", ")));
    });
}

/// What a kind does, e.g. `Keep all events`.
fn kind_label(i18n: &I18nMap, kind: BulkKind) -> &str {
    match kind {
        BulkKind::KeepAllEvents => i18n.t(I18nKey::ConflictKeepAllEvents),
    }
}

/// What a candidate is told apart by.
fn key_of(item: &BulkResolution) -> (String, String) {
    (item.request.target.clone(), item.request.path.clone())
}
