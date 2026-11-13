//! Draws the command palette and reads the keys it and the editor's commands use.
//! The logic is in [`crate::palette`].

use egui::{Key, Modifiers};

use crate::palette::{
    Command, EditorAction, EditorLabels, Palette, PaletteContext, PaletteKeys, Picked, Triggers, commands,
};

/// What the palette needs from the editor this frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PaletteInput<'a, 'b> {
    pub(crate) id: egui::Id,
    pub(crate) has_focus: bool,
    pub(crate) context: PaletteContext,
    pub(crate) labels: &'b EditorLabels<'a>,
    pub(crate) actions: &'a [EditorAction<'a>],
    /// See [`crate::CodeEditor::requests`].
    pub(crate) requests: Option<&'b [crate::EditorRequest]>,
}

/// Opens on `Ctrl+Shift+P`; while open, handles its keys and draws. Call before the keymap
/// (Vim uses `Ctrl+P` itself) and the `TextEdit`. Returns the command that was run.
pub(crate) fn show<'a>(palette: &mut Palette, ui: &egui::Ui, input: PaletteInput<'a, '_>) -> Option<Picked<'a>> {
    let asked = match input.requests {
        Some(_) => crate::request::asked(input.requests, crate::EditorRequest::CommandPalette),
        None => input.has_focus && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::P)),
    };
    if asked {
        open(palette, ui, input.id);
    }
    if !palette.open {
        return None;
    }
    let commands = commands(&palette.query, input.context, input.labels, input.actions);
    let keys = read_keys(ui);
    let from_keys = palette.on_keys(keys, &commands);
    let clicked = draw(palette, ui, input, &commands);
    let picked = from_keys.or(clicked);
    close_if_done(palette, ui, input.id, picked.is_some() || keys.escape);
    picked
}

/// Opens the palette with the keyboard in its field (when it is drawn). `editor`: the editor it
/// belongs to; the keyboard leaves it now, so that it takes no key meanwhile.
pub(crate) fn open(palette: &mut Palette, ui: &egui::Ui, editor: egui::Id) {
    palette.reopen();
    ui.memory_mut(|m| m.surrender_focus(editor));
}

/// The keys the editor takes in any keymap, whether or not the palette is open.
///
/// Only with `has_focus`, and before the keymap, which would swallow them. `Shift+F12` is tried
/// before `F12`: `consume_key` ignores extra Shift / Alt.
/// With `requests` (see [`crate::CodeEditor::requests`]) the keys are the host's: what it asked for.
pub(crate) fn read_triggers(ui: &egui::Ui, has_focus: bool, requests: Option<&[crate::EditorRequest]>) -> Triggers {
    use crate::{EditorRequest as Request, request::asked};
    if requests.is_some() {
        let navigate = if asked(requests, Request::FindReferences) {
            Some(true)
        } else {
            asked(requests, Request::GoToDefinition).then_some(false)
        };
        return Triggers {
            quick_fix: asked(requests, Request::QuickFix),
            navigate,
            format: asked(requests, Request::Format),
            completion: asked(requests, Request::Completion),
            ..Triggers::default()
        };
    }
    if !has_focus {
        return Triggers::default();
    }
    ui.input_mut(|i| Triggers {
        quick_fix: i.consume_key(Modifiers::COMMAND, Key::Period),
        navigate: if i.consume_key(Modifiers::SHIFT, Key::F12) {
            Some(true)
        } else {
            i.consume_key(Modifiers::NONE, Key::F12).then_some(false)
        },
        format: i.consume_key(Modifiers::SHIFT | Modifiers::ALT, Key::F),
        completion: i.consume_key(Modifiers::COMMAND, Key::Space),
        ..Triggers::default()
    })
}

fn read_keys(ui: &egui::Ui) -> PaletteKeys {
    ui.input_mut(|i| PaletteKeys {
        up: i.consume_key(Modifiers::NONE, Key::ArrowUp) || i.consume_key(Modifiers::COMMAND, Key::P),
        down: i.consume_key(Modifiers::NONE, Key::ArrowDown) || i.consume_key(Modifiers::COMMAND, Key::N),
        enter: i.consume_key(Modifiers::NONE, Key::Enter),
        escape: i.consume_key(Modifiers::NONE, Key::Escape),
    })
}

/// Closes after a pick, `Esc` (the editor gets the keyboard back) or a click outside (the click
/// may have focused something else, so the focus is left alone).
fn close_if_done(palette: &mut Palette, ui: &egui::Ui, editor: egui::Id, done: bool) {
    let clicked_outside = ui.input(|i| {
        let click = i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten();
        click.is_some_and(|pos| palette.rect.is_some_and(|rect| !rect.contains(pos)))
    });
    if clicked_outside {
        palette.open = false;
    } else if done {
        palette.open = false;
        ui.memory_mut(|m| m.request_focus(editor));
    }
}

/// Draws the field and the list at the top of the view. Returns the command clicked.
fn draw<'a>(
    palette: &mut Palette,
    ui: &egui::Ui,
    input: PaletteInput<'a, '_>,
    commands: &[Command<'a>],
) -> Option<Picked<'a>> {
    let mut clicked = None;
    let top = ui.clip_rect().center_top() + egui::vec2(0.0, 8.0);
    let area = egui::Area::new(input.id.with("palette"))
        .order(egui::Order::Foreground)
        .pivot(egui::Align2::CENTER_TOP)
        .fixed_pos(top)
        .constrain(true)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(420.0);
                let field = egui::TextEdit::singleline(&mut palette.query)
                    .id(field_id(input.id))
                    .hint_text(input.labels.palette_hint)
                    .desired_width(f32::INFINITY);
                let field = ui.add(field);
                if std::mem::take(&mut palette.focus_field) {
                    field.request_focus();
                }
                if field.changed() {
                    palette.selected = 0;
                }
                for (i, command) in commands.iter().enumerate() {
                    ui.horizontal(|ui| {
                        // VS Code's `⚙`: change the keys of the command.
                        let gear = if command.configurable { CONFIGURE_WIDTH } else { 0.0 };
                        let button =
                            egui::Button::selectable(i == palette.selected, command.label).right_text(command.shortcut);
                        if ui.add_sized([ui.available_width() - gear, 0.0], button).clicked() {
                            clicked = Some(command.picked);
                        }
                        let Picked::Host(id) = command.picked else { return };
                        if command.configurable
                            && ui
                                .add(egui::Button::new("⚙").small().frame(false))
                                .on_hover_text(input.labels.configure_keybinding)
                                .clicked()
                        {
                            clicked = Some(Picked::Configure(id));
                        }
                    });
                }
            });
        });
    palette.rect = Some(area.response.rect);
    clicked
}

/// The room of a row's `⚙`.
const CONFIGURE_WIDTH: f32 = 22.0;

fn field_id(editor: egui::Id) -> egui::Id {
    editor.with("palette_field")
}
