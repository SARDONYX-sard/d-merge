//! The path to the cursor, above the editor, as VS Code's breadcrumbs: a click goes to that part.
//!
//! ```text
//! #0100 hkbStateMachine › states › hkobject › toStateId
//! ```
//!
//! What the parts are is the language's: e.g. [`crate::languages::xml_path`]. The host keeps them
//! while the cursor stays on its line (they are read from the text), and moves the cursor to the
//! one clicked.

/// One part of the path: what it shows, and where it starts in the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crumb {
    /// e.g. `#0100 hkbStateMachine`, `states`
    pub label: String,
    /// 0-based.
    pub line: usize,
    /// The char it starts at in its line. e.g. the `<` of a tag
    pub column: usize,
}

/// The row of [`Crumb`]s. See the module docs.
#[derive(Debug, Clone, Copy)]
pub struct Breadcrumbs<'a> {
    crumbs: &'a [Crumb],
}

impl<'a> Breadcrumbs<'a> {
    #[inline]
    pub const fn new(crumbs: &'a [Crumb]) -> Self {
        Self { crumbs }
    }

    /// Draws the row, cut to the width it has. Returns the part clicked.
    pub fn show(self, ui: &mut egui::Ui) -> Option<&'a Crumb> {
        let mut clicked = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (at, crumb) in self.crumbs.iter().enumerate() {
                if at > 0 {
                    ui.weak("›");
                }
                let text = egui::RichText::new(&crumb.label).small();
                let last = at + 1 == self.crumbs.len();
                let text = if last { text.strong() } else { text };
                if ui.add(egui::Button::new(text).frame(false).truncate()).clicked() {
                    clicked = Some(crumb);
                }
            }
        });
        clicked
    }
}
