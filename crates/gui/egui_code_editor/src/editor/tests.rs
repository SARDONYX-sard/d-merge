use egui::text::{CCursor, CCursorRange};

use super::{popups::apply_quick_fix, text::sorted_chars, *};
use crate::{Indent, QuickFix};

#[test]
fn quick_fix_replaces_columns_of_its_line() {
    let mut text = "first\nabc def".to_owned();
    let fix = QuickFix { title: String::new(), line: 1, columns: 4..7, new_text: "XY".to_owned() };
    // `first\n` is 6 chars, then `abc ` is 4.
    assert_eq!(apply_quick_fix(&mut text, &fix), 10);
    assert_eq!(text, "first\nabc XY");
}

/// Shows an editor for pointer events, one frame at a time.
struct Frames {
    ctx: egui::Context,
    text: String,
    state: CodeEditorState,
    id: egui::Id,
    rect: egui::Rect,
    keymap: Keymap,
    command: Option<EditorCommand>,
    language: &'static dyn Language,
}

/// A language that indents with tabs.
struct Tabbed;
impl Language for Tabbed {
    fn indent(&self) -> Indent {
        Indent::Tab(4)
    }
}

/// A language whose form is easy to tell.
struct Upper;
impl Language for Upper {
    fn format(&self, text: &str) -> Option<String> {
        Some(text.to_uppercase())
    }
}

impl Frames {
    fn new(text: &str) -> Self {
        let mut frames = Self {
            ctx: egui::Context::default(),
            text: text.to_owned(),
            state: CodeEditorState::default(),
            id: egui::Id::NULL,
            rect: egui::Rect::NOTHING,
            keymap: Keymap::Normal,
            command: None,
            language: &Upper,
        };
        frames.frame(Vec::new());
        frames.ctx.memory_mut(|memory| memory.request_focus(frames.id));
        frames.frame(Vec::new());
        frames
    }

    fn frame(&mut self, events: Vec<egui::Event>) {
        self.frame_output(events);
    }

    /// The text this frame put on the clipboard, if any.
    fn copied(&mut self, events: Vec<egui::Event>) -> Option<String> {
        self.frame_output(events).platform_output.commands.into_iter().find_map(|command| match command {
            egui::OutputCommand::CopyText(text) => Some(text),
            _ => None,
        })
    }

    fn frame_output(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
        let raw = egui::RawInput { events, ..Default::default() };
        let (text, state, id, rect) = (&mut self.text, &mut self.state, &mut self.id, &mut self.rect);
        let (keymap, command, language) = (self.keymap, &mut self.command, self.language);
        let mut output = self.ctx.run_ui(raw, |ui| {
            let shown = CodeEditor::new("editor", text).language(language).keymap(keymap).show(ui, state);
            *command = shown.command;
            let response = shown.response;
            *id = response.id;
            *rect = response.rect;
        });
        output.textures_delta.clear();
        output
    }

    fn select(&self, from: usize, to: usize) {
        let mut edit = egui::TextEdit::load_state(&self.ctx, self.id).unwrap_or_default();
        edit.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(from), CCursor::new(to))));
        edit.store(&self.ctx, self.id);
    }

    fn selection(&self) -> std::ops::Range<usize> {
        let edit = egui::TextEdit::load_state(&self.ctx, self.id).unwrap();
        sorted_chars(&edit.cursor.char_range().unwrap())
    }

    fn move_to(&mut self, pos: egui::Pos2) {
        self.frame(vec![egui::Event::PointerMoved(pos)]);
    }

    fn button(&mut self, button: egui::PointerButton, pos: egui::Pos2, pressed: bool) {
        let modifiers = egui::Modifiers::NONE;
        self.frame(vec![egui::Event::PointerButton { pos, button, pressed, modifiers }]);
    }

    /// A point on the 0-based `line`, a little in from the start of the text.
    fn on_line(&self, line: usize) -> egui::Pos2 {
        let font = egui::TextStyle::Monospace.resolve(&self.ctx.global_style());
        let row = self.ctx.fonts_mut(|fonts| fonts.row_height(&font));
        self.rect.left_top() + egui::vec2(20.0, row.mul_add(line as f32, row / 2.0) + 2.0)
    }
}

