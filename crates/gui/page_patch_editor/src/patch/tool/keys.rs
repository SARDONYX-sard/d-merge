//! This frame's keys, as the key bindings read them ([`keybinding`]): each command asked for is
//! done once, by whoever takes it while the page is drawn.
//!
//! ```text
//! frame:  keys.frame(ctx, bindings, &context, i18n, notify)   the commands of this frame's keys
//! then:   if keys.take(commands::TABS_CLOSE) { .. }           done here; taken again: false
//!         keys.take_mapped(host::tree_key)                    all of a kind, e.g. the tree's
//! ```

use egui_widgets::notify::Notification;
use gui_shared::i18n::{I18nKey, I18nMap};

/// See the module docs.
#[derive(Debug, Default)]
pub(in crate::patch) struct Keys {
    /// The commands of this frame's keys, not taken yet.
    commands: Vec<&'static str>,
    /// The keys of the palette's actions, the `⚙` that changes them, and its recorder.
    pub(in crate::patch) ui: keybinding::KeyUi,
}

impl Keys {
    /// Reads this frame's keys. `bindings`: `None` when another has the keyboard (the main work's
    /// tool while the user patch window is open). While the `⚙` records new keys, it has them all.
    pub(in crate::patch) fn frame(
        &mut self,
        ctx: &egui::Context,
        bindings: Option<&mut keybinding::Keybindings>,
        context: &keybinding::KeyContext,
        (i18n, notify): (&I18nMap, &mut Notification),
    ) {
        self.commands.clear();
        let Some(bindings) = bindings else { return };

        let frame = self.ui.frame(ctx, bindings, context, recorder_texts(i18n));
        self.commands = frame.commands;
        match frame.saved {
            Some(Ok(())) => notify.success(i18n.t(I18nKey::KeybindingSaved)),
            Some(Err(err)) => notify.error(err),
            None => {}
        }
    }

    /// Was `command` asked for by this frame's keys? It is done then: asked again, `false`.
    pub(in crate::patch) fn take(&mut self, command: &str) -> bool {
        keybinding::host::take(&mut self.commands, command)
    }

    /// Takes every command `map` knows, in the order they came, as what `map` makes them.
    pub(in crate::patch) fn take_mapped<T>(&mut self, map: impl Fn(&str) -> Option<T>) -> Vec<T> {
        keybinding::host::take_mapped(&mut self.commands, map)
    }
}

/// The words of the key recorder, in the page's language.
fn recorder_texts(i18n: &I18nMap) -> keybinding::RecorderTexts<'_> {
    keybinding::RecorderTexts {
        hint: i18n.t(I18nKey::KeybindingRecordHint),
        also_bound: i18n.t(I18nKey::KeybindingAlsoBound),
        edit_json: i18n.t(I18nKey::KeybindingEditInJson),
        labels: egui_code_editor::EditorLabels::from_texts(gui_shared::i18n::editor_label_texts(i18n)),
        unsaved: i18n.t(I18nKey::KeybindingUnsaved),
        save: i18n.t(I18nKey::KeybindingSave),
        dont_save: i18n.t(I18nKey::KeybindingDontSave),
        cancel: i18n.t(I18nKey::CancelButton),
    }
}
