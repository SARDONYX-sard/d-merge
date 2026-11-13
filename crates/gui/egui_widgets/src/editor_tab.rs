//! A tab of an editor, as VS Code draws them: what is shown, and its state, at a glance.
//!
//! ```text
//!  ▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔                         <- accent: the shown tab, its editor has the keyboard
//! │ Root      ✖ │ GetUpModList ● │ Idle  │  <- the others: no background, weaker text
//!               ^ shown: editor background, strong text
//! ```
//!
//! | state                      | drawn as                                                  |
//! | -------------------------- | --------------------------------------------------------- |
//! | shown (`active`)           | the editor's background, strong text, a line on top       |
//! | its editor has the keyboard| that line in the accent color (else gray)                 |
//! | hovered                    | a faint background, and `✖`                               |
//! | `modified`                 | `●` where `✖` is, `✖` when hovered                         |
//! | `preview`                  | italic: the next tab opened replaces it                   |
//! | `tone`                     | the text in that color, e.g. red for errors in the file   |

/// One tab. See the module docs.
#[derive(Debug, Clone)]
#[must_use = "Call `show` to draw it"]
pub struct EditorTab {
    text: egui::WidgetText,
    active: bool,
    focused: bool,
    modified: bool,
    preview: bool,
    tone: Option<egui::Color32>,
    read_only: bool,
    hover: Option<egui::WidgetText>,
    height: f32,
    padding: f32,
    max_text_width: f32,
}

/// What happened to an [`EditorTab`] this frame.
#[derive(Debug)]
pub struct EditorTabOutput {
    /// Of the whole tab: clicks, the hover text, the context menu.
    pub response: egui::Response,
    /// Its `✖` was clicked, or it was middle clicked.
    pub close: bool,
    /// Its text was cut with `…` to [`EditorTab::max_text_width`]: the host may show the whole on hover.
    pub elided: bool,
    /// It was drawn as the preview tab: its menu has `Keep Open`.
    pub preview: bool,
}

/// What a tab asks of the host, as VS Code's: see [`EditorTabOutput::action`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabAction {
    /// A click: show it.
    Show,
    /// A double click, or the menu: it is no longer the preview tab.
    KeepOpen,
    /// `✖`, a middle click, or the menu.
    Close,
    CloseOthers,
    CloseToTheRight,
    CloseAll,
}

/// The words of a tab's right-click menu, in the host's language.
///
/// e.g. `{ keep_open: "Keep Open", close: "Close", close_others: "Close Others", close_to_the_right:
/// "Close to the Right", close_all: "Close All" }`
#[derive(Debug, Clone, Copy)]
pub struct TabMenu<'a> {
    pub keep_open: &'a str,
    pub close: &'a str,
    pub close_others: &'a str,
    pub close_to_the_right: &'a str,
    pub close_all: &'a str,
}

impl<'a> TabMenu<'a> {
    /// From `[keep_open, close, close_others, close_to_the_right, close_all]`, e.g. as the host's
    /// translations give them.
    #[inline]
    #[must_use]
    pub const fn from_texts([keep_open, close, close_others, close_to_the_right, close_all]: [&'a str; 5]) -> Self {
        Self { keep_open, close, close_others, close_to_the_right, close_all }
    }
}

impl EditorTabOutput {
    /// What the tab asked for this frame, as the host's event (`event`): a click shows it, a double
    /// click keeps it open, `✖` or a middle click closes it, and its right-click menu has `menu`
    /// (`Keep Open` only on the preview tab), then the host's own items (`more`, e.g. `Copy Path`).
    pub fn action<E>(
        &self,
        menu: &TabMenu<'_>,
        event: impl Fn(TabAction) -> E,
        more: impl FnOnce(&mut egui::Ui) -> Option<E>,
    ) -> Option<E> {
        let mut picked = if self.close {
            Some(TabAction::Close)
        } else if self.response.double_clicked() {
            Some(TabAction::KeepOpen)
        } else if self.response.clicked() {
            Some(TabAction::Show)
        } else {
            None
        }
        .map(&event);

        self.response.context_menu(|ui| {
            let items = [
                (menu.keep_open, self.preview, TabAction::KeepOpen),
                (menu.close, true, TabAction::Close),
                (menu.close_others, true, TabAction::CloseOthers),
                (menu.close_to_the_right, true, TabAction::CloseToTheRight),
                (menu.close_all, true, TabAction::CloseAll),
            ];
            for (label, enabled, action) in items {
                if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                    picked = Some(event(action));
                    ui.close();
                }
            }
            ui.separator();

            if let Some(own) = more(ui) {
                picked = Some(own);
            }
        });

        picked
    }
}

impl EditorTab {
    /// Default of [`Self::height`].
    const HEIGHT: f32 = 26.0;
    /// Default of [`Self::padding`].
    const PADDING: f32 = 10.0;
    /// Default of [`Self::max_text_width`].
    const MAX_TEXT_WIDTH: f32 = 240.0;
    /// The square of `✖` / `●`.
    const CLOSE_SIZE: f32 = 16.0;
    /// The line on top of the shown tab.
    const TOP_LINE: f32 = 2.0;