#[test]
fn the_right_button_on_the_selection_keeps_it() {
    let mut frames = Frames::new("abcdefghijkl\nsecond line");
    frames.select(0, 12);
    let on_it = frames.on_line(0);
    frames.move_to(on_it);

    frames.button(egui::PointerButton::Secondary, on_it, true);
    assert_eq!(frames.selection(), 0..12, "on the press");
    frames.frame(Vec::new());
    assert_eq!(frames.selection(), 0..12, "while it is down");
    frames.button(egui::PointerButton::Secondary, on_it, false);
    assert_eq!(frames.selection(), 0..12, "on the release");
    frames.frame(Vec::new());
    assert_eq!(frames.selection(), 0..12, "after it");
    assert_eq!(frames.state.held_selection, None);
}

#[test]
fn the_right_button_elsewhere_moves_the_cursor() {
    let mut frames = Frames::new("abcdefghijkl\nsecond line");
    frames.select(0, 12);
    let elsewhere = frames.on_line(1);
    frames.move_to(elsewhere);

    frames.button(egui::PointerButton::Secondary, elsewhere, true);
    let selection = frames.selection();
    assert!(selection.is_empty() && selection.start > 12, "{selection:?}");
}

/// A language with the same hover over all of its text.
#[cfg(feature = "vim")]
struct Hovered(&'static str);
#[cfg(feature = "vim")]
impl Language for Hovered {
    fn hover_at(&self, _text: &str, _line: usize, _column: usize) -> Option<String> {
        Some(self.0.to_owned())
    }
}

/// Presses a key the way a window system reports it: the key, then the text it typed.
#[cfg(feature = "vim")]
fn press(frames: &mut Frames, key: egui::Key, modifiers: egui::Modifiers, typed: &str) {
    let mut events = vec![egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }];
    if !typed.is_empty() {
        events.push(egui::Event::Text(typed.to_owned()));
    }
    frames.frame(events);
}

#[cfg(feature = "vim")]
fn press_shift_k(frames: &mut Frames) {
    press(frames, egui::Key::K, egui::Modifiers::SHIFT, "K");
}

#[cfg(feature = "vim")]
fn press_j(frames: &mut Frames) {
    press(frames, egui::Key::J, egui::Modifiers::NONE, "j");
}

/// `K` twice, in Vim.
#[cfg(feature = "vim")]
fn in_hover_of(doc: &'static str) -> Frames {
    // (A leak is fine in a test: a language has to live as long as the frames.)
    let language: &'static Hovered = Box::leak(Box::new(Hovered(doc)));
    let mut frames = Frames::new("first\nsecond");
    frames.language = language;
    frames.keymap = Keymap::Vim;
    frames.frame(Vec::new());
    press_shift_k(&mut frames);
    press_shift_k(&mut frames);
    frames
}

#[cfg(feature = "vim")]
fn open_hover(frames: &Frames) -> &crate::popups::hover::Open {
    frames.state.hover.open.as_ref().expect("the popup is open")
}

#[cfg(feature = "vim")]
#[test]
fn k_twice_with_nothing_to_do_in_the_hover_leaves_the_keyboard_in_the_editor() {
    let mut frames = in_hover_of("plain text");
    assert!(!open_hover(&frames).focused);

    press_j(&mut frames);
    assert!(frames.state.hover.open.is_none(), "the next key closes it");
}

