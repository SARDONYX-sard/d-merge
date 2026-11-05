//! The directory row of the pages.

use std::path::PathBuf;

use crate::shadcn::{Button, Icon, TextEdit};

/// What a [`PathRow`] does with the file system. Given by the app: this crate only knows egui.
///
/// e.g. `PathActions { open: gui_shared::fs::open_dir_of, pick: gui_shared::fs::pick_folder_from }`
#[derive(Debug, Clone, Copy)]
pub struct PathActions {
    /// Opens the directory `path` in the file manager. `Err`: why it could not.
    pub open: fn(path: &str) -> Result<(), String>,
    /// The folder dialog, starting from `path`. `None`: cancelled.
    pub pick: fn(path: &str) -> Option<PathBuf>,
}

/// A row of directories, so that all of them line up:
/// ```text
/// |<---label--->|<------ stretch ------>|<-trailing->|<-select->|
/// | [Skyrim]    | D:\GAME\...           | [Auto]     | [Select] |
/// | [Output]    | D:\out                |            | [Select] |
/// ```
/// The label is a button that opens the directory: the path, or else what exists of it (a path being
/// typed often does not exist yet).
#[derive(Debug, Clone, Copy)]
#[must_use = "Call `show` or `show_with` to draw it"]
pub struct PathRow<'a> {
    label: &'a str,
    select: &'a str,
    actions: PathActions,
    hover: Option<&'a str>,
    hint: Option<&'a str>,
}

/// What happened in a [`PathRow`] this frame.
#[derive(Debug, Default)]
#[must_use]
pub struct PathRowOutput {
    /// A directory was chosen in the dialog.
    pub picked: bool,
    /// `Enter` was pressed in the field.
    pub submitted: bool,
    /// The path changed, by typing or in the dialog.
    pub changed: bool,
    /// Why the label button could not open the directory.
    pub open_error: Option<String>,
}

impl<'a> PathRow<'a> {
    const LABEL_WIDTH: f32 = 140.0;
    const RIGHT_MARGIN: f32 = 8.0;

    /// The size of the buttons on the right. A button in the `trailing` of [`Self::show_with`] should
    /// have it too.
    pub const BUTTON_SIZE: [f32; 2] = [90.0, 32.0];

    /// `label`: the button on the left, `select`: the one that opens the dialog.
    pub const fn new(label: &'a str, select: &'a str, actions: PathActions) -> Self {
        Self { label, select, actions, hover: None, hint: None }
    }

    /// The label button's hover text.
    pub const fn hover(mut self, hover: &'a str) -> Self {
        self.hover = Some(hover);
        self
    }

    /// The field's text while it is empty.
    pub const fn hint(mut self, hint: Option<&'a str>) -> Self {
        self.hint = hint;
        self
    }

    /// Draws the row.
    pub fn show(self, ui: &mut egui::Ui, path: &mut String) -> PathRowOutput {
        self.show_with(ui, path, |_| {})
    }

    /// Draws the row, with `trailing` between the field and the select button (right to left).
    pub fn show_with(
        self,
        ui: &mut egui::Ui,
        path: &mut String,
        trailing: impl FnOnce(&mut egui::Ui),
    ) -> PathRowOutput {
        let mut output = PathRowOutput::default();
        ui.horizontal(|ui| {
            let size = [Self::LABEL_WIDTH, Self::BUTTON_SIZE[1]];
            let mut open = ui.add_sized(size, Button::new(self.label));
            if let Some(hover) = self.hover {
                open = open.on_hover_text(hover);
            }
            if open.clicked() {
                output.open_error = (self.actions.open)(path.as_str()).err();
            }
            // Right to left: the buttons take their width first, the field gets the rest.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(Self::RIGHT_MARGIN);
                let select = Button::new(self.select).icon(Icon::FolderSearch).full_width();
                if ui.add_sized(Self::BUTTON_SIZE, select).clicked()
                    && let Some(dir) = (self.actions.pick)(path.as_str())
                {
                    *path = dir.display().to_string();
                    output.picked = true;
                    output.changed = true;
                }
                trailing(ui);

                let mut field = TextEdit::singleline(path).desired_width(ui.available_width());
                if let Some(hint) = self.hint {
                    field = field.hint_text(hint);
                }
                let field = ui.add(field);
                output.changed |= field.changed();
                output.submitted = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
        });
        output
    }
}
