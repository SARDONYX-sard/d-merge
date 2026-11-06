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
    /// See [`Self::help`].
    field_hover: Option<&'a str>,
    hint: Option<&'a str>,
    label_width: f32,
    right_margin: f32,
    button_size: [f32; 2],
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
    /// Default of [`Self::label_width`].
    const LABEL_WIDTH: f32 = 140.0;
    /// Default of [`Self::right_margin`].
    const RIGHT_MARGIN: f32 = 8.0;

    /// Default of [`Self::button_size`]. A button in the `trailing` of [`Self::show_with`] should have
    /// the same size.
    pub const BUTTON_SIZE: [f32; 2] = [90.0, 32.0];

    /// `label`: the button on the left, `select`: the one that opens the dialog.
    #[inline]
    pub const fn new(label: &'a str, select: &'a str, actions: PathActions) -> Self {
        Self {
            label,
            select,
            actions,
            hover: None,
            field_hover: None,
            hint: None,
            label_width: Self::LABEL_WIDTH,
            right_margin: Self::RIGHT_MARGIN,
            button_size: Self::BUTTON_SIZE,
        }
    }

    /// The width of the label button. Rows that line up need the same one.
    #[inline]
    pub const fn label_width(mut self, label_width: f32) -> Self {
        self.label_width = label_width;
        self
    }

    /// The space to the right of the row.
    #[inline]
    pub const fn right_margin(mut self, right_margin: f32) -> Self {
        self.right_margin = right_margin;
        self
    }

    /// The size of the select button, whose height the label button has too.
    #[inline]
    pub const fn button_size(mut self, button_size: [f32; 2]) -> Self {
        self.button_size = button_size;
        self
    }

    /// The label button's hover text.
    #[inline]
    pub const fn hover(mut self, hover: &'a str) -> Self {
        self.hover = Some(hover);
        self
    }

    /// What to enter: the hover text of both the label button and the field, which is what one looks
    /// at when not knowing what to enter.
    #[inline]
    pub const fn help(mut self, help: &'a str) -> Self {
        self.hover = Some(help);
        self.field_hover = Some(help);
        self
    }

    /// The field's text while it is empty.
    #[inline]
    pub const fn hint(mut self, hint: Option<&'a str>) -> Self {
        self.hint = hint;
        self
    }

    /// Draws the row.
    #[inline]
    pub fn show(self, ui: &mut egui::Ui, path: &mut String) -> PathRowOutput {
        self.show_with(ui, path, |_| {})
    }

    /// Draws a row of a directory that is not chosen here: the path as text, then `trailing` (right to
    /// left) in place of the select button. Returns why the label button could not open it, if so.
    pub fn show_read_only(self, ui: &mut egui::Ui, path: &str, trailing: impl FnOnce(&mut egui::Ui)) -> Option<String> {
        let mut open_error = None;
        ui.horizontal(|ui| {
            let size = [self.label_width, self.button_size[1]];
            let mut open = ui.add_sized(size, Button::new(self.label));
            if let Some(hover) = self.hover {
                open = open.on_hover_text(hover);
            }
            if open.clicked() {
                open_error = (self.actions.open)(path).err();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(self.right_margin);
                trailing(ui);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add(egui::Label::new(egui::RichText::new(path).weak()).truncate());
                });
            });
        });
        open_error
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
            let size = [self.label_width, self.button_size[1]];
            let mut open = ui.add_sized(size, Button::new(self.label));
            if let Some(hover) = self.hover {
                open = open.on_hover_text(hover);
            }
            if open.clicked() {
                output.open_error = (self.actions.open)(path.as_str()).err();
            }
            // Right to left: the buttons take their width first, the field gets the rest.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(self.right_margin);
                let select = Button::new(self.select).icon(Icon::FolderSearch).full_width();
                if ui.add_sized(self.button_size, select).clicked()
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
                let mut field = ui.add(field);
                if let Some(help) = self.field_hover {
                    field = field.on_hover_text(help);
                }
                output.changed |= field.changed();
                output.submitted = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
        });
        output
    }
}