#[cfg(feature = "vim")]
#[test]
fn j_and_k_scroll_a_long_hover() {
    let doc: &'static str = "a line\n\n".repeat(100).leak();
    let mut frames = in_hover_of(doc);
    assert!(open_hover(&frames).focused, "there is more text to see");
    assert_eq!(open_hover(&frames).scroll_offset, 0.0);

    press_j(&mut frames);
    press_j(&mut frames);
    frames.frame(Vec::new());
    let scrolled = open_hover(&frames).scroll_offset;
    assert!(scrolled > 0.0, "{scrolled}");

    press(&mut frames, egui::Key::K, egui::Modifiers::NONE, "k");
    frames.frame(Vec::new());
    assert!(open_hover(&frames).scroll_offset < scrolled);
    assert!(frames.state.hover.open.is_some(), "not closed by them");
    assert_eq!(frames.text, "first\nsecond", "nor do they reach the editor");
}

#[cfg(feature = "vim")]
#[test]
fn tab_goes_through_the_links_of_a_hover() {
    let mut frames = in_hover_of("[one](https://example.com/1) and [two](https://example.com/2)");
    assert!(open_hover(&frames).focused);
    assert_eq!(open_hover(&frames).link, None, "on the popup, until asked");

    press_tab(&mut frames);
    assert_eq!(open_hover(&frames).link, Some(0));
    press_tab(&mut frames);
    assert_eq!(open_hover(&frames).link, Some(1));
    press(&mut frames, egui::Key::Tab, egui::Modifiers::SHIFT, "");
    assert_eq!(open_hover(&frames).link, Some(0));
    assert!(!frames.ctx.memory(|memory| memory.has_focus(frames.id)), "the keyboard is not in the editor");
}

#[cfg(feature = "vim")]
#[test]
fn q_leaves_the_hover_for_the_editor() {
    let mut frames = in_hover_of("[one](https://example.com/1)");
    press(&mut frames, egui::Key::Q, egui::Modifiers::NONE, "q");
    assert!(frames.state.hover.open.is_none());
    assert!(frames.ctx.memory(|memory| memory.has_focus(frames.id)));
    let status = frames.state.keymap.status(Keymap::Vim).unwrap();
    assert!(!status.text.contains("recording"), "the `q` is not Vim's: {}", status.text);
}

