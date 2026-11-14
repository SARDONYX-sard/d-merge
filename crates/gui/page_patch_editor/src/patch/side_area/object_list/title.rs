//! The OBJECTS title's buttons, right to left `[⋯] [≡] [+]` (VS Code's order is `[+] [≡] [⋯]`).
//! They see only [`Title`], and return what was clicked ([`TitleAsk`]).
//!
//! ```text
//! [+]   a new object (a class to pick), or new clips for the clip data
//! [≡]   tree / flat list; for animation data: expand all / collapse all
//! [⋯]   changed only, unlinked only, the Pandora file, duplicate / revert the selected object
//! ```

use std::rc::Rc;

use egui_widgets::shadcn::{Button, Icon};
use gui_shared::i18n::{I18nKey, I18nMap};

/// What the buttons show.
pub(super) struct Title<'a> {
    /// The active behavior is animation data; `adsf`: its clip data.
    pub(super) anim: bool,
    pub(super) adsf: bool,
    pub(super) tree_view: bool,
    /// The mod code is one new objects can be named by.
    pub(super) code_ok: bool,
    /// The selected object, and whether it is new (deleted, not reverted) or the Pandora file.
    pub(super) selected: Option<&'a Rc<str>>,
    pub(super) selected_is_new: bool,
    pub(super) selected_is_pandora_doc: bool,
    /// The `⋯` menu's filters: changed at once.
    pub(super) changed_only: &'a mut bool,
    pub(super) unlinked_only: &'a mut bool,
}

/// What a button asked for.
#[derive(Debug)]
pub(super) enum TitleAsk {
    ToggleTreeView,
    ExpandAll,
    CollapseAll,
    OpenPandoraDoc,
    Duplicate(Rc<str>),
    RevertOrDelete(Rc<str>),
    /// `[+]`: a class to pick, or new clips.
    New,
}

/// Draws the buttons, right to left.
pub(super) fn buttons(ui: &mut egui::Ui, i18n: &I18nMap, title: Title<'_>) -> Option<TitleAsk> {
    let selected = selected_items(&title);
    let more = more_menu(ui, i18n, title.anim, (title.changed_only, title.unlinked_only), selected);
    let view = view_buttons(ui, i18n, title.anim, title.tree_view);
    let new = new_button(ui, i18n, (title.anim, title.adsf), title.code_ok);
    more.or(view).or(new)
}

/// `⋯`'s items about the selected object: `(object, it is new, can it be duplicated)`.
fn selected_items(title: &Title<'_>) -> Option<(Rc<str>, bool, bool)> {
    let selected = Rc::clone(title.selected?);
    let can_copy = !title.anim && !title.selected_is_pandora_doc && title.code_ok;
    Some((selected, title.selected_is_new, can_copy))
}

/// `[≡]`: a tree or a flat list. Animation data: collapse / expand all instead.
fn view_buttons(ui: &mut egui::Ui, i18n: &I18nMap, anim: bool, tree_view: bool) -> Option<TitleAsk> {
    if anim {
        let expand = Button::new("").icon(Icon::ChevronDown).small();
        let collapse = Button::new("").icon(Icon::ChevronRight).small();
        if ui.add(expand).on_hover_text(i18n.t(I18nKey::ExpandAllButton)).clicked() {
            return Some(TitleAsk::ExpandAll);
        }
        if ui.add(collapse).on_hover_text(i18n.t(I18nKey::CollapseAllButton)).clicked() {
            return Some(TitleAsk::CollapseAll);
        }
        return None;
    }

    let tree = Button::selectable(tree_view, "").icon(Icon::ListTree).small();
    ui.add(tree).on_hover_text(i18n.t(I18nKey::NemesisTreeViewHover)).clicked().then_some(TitleAsk::ToggleTreeView)
}

/// `[⋯]`, as VS Code's "More Actions": what is used less.
fn more_menu(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    anim: bool,
    (changed_only, unlinked_only): (&mut bool, &mut bool),
    selected: Option<(Rc<str>, bool, bool)>,
) -> Option<TitleAsk> {
    let mut asked = None;
    ui.menu_button("⋯", |ui| {
        menu_items(ui, i18n, anim, (changed_only, unlinked_only), selected, &mut asked);
        if asked.is_some() {
            ui.close();
        }
    });
    asked
}

/// The items of `[⋯]`; what was clicked goes to `asked`.
fn menu_items(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    anim: bool,
    (changed_only, unlinked_only): (&mut bool, &mut bool),
    selected: Option<(Rc<str>, bool, bool)>,
    asked: &mut Option<TitleAsk>,
) {
    {
        ui.checkbox(changed_only, i18n.t(I18nKey::NemesisChangedOnly));
        if !anim {
            ui.checkbox(unlinked_only, i18n.t(I18nKey::NemesisUnlinkedOnly));
            ui.separator();
            if ui.button(i18n.t(I18nKey::NemesisPandoraDocHover)).clicked() {
                *asked = Some(TitleAsk::OpenPandoraDoc);
            }
        }

        let Some((selected, is_new, can_copy)) = selected else { return };
        ui.separator();
        if ui.add_enabled(can_copy, Button::new(i18n.t(I18nKey::NemesisDuplicate))).clicked() {
            *asked = Some(TitleAsk::Duplicate(Rc::clone(&selected)));
        }
        let label = if is_new { I18nKey::NemesisDeleteObject } else { I18nKey::RevertButton };
        if ui.add(Button::new(i18n.t(label)).destructive()).clicked() {
            *asked = Some(TitleAsk::RevertOrDelete(selected));
        }
    }
}

/// `[+]`: a new object of a class to pick; for clip data, new clips. Anim sets can only be edited.
fn new_button(ui: &mut egui::Ui, i18n: &I18nMap, (anim, adsf): (bool, bool), code_ok: bool) -> Option<TitleAsk> {
    if anim && !adsf {
        return None;
    }

    let hover = if adsf { I18nKey::NemesisNewClipsHover } else { I18nKey::NemesisNewObject };
    let button = Button::new("").icon(Icon::Plus).small();
    ui.add_enabled(code_ok, button)
        .on_hover_text(i18n.t(hover))
        .on_disabled_hover_text(i18n.t(I18nKey::NemesisModCodeInvalid))
        .clicked()
        .then_some(TitleAsk::New)
}
