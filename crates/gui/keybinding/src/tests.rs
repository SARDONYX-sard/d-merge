use super::*;

/// A key press as egui reports it: the key, then the char it types (if it types one).
fn press(keys: &str) -> Vec<egui::Event> {
    parse_keys(keys).map(|keys| give_back(&keys)).unwrap()
}

/// Runs a frame of `events` through `keys`. Returns the commands and the events left for the widgets.
fn frame(
    ctx: &egui::Context,
    keys: &mut Keybindings,
    context: &KeyContext,
    events: Vec<egui::Event>,
    time: f64,
) -> (Vec<&'static str>, Vec<egui::Event>) {
    let raw = egui::RawInput { events, time: Some(time), ..Default::default() };
    let (mut commands, mut left) = (Vec::new(), Vec::new());
    let mut output = ctx.run_ui(raw, |ui| {
        commands = keys.dispatch(ui.ctx(), context);
        left = ui.input(|input| input.events.clone());
    });
    output.textures_delta.clear();
    (commands, left)
}

fn entry(key: &str, command: &str, when: Option<&str>) -> file::Entry {
    file::Entry { key: key.to_owned(), command: command.to_owned(), when: when.map(str::to_owned), args: None }
}

const EDITOR: KeyContext = KeyContext {
    page: "patchEditor",
    editor_focus: true,
    vim: true,
    vim_mode: Some("normal"),
    explorer_focus: false,
    explorer_view: None,
    problems_focus: false,
    problems_visible: false,
    input_focus: false,
    user_patch: false,
};

#[test]
fn a_default_key_is_taken_as_its_command() {
    let (ctx, mut keys) = (egui::Context::default(), Keybindings::default());
    let (commands, left) = frame(&ctx, &mut keys, &EDITOR, press("ctrl+shift+l"), 0.0);
    assert_eq!(commands, [commands::PROBLEMS_TOGGLE]);
    assert!(left.is_empty(), "the widgets don't see it: {left:?}");

    let (commands, left) = frame(&ctx, &mut keys, &EDITOR, press("j"), 0.1);
    assert!(commands.is_empty());
    assert_eq!(left, press("j"), "no binding: the key stays, with its text");
}

#[test]
fn when_decides_and_the_last_binding_wins() {
    let ctx = egui::Context::default();
    // `t` is the explorer's: not the editor's.
    let mut keys = Keybindings::default();
    assert!(frame(&ctx, &mut keys, &EDITOR, press("t"), 0.0).0.is_empty());
    let explorer = KeyContext { editor_focus: false, explorer_focus: true, ..EDITOR };
    assert_eq!(frame(&ctx, &mut keys, &explorer, press("t"), 0.1).0, [commands::EXPLORER_TOGGLE_VIEW]);

    let normal = Some("editorFocus && vim.mode == 'normal'");
    let mut keys =
        Keybindings::with_entries(&[entry("shift+l", "tabs.next", normal), entry("alt+l", "tabs.close", None)]);
    assert!(keys.take_warnings().is_empty());
    assert_eq!(frame(&ctx, &mut keys, &EDITOR, press("shift+l"), 0.2).0, [commands::TABS_NEXT]);
    let insert = KeyContext { vim_mode: Some("insert"), ..EDITOR };
    assert!(frame(&ctx, &mut keys, &insert, press("shift+l"), 0.3).0.is_empty(), "types `L` in Insert mode");
    assert_eq!(frame(&ctx, &mut keys, &EDITOR, press("alt+l"), 0.4).0, [commands::TABS_CLOSE], "the user's");
}

#[test]
fn a_minus_entry_takes_a_default_away() {
    let ctx = egui::Context::default();
    let mut keys = Keybindings::with_entries(&[entry("ctrl+shift+l", "-problems.toggle", None)]);
    assert!(keys.take_warnings().is_empty());
    let (commands, left) = frame(&ctx, &mut keys, &EDITOR, press("ctrl+shift+l"), 0.0);
    assert!(commands.is_empty() && !left.is_empty());
    // Only that key: `tabs.next` keeps `ctrl+pagedown` when `alt+l` goes.
    let mut keys = Keybindings::with_entries(&[entry("alt+l", "-tabs.next", None)]);
    assert_eq!(frame(&ctx, &mut keys, &EDITOR, press("ctrl+pagedown"), 0.1).0, [commands::TABS_NEXT]);
    assert!(frame(&ctx, &mut keys, &EDITOR, press("alt+l"), 0.2).0.is_empty());
}