fn press_tab(frames: &mut Frames) {
    frames.frame(vec![egui::Event::Key {
        key: egui::Key::Tab,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]);
}

#[cfg(feature = "vim")]
fn shift_right_in_vim(frames: &mut Frames) {
    frames.keymap = Keymap::Vim;
    frames.frame(vec![egui::Event::Text(">".to_owned())]);
    frames.frame(vec![egui::Event::Text(">".to_owned())]);
}

#[test]
fn indents_as_the_language_does() {
    let mut frames = Frames::new("a");
    press_tab(&mut frames);
    assert_eq!(frames.text, "a    ", "at the cursor, which is at the end");
    assert_eq!(frames.state.indent.to_string(), "Spaces: 4");

    let mut frames = Frames::new("a");
    frames.language = &Tabbed;
    frames.frame(Vec::new());
    press_tab(&mut frames);
    assert_eq!(frames.text, "a\t");
    assert_eq!(frames.state.indent.to_string(), "Tab: 4");
}

#[cfg(feature = "vim")]
#[test]
fn vim_shifts_as_the_language_does() {
    let mut frames = Frames::new("a");
    shift_right_in_vim(&mut frames);
    assert_eq!(frames.text, "    a");

    let mut frames = Frames::new("a");
    frames.language = &Tabbed;
    frames.frame(Vec::new());
    shift_right_in_vim(&mut frames);
    assert_eq!(frames.text, "\ta");
}

#[test]
fn a_command_of_the_palette_is_the_command() {
    for command in EditorCommand::ALL {
        assert_eq!(EditorCommand::from_action_id(command.action("label").id), Some(command));
    }
    assert_eq!(EditorCommand::from_action_id("of_the_host"), None);
}

#[cfg(feature = "vim")]
#[test]
fn a_key_that_waits_starts_the_list_of_keys() {
    let mut frames = Frames::new("first");
    frames.keymap = Keymap::Vim;
    frames.frame(vec![egui::Event::Text(" ".to_owned())]);
    assert!(matches!(frames.state.which_key_waiting, Some((crate::keymap::Prefix::Leader, _))));

    frames.frame(vec![egui::Event::Text("z".to_owned())]);
    assert_eq!(frames.state.which_key_waiting, None, "not a command: the keys are dropped");

    frames.frame(vec![egui::Event::Text("g".to_owned())]);
    assert!(matches!(frames.state.which_key_waiting, Some((crate::keymap::Prefix::G, _))));
}

#[cfg(feature = "vim")]
#[test]
fn vim_asks_the_host_for_the_next_change() {
    let mut frames = Frames::new("first");
    frames.keymap = Keymap::Vim;
    frames.frame(vec![egui::Event::Text("]".to_owned())]);
    assert_eq!(frames.command, None, "waiting");
    frames.frame(vec![egui::Event::Text("c".to_owned())]);
    assert_eq!(frames.command, Some(EditorCommand::NextChange));
    assert_eq!(frames.text, "first");
}

#[cfg(feature = "vim")]
#[test]
fn vim_space_w_formats_then_saves() {
    let mut frames = Frames::new("first\nsecond");
    frames.keymap = Keymap::Vim;
    frames.frame(vec![egui::Event::Text(" ".to_owned())]);
    assert_eq!(frames.text, "first\nsecond", "a leader key, waiting");

    frames.frame(vec![egui::Event::Text("w".to_owned())]);
    assert_eq!(frames.text, "FIRST\nSECOND");
    assert_eq!(frames.command, Some(EditorCommand::Save), "the host saves what is formatted");
}

#[cfg(feature = "vim")]
#[test]
fn vim_space_f_only_formats() {
    let mut frames = Frames::new("first");
    frames.keymap = Keymap::Vim;
    frames.frame(vec![egui::Event::Text(" ".to_owned())]);
    frames.frame(vec![egui::Event::Text("f".to_owned())]);
    assert_eq!(frames.text, "FIRST");
    assert_eq!(frames.command, None);
}

#[test]
fn selecting_with_the_mouse_asks_for_no_hover() {
    let mut frames = Frames::new("abcdefghijkl");
    let start = frames.on_line(0);
    frames.move_to(start);
    frames.frame(Vec::new());
    assert!(frames.state.hover.resting.is_some(), "resting on a char waits for the hover");

    frames.button(egui::PointerButton::Primary, start, true);
    let dragged = start + egui::vec2(30.0, 0.0);
    frames.move_to(dragged);
    frames.frame(Vec::new());
    assert_eq!(frames.state.hover.resting, None, "dragging");

    frames.button(egui::PointerButton::Primary, dragged, false);
    frames.frame(Vec::new());
    assert_eq!(frames.state.hover.resting, None, "where the drag ended");

    frames.move_to(dragged + egui::vec2(10.0, 0.0));
    assert!(frames.state.hover.resting.is_some(), "moved on");
}

#[test]
fn copy_puts_the_selection_on_the_clipboard() {
    for keymap in Keymap::ALL {
        let mut frames = Frames::new(
            "abcdefghijkl
second line",
        );
        frames.keymap = keymap;
        frames.frame(Vec::new());
        frames.select(0, 3);
        frames.frame(Vec::new());
        assert_eq!(frames.copied(vec![egui::Event::Copy]).as_deref(), Some("abc"), "{keymap:?}");
    }
}

/// Vim's Visual mode: `Ctrl+C` copies what `v` selected, and leaves the mode.
#[cfg(feature = "vim")]
#[test]
fn copy_in_vim_visual_mode() {
    let mut frames = Frames::new(
        "abcdefghijkl
second line",
    );
    frames.keymap = Keymap::Vim;
    frames.frame(Vec::new());
    frames.select(0, 0);
    for key in ["v", "l", "l"] {
        frames.frame(vec![egui::Event::Text(key.to_owned())]);
    }
    assert_eq!(frames.copied(vec![egui::Event::Copy]).as_deref(), Some("abc"));
    assert!(frames.selection().is_empty(), "back to Normal mode");
}

/// `Ctrl+Shift+P`: the palette opens with the keyboard in its field, as VS Code's.
#[test]
fn the_palette_takes_the_keyboard() {
    for keymap in Keymap::ALL {
        let mut frames = Frames::new("abc");
        frames.keymap = keymap;
        frames.frame(Vec::new());
        let modifiers = egui::Modifiers { ctrl: true, command: true, shift: true, ..Default::default() };
        let key =
            |pressed| egui::Event::Key { key: egui::Key::P, physical_key: None, pressed, repeat: false, modifiers };
        frames.frame(vec![key(true), key(false)]);
        frames.frame(Vec::new());
        assert!(frames.state.palette.open, "{keymap:?}");
        let field = frames.id.with("palette_field");
        assert_eq!(frames.ctx.memory(|m| m.focused()), Some(field), "{keymap:?}");
        frames.frame(vec![egui::Event::Text("fmt".to_owned())]);
        assert_eq!(frames.state.palette.query, ">fmt", "{keymap:?}: typed after the `>` it opens with");
    }
}

/// `Ctrl+/` comments the lines out and back in, in every keymap; Vim's `gcc` too.
#[cfg(feature = "vim")]
#[test]
fn comments_are_toggled() {
    let ctrl_slash = |pressed| egui::Event::Key {
        key: egui::Key::Slash,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers { ctrl: true, command: true, ..Default::default() },
    };
    for keymap in Keymap::ALL {
        let mut frames = Frames::new("\t<a/>\n\t<b/>");
        (frames.keymap, frames.language) = (keymap, &crate::languages::Xml);
        frames.frame(Vec::new());
        frames.select(1, 1);
        frames.frame(vec![ctrl_slash(true), ctrl_slash(false)]);
        assert_eq!(frames.text, "\t<!-- <a/> -->\n\t<b/>", "{keymap:?}");
        frames.frame(vec![ctrl_slash(true), ctrl_slash(false)]);
        assert_eq!(frames.text, "\t<a/>\n\t<b/>", "{keymap:?}: back");
    }

    let mut frames = Frames::new("\t<a/>\n\t<b/>");
    (frames.keymap, frames.language) = (Keymap::Vim, &crate::languages::Xml);
    frames.frame(Vec::new());
    frames.select(0, 0);
    for key in ["g", "c", "j"] {
        frames.frame(vec![egui::Event::Text(key.to_owned())]);
    }
    assert_eq!(frames.text, "\t<!-- <a/> -->\n\t<!-- <b/> -->", "gcj: this line and the next");
    for key in ["g", "c", "c"] {
        frames.frame(vec![egui::Event::Text(key.to_owned())]);
    }
    assert_eq!(frames.text, "\t<a/>\n\t<!-- <b/> -->", "gcc: the line");
}

/// The palette without `>` searches the text: `Enter` goes to the line found, as VS Code's.
#[test]
fn the_palette_finds_a_line() {
    let mut frames = Frames::new(
        "first
  second line
third",
    );
    frames.select(0, 0);
    frames.state.palette.reopen();
    frames.state.palette.query = "SECOND".to_owned();
    let enter = |pressed| egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    frames.frame(vec![enter(true), enter(false)]);
    frames.frame(Vec::new());
    assert!(!frames.state.palette.open);
    assert_eq!(frames.selection(), 8..8, "at `second` of line 2");
}

/// The peek's gap: the rows below the line move down by its height; its top is that line's bottom.
#[test]
fn a_gap_pushes_the_lines_below_down() {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        let font = egui::FontId::monospace(14.0);
        let galley = ui.painter().layout_no_wrap(
            "a
b
c"
            .to_owned(),
            font,
            egui::Color32::WHITE,
        );
        let gapped = text::with_gap(&galley, 1, 300.0);
        let tops: Vec<f32> = gapped.rows.iter().map(|row| row.pos.y).collect();
        assert_eq!(tops[..2], [galley.rows[0].pos.y, galley.rows[1].pos.y], "above: as they were");
        assert_eq!(tops[2], galley.rows[2].pos.y + 300.0);
        assert_eq!(gapped.rect.height(), galley.rect.height() + 300.0);
        assert_eq!(text::gap_top(&gapped, 1), galley.rows[1].rect().bottom());
    });
    output.textures_delta.clear();
}