    /// `text`: e.g. `GetUpModList`. `active`: it is the tab shown.
    #[inline]
    pub fn new(text: impl Into<egui::WidgetText>, active: bool) -> Self {
        Self {
            text: text.into(),
            active,
            focused: false,
            modified: false,
            preview: false,
            tone: None,
            read_only: false,
            hover: None,
            height: Self::HEIGHT,
            padding: Self::PADDING,
            max_text_width: Self::MAX_TEXT_WIDTH,
        }
    }

    /// The editor of the shown tab has the keyboard: its top line takes the accent color.
    #[inline]
    pub const fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// It differs from what it was: `●` instead of `✖` until hovered.
    #[inline]
    pub const fn modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }

    /// The preview tab, which the next one opened replaces: italic.
    #[inline]
    pub const fn preview(mut self, preview: bool) -> Self {
        self.preview = preview;
        self
    }

    /// The text's color, e.g. of the worst problem in the file.
    #[inline]
    pub const fn tone(mut self, tone: Option<egui::Color32>) -> Self {
        self.tone = tone;
        self
    }

    /// What it shows can't be edited (a diff): a lock before `✖`, as VS Code's.
    #[inline]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// What its hover shows, e.g. the whole name or path.
    #[inline]
    pub fn hover(mut self, text: impl Into<egui::WidgetText>) -> Self {
        self.hover = Some(text.into());
        self
    }

    /// The widest its text gets, cut with `…` past it, so that `✖` stays in reach (default 240).
    #[inline]
    pub const fn max_text_width(mut self, width: f32) -> Self {
        self.max_text_width = width;
        self
    }

    /// Its height (default 26).
    #[inline]
    pub const fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Room left of the text and right of `✖` (default 10).
    #[inline]
    pub const fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> EditorTabOutput {
        let visuals = ui.visuals();
        let color = self
            .tone
            .unwrap_or_else(|| if self.active { visuals.strong_text_color() } else { visuals.weak_text_color() });
        let text = match self.text {
            egui::WidgetText::RichText(text) if self.preview => {
                egui::WidgetText::RichText(std::sync::Arc::new((*text).clone().italics()))
            }
            text => text,
        };
        let galley =
            text.into_galley(ui, Some(egui::TextWrapMode::Truncate), self.max_text_width, egui::TextStyle::Body);
        let elided = galley.elided;
        let lock = if self.read_only { Self::CLOSE_SIZE + 2.0 } else { 0.0 };
        let width = self.padding + galley.size().x + 6.0 + lock + Self::CLOSE_SIZE + self.padding;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, self.height), egui::Sense::click());
        // A hand over it, as over a link: it is clicked, not typed in.
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        let response = match self.hover {
            Some(hover) => response.on_hover_text(hover),
            None => response,
        };
        let preview = self.preview;
        let close_rect = egui::Rect::from_center_size(
            egui::pos2(rect.right() - self.padding - Self::CLOSE_SIZE / 2.0, rect.center().y),
            egui::Vec2::splat(Self::CLOSE_SIZE),
        );
        let close_response = ui.interact(close_rect, response.id.with("close"), egui::Sense::click());
        let close_response = close_response.on_hover_cursor(egui::CursorIcon::PointingHand);
        let close = close_response.clicked() || response.middle_clicked();
        if !ui.is_rect_visible(rect) {
            return EditorTabOutput { response, close, elided, preview };
        }

        let visuals = ui.visuals();
        let painter = ui.painter();
        let hovered = response.hovered() || close_response.hovered();
        if self.active {
            painter.rect_filled(rect, 0.0, visuals.extreme_bg_color);
            let line = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), Self::TOP_LINE));
            let accent = if self.focused { visuals.selection.stroke.color } else { visuals.weak_text_color() };
            painter.rect_filled(line, 0.0, accent);
        } else if hovered {
            painter.rect_filled(rect, 0.0, visuals.faint_bg_color);
        }
        // The line between tabs.
        let border = visuals.widgets.noninteractive.bg_stroke;
        painter.vline(rect.right() - border.width / 2.0, rect.y_range(), border);

        if self.read_only {
            let center = egui::pos2(close_rect.left() - 2.0 - Self::CLOSE_SIZE / 2.0, rect.center().y);
            let lock_rect = egui::Rect::from_center_size(center, egui::Vec2::splat(Self::CLOSE_SIZE * 0.75));
            crate::shadcn::icon::paint_icon(painter, lock_rect, crate::shadcn::Icon::Lock, visuals.weak_text_color());
        }
        let text_pos = egui::pos2(rect.left() + self.padding, rect.center().y - galley.size().y / 2.0);
        painter.galley(text_pos, galley, color);

        // `✖` on the shown or hovered tab, `●` on a modified one otherwise: as VS Code.
        let mark = if hovered || (self.active && !self.modified) {
            Some("✖")
        } else if self.modified {
            Some("●")
        } else {
            None
        };
        if let Some(mark) = mark {
            if close_response.hovered() {
                painter.rect_filled(close_rect, 3.0, visuals.widgets.hovered.weak_bg_fill);
            }
            let font = egui::FontId::proportional(Self::CLOSE_SIZE * 0.75);
            painter.text(close_rect.center(), egui::Align2::CENTER_CENTER, mark, font, color);
        }
        EditorTabOutput { response, close, elided, preview }
    }
}