#[test]
fn a_sequence_waits_and_gives_back_what_starts_nothing() {
    let ctx = egui::Context::default();
    let mut keys = Keybindings::with_entries(&[entry("space b", "history.back", None)]);
    let (commands, left) = frame(&ctx, &mut keys, &EDITOR, press("space"), 0.0);
    assert!(commands.is_empty() && left.is_empty(), "held for the rest");
    assert_eq!(frame(&ctx, &mut keys, &EDITOR, press("b"), 0.1).0, [commands::HISTORY_BACK]);

    // `space z` starts nothing: both keys go on, as they came.
    frame(&ctx, &mut keys, &EDITOR, press("space"), 0.2);
    let (commands, left) = frame(&ctx, &mut keys, &EDITOR, press("z"), 0.3);
    assert!(commands.is_empty());
    assert_eq!(left, [press("space"), press("z")].concat());

    // However long the rest takes, as VS Code's chords; meanwhile what may follow is known.
    frame(&ctx, &mut keys, &EDITOR, press("space"), 1.0);
    assert_eq!(keys.pending(), parse_keys("space").unwrap());
    let next = keys.continuations(&EDITOR);
    assert!(next.iter().any(|(rest, command)| rest == "b" && command.id == commands::HISTORY_BACK), "{next:?}");
    assert!(next.iter().any(|(rest, command)| rest == "e" && command.id == commands::EXPLORER_FOCUS));
    assert_eq!(frame(&ctx, &mut keys, &EDITOR, press("b"), 60.0).0, [commands::HISTORY_BACK]);
}

#[test]
fn what_of_the_file_cannot_be_taken_is_told() {
    let mut keys = Keybindings::with_entries(&[
        entry("ctrl+q", "tabs.closee", None),
        entry("hyper+q", "tabs.close", None),
        entry("q", "tabs.close", Some("a &&")),
        entry("q", "-tabs.close", None),
    ]);
    let warnings = keys.take_warnings();
    assert_eq!(warnings.len(), 4, "{warnings:#?}");
    assert!(warnings[0].contains("tabs.closee"));
    assert!(keys.take_warnings().is_empty(), "told once");
}

#[test]
fn the_keys_of_a_command_are_shown_as_written() {
    let keys = Keybindings::default();
    assert_eq!(keys.keys_of(commands::PROBLEMS_TOGGLE, &EDITOR).as_deref(), Some("ctrl+shift+l"));
    // The last binding of it: `alt+l` comes after `ctrl+pagedown`.
    assert_eq!(keys.keys_of(commands::TABS_NEXT, &EDITOR).as_deref(), Some("alt+l"));
}

#[test]
fn the_user_file_is_made_and_read_again_when_it_changes() {
    let dir = std::env::temp_dir().join(format!("d_merge_keys_{}", std::process::id()));
    let path = dir.join("keybindings.json");
    let mut keys = Keybindings::load(&path);
    assert!(path.exists() && dir.join(file::SCHEMA_FILE).exists());
    assert!(keys.take_warnings().is_empty(), "the new file is fine");

    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&path, r#"[{ "key": "f9", "command": "tabs.close" }]"#).unwrap();
    // Some file systems keep the time to a second or more: make it differ.
    let later = SystemTime::now() + std::time::Duration::from_secs(5);
    let _ = std::fs::File::options().write(true).open(&path).and_then(|file| file.set_modified(later));
    assert!(keys.reload_if_changed(10.0));
    assert!(!keys.reload_if_changed(10.5), "once a second");
    let ctx = egui::Context::default();
    assert_eq!(frame(&ctx, &mut keys, &EDITOR, press("f9"), 11.0).0, [commands::TABS_CLOSE]);
    std::fs::remove_dir_all(&dir).unwrap();
}