/// The references' peek has the keys while it is open: `j` the next line, `Esc` closes it, and
/// none of them reach the editor.
#[test]
fn the_peek_has_the_keys() {
    fn frame(
        ctx: &egui::Context,
        (text, state): (&mut String, &mut CodeEditorState),
        list: &mut Option<ReferenceList>,
        events: Vec<egui::Event>,
    ) -> Option<PeekAsk> {
        let texts = |_: &str| {
            Some(
                "a
b
c"
                .to_owned(),
            )
        };
        let input = PeekInput { texts: &texts, language: &crate::languages::Xml };
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 900.0));
        let raw = egui::RawInput { events, screen_rect: Some(screen), ..Default::default() };
        let mut asked = None;
        let mut output = ctx.run_ui(raw, |ui| {
            asked = CodeEditor::new("editor", text).references(list, &input).show(ui, state).peek;
        });
        output.textures_delta.clear();
        asked
    }
    let key = |key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    let at = |line| Location { document: "#a".into(), line, detail: "".into() };

    let ctx = egui::Context::default();
    let (mut text, mut state) = (
        "a
b
c"
        .to_owned(),
        CodeEditorState::default(),
    );
    let mut list = Some(ReferenceList::from(References { symbol: "x".into(), locations: vec![at(0), at(2)] }));
    frame(&ctx, (&mut text, &mut state), &mut list, Vec::new());
    frame(&ctx, (&mut text, &mut state), &mut list, vec![key(egui::Key::J)]);
    assert_eq!(list.as_mut().and_then(|list| list.step(true)), Some(("#a", 0)), "`j` went to the second");

    let asked = frame(&ctx, (&mut text, &mut state), &mut list, vec![key(egui::Key::Escape)]);
    assert_eq!(asked, Some(PeekAsk::Close));
    assert!(list.is_none());
    assert_eq!(
        text,
        "a
b
c",
        "the keys did not reach the editor"
    );
}

