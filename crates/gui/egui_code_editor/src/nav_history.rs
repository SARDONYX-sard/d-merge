//! Where the editor has been, as VS Code's Go Back / Go Forward (`Alt+←` / `Alt+→`, Vim's `Ctrl+O` /
//! `Ctrl+I`):
//!
//! ```text
//! let mut history = NavHistory::<String>::default();
//! history.visit("#a");                          // leaving #a for a definition in #b
//! history.walk(true, "#b")    -> Some("#a")     // back
//! history.walk(false, "#a")   -> Some("#b")     // forward again
//! ```
//! What a place is (a document, a document and a line) is the host's: `T`.

use std::borrow::Borrow;

/// See the module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavHistory<T> {
    back: Vec<T>,
    forward: Vec<T>,
}

impl<T> Default for NavHistory<T> {
    #[inline]
    fn default() -> Self {
        Self { back: Vec::new(), forward: Vec::new() }
    }
}

impl<T> NavHistory<T> {
    /// Leaving `from` for another place: it is where back goes. Going forward is forgotten, as in a
    /// browser. Leaving the same place twice keeps it once.
    pub fn visit<Q>(&mut self, from: &Q)
    where
        Q: ?Sized + PartialEq + ToOwned<Owned = T>,
        T: Borrow<Q>,
    {
        if self.back.last().map(Borrow::borrow) != Some(from) {
            self.back.push(from.to_owned());
        }
        self.forward.clear();
    }

    /// Is there a place to go back (`older`) / forward to? e.g. to enable `←` / `→`.
    #[inline]
    #[must_use]
    pub const fn can_walk(&self, older: bool) -> bool {
        !(if older { &self.back } else { &self.forward }).is_empty()
    }

    /// The place to go back (`older`) / forward to, from `current`, which the other way then goes to.
    pub fn walk<Q>(&mut self, older: bool, current: &Q) -> Option<T>
    where
        Q: ?Sized + ToOwned<Owned = T>,
    {
        let (from, to) = if older { (&mut self.back, &mut self.forward) } else { (&mut self.forward, &mut self.back) };
        let target = from.pop()?;
        to.push(current.to_owned());
        Some(target)
    }
}

impl<T> NavHistory<T> {
    /// `←` `→`, enabled when there is somewhere to go, their hovers with the keys (`Go Back (Alt+←)`):
    /// for those who don't know the keys. `(back, forward)`: their names in the host's language, e.g.
    /// `("Go Back", "Go Forward")`. Returns `Some(true)` to go back, `Some(false)` forward.
    pub fn buttons(&self, ui: &mut egui::Ui, (back, forward): (&str, &str)) -> Option<bool> {
        use egui_widgets::shadcn::{Button, Icon};
        let buttons = [(true, Icon::ArrowLeft, back, "Alt+←"), (false, Icon::ArrowRight, forward, "Alt+→")];
        let mut walk = None;
        for (older, icon, label, keys) in buttons {
            let response = ui.add_enabled(self.can_walk(older), Button::new("").icon(icon).small());
            if response.on_hover_text(format!("{label} ({keys})")).clicked() {
                walk = Some(older);
            }
        }
        walk
    }

    /// The mouse's back button: `Some(true)`; its forward button: `Some(false)`. As in VS Code, in
    /// every keymap: going to a definition is only useful if you can come back.
    #[must_use]
    pub fn mouse(ui: &egui::Ui) -> Option<bool> {
        ui.input(|input| {
            if input.pointer.button_pressed(egui::PointerButton::Extra1) {
                Some(true)
            } else if input.pointer.button_pressed(egui::PointerButton::Extra2) {
                Some(false)
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_goes_back_and_forward() {
        let mut history = NavHistory::<String>::default();
        assert!(!history.can_walk(true) && !history.can_walk(false), "nowhere to go at the start");
        history.visit("#a"); // a -> b
        assert!(history.can_walk(true) && !history.can_walk(false));
        history.visit("#b"); // b -> c
        assert_eq!(history.walk(true, "#c").as_deref(), Some("#b"));
        assert_eq!(history.walk(true, "#b").as_deref(), Some("#a"));
        assert_eq!(history.walk(true, "#a"), None);
        assert_eq!(history.walk(false, "#a").as_deref(), Some("#b"));

        history.visit("#b"); // a new jump drops the forward list
        assert_eq!(history.walk(false, "#z"), None);
    }
}