/// The wheel over the references' peek scrolls only it, never the editor around it, even where it
/// has nothing more to scroll.
#[test]
fn the_wheel_over_the_peek_does_not_scroll_the_editor() {
    fn frame(
        ctx: &egui::Context,
        (text, state): (&mut String, &mut CodeEditorState),
        list: &mut Option<ReferenceList>,
        events: Vec<egui::Event>,
    ) -> f32 {
        let texts = |_: &str| Some("a".to_owned());
        let input = PeekInput { texts: &texts, language: &crate::languages::Xml };
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let raw = egui::RawInput { events, screen_rect: Some(screen), ..Default::default() };
        let mut top = 0.0;
        let mut output = ctx.run_ui(raw, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                top = CodeEditor::new("editor", text).references(list, &input).show(ui, state).response.rect.top();
            });
        });
        output.textures_delta.clear();
        top
    }
    let wheel = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -200.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
    };
    let at = Location { document: "#a".into(), line: 0, detail: "".into() };

    let ctx = egui::Context::default();
    let mut text = "line\n".repeat(200);
    let mut state = CodeEditorState::default();
    let mut list = Some(ReferenceList::from(References { symbol: "x".into(), locations: vec![at] }));
    let shown = (&mut text, &mut state);
    let before = frame(&ctx, shown, &mut list, Vec::new());
    for _ in 0..10 {
        // Inside the peek, under the first line.
        let events = vec![egui::Event::PointerMoved(egui::pos2(400.0, 150.0)), wheel.clone()];
        frame(&ctx, (&mut text, &mut state), &mut list, events);
    }
    let after = frame(&ctx, (&mut text, &mut state), &mut list, Vec::new());
    assert_eq!(after, before, "the editor stayed");
}
