//! Theme editor window and settings types.
//!
//! # Window layout
//!
//! ```text
//! ┌─ Theme Editor ──────────────────────────────────────────┐
//! │  Preset: [dark-ocean        ▼] [Load] [<>] [Delete]     │
//! │  Save as [_________________ ] [Save]                    │
//! │  <status message (yellow)>                              |
//! ├─────────────────────────────────────────────────────────┤
//! │  ▼ egui Visuals                                         │
//! │    dark mode [✓]                                        │
//! │    disabled alpha [────●────]                           │
//! │    window fill [■]                                      │
//! |    panel fill [■]                                       │
//! |    …                                                    │
//! ├─────────────────────────────────────────────────────────┤
//! │  ▼ Widgets                                              │
//! │    [Noninteractive][Inactive][Hovered][Active][Open]    │
//! │    bg_fill [■]                                          │
//! |    weak_bg_fill [■]                                     │
//! |    …                                                    │
//! ├─────────────────────────────────────────────────────────┤
//! │  ▼ shadcn Colors                                        │
//! │    background [■]  foreground [■]  …                    │
//! └─────────────────────────────────────────────────────────┘
//! ```
mod cache;
mod preset;

use d_merge_gui_shared::{fs::open_existing_dir_or_ancestor, settings::ui::theme::*};
use egui_shadcn::ShadcnThemeExt;

use crate::{
    theme::EguiColorExt as _,
    ui::{
        shadcn_compat::{button, checkbox, searchable_index_select, small_button, text},
        theme::cache::ThemeCache,
    },
};

// ─── Widget-state tab ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WidgetTab {
    Noninteractive,
    Inactive,
    Hovered,
    Active,
    Open,
}

impl WidgetTab {
    const ALL: &'static [Self] =
        &[Self::Noninteractive, Self::Inactive, Self::Hovered, Self::Active, Self::Open];

    const fn label(self) -> &'static str {
        match self {
            Self::Noninteractive => "Noninteractive",
            Self::Inactive => "Inactive",
            Self::Hovered => "Hovered",
            Self::Active => "Active",
            Self::Open => "Open",
        }
    }
}

// ─── ThemeManager ─────────────────────────────────────────────────────────────

/// Egui window for browsing, applying, editing, and saving theme presets.
///
/// Holds an `Arc<ThemeCache>` so it shares the same loaded-preset pool with
/// the rest of the application.  The caller is responsible for calling
/// [`show`](Self::show) every frame and applying the returned changes.
pub(crate) struct ThemeManager {
    /// Shared preset cache.
    pub(crate) cache: ThemeCache,

    /// Sorted preset names snapshotted from the cache at construction and
    /// after every `reload_dir`.  Stored locally so the combo-box can iterate
    /// them without re-acquiring the `RwLock` every frame.
    names: Vec<String>,

    /// Index into `names` for the combo-box.
    selected_index: usize,

    /// The preset currently shown in the editor.
    ///
    /// Populated by `Load`; mutated in real-time by the color pickers.
    /// `None` until the user (or the boot path) loads a preset.
    pub(crate) editing: Option<ThemePreset>,

    /// Buffer for the save-as name field.
    save_name: String,

    /// Selected tab in the Widgets section.
    widget_tab: WidgetTab,

    /// Non-fatal status message (shown for one or two seconds, then cleared).
    status: Option<StatusMsg>,
}

struct StatusMsg {
    text: String,
    is_error: bool,
}

impl ThemeManager {
    /// Create the window state.
    pub(crate) fn new(themes_dir: impl Into<std::path::PathBuf>, selected: Option<&str>) -> Self {
        let themes_dir = themes_dir.into();
        let cache = cache::ThemeCache::new(&themes_dir);
        let preset = selected.and_then(|name| cache.get(name).ok());

        let names = cache.names();
        let selected_index = names.iter().position(|n| Some(n.as_str()) == selected).unwrap_or(0);

        Self {
            cache,
            names,
            selected_index,
            editing: preset,
            save_name: selected.unwrap_or("Custom").to_string(),
            widget_tab: WidgetTab::Noninteractive,
            status: None,
        }
    }

    pub(crate) fn current_bg_color(&self) -> Option<&Rgba> {
        self.editing.as_ref().map(|preset| &preset.visuals.panel_fill)
    }

    /// Name that is currently selected in the combo-box.
    pub(crate) fn selected_name(&self) -> Option<&str> {
        self.names.get(self.selected_index).map(String::as_str)
    }

    fn load_by_name(&mut self, name: &str) {
        match self.cache.get(name) {
            Ok(preset) => {
                self.save_name = preset.name.clone();
                self.editing = Some(preset);
                self.status = None;
            }
            Err(e) => {
                self.set_error(format!("Load failed: {e}"));
            }
        }
    }

    fn load_selected(&mut self) {
        if let Some(name) = self.names.get(self.selected_index).cloned() {
            self.load_by_name(&name);
        }
    }

    fn save_current(&mut self) {
        let Some(editing) = &mut self.editing else {
            self.set_error("Nothing to save — load a preset first.".into());
            return;
        };

        editing.name = self.save_name.clone();
        let preset = editing.clone();

        match self.cache.save(preset) {
            Ok(_) => {
                // Refresh the local name snapshot in case a new entry was added.
                self.names = self.cache.names();
                self.selected_index =
                    self.names.iter().position(|n| n == &self.save_name).unwrap_or(0);
                self.set_ok(format!("Saved \"{}\".", self.save_name));
            }
            Err(e) => {
                self.set_error(format!("Save failed: {e}"));
            }
        }
    }

    fn reload_dir(&mut self) {
        let selected_name = self.selected_name().map(str::to_owned);

        self.cache.reload_dir();
        self.names = self.cache.names();

        // re select
        self.selected_index = selected_name
            .as_deref()
            .and_then(|name| self.names.iter().position(|n| n == name))
            .unwrap_or(0);

        self.set_ok("Directory reloaded.".into());
    }

    fn set_ok(&mut self, text: String) {
        self.status = Some(StatusMsg { text, is_error: false });
    }

    fn set_error(&mut self, text: String) {
        self.status = Some(StatusMsg { text, is_error: true });
    }

    // ── UI ───────────────────────────────────────────────────────────────

    /// Draw the theme editor window.
    ///
    /// Returns `Some(name)` when the caller should persist a new
    /// `selected_theme` in `settings.json` **and** re-apply the visuals.
    /// Returns `None` on frames with no actionable change.
    pub(crate) fn show(&mut self, ctx: &egui::Context) -> Option<SettingsUpdate> {
        let mut update: Option<SettingsUpdate> = None;

        egui::Window::new("Theme Editor")
            // .default_pos()
            .default_width(380.0)
            .min_width(300.0)
            .resizable(true)
            .show(ctx, |ui| {
                update = self.ui_contents(ui);
            });

        update
    }

    fn ui_contents(&mut self, ui: &mut egui::Ui) -> Option<SettingsUpdate> {
        let mut visuals_changed = false;
        let mut selection_changed = false;

        egui::Grid::new("theme_manager_grid").num_columns(2).spacing([8.0, 8.0]).show(ui, |ui| {
            // ── Preset row ────────────────────────────────────────────────────────
            if ui.add(button("Preset:")).on_hover_text("Open theme directory").clicked()
                && let Err(err) = open_existing_dir_or_ancestor(&self.cache.dir)
            {
                self.set_error(err);
            }
            ui.horizontal(|ui| {
                if searchable_index_select(
                    ui,
                    "theme_preset_combo",
                    &mut self.selected_index,
                    &self.names,
                    "Search...",
                )
                .changed()
                {
                    selection_changed = true;
                    self.load_selected();
                }

                if ui.add(button("Load")).clicked() {
                    selection_changed = true;
                    self.load_selected();
                }

                if ui.add(button("⟳")).on_hover_text("Reload themes directory").clicked() {
                    self.reload_dir();
                }
            });
            ui.end_row();

            // ── Save-as row ───────────────────────────────────────────────────────
            label("Save as:", ui);
            ui.horizontal(|ui| {
                text(ui, &mut self.save_name, None, 220.0);

                if ui.add(button("Save")).clicked() {
                    self.save_current();
                    selection_changed = true;
                }
            });
            ui.end_row();

            // ── Status bar ────────────────────────────────────────────────────────
            ui.label("Status: ");
            if let Some(msg) = &self.status {
                let color = if msg.is_error {
                    self.editing.as_ref().map_or(egui::Color32::from_rgb(220, 80, 80), |preset| {
                        preset.visuals.error_fg_color.to_egui_color32()
                    })
                } else {
                    egui::Color32::from_rgb(100, 200, 120)
                };
                ui.colored_label(color, &msg.text);
            }
            ui.end_row();
        });

        ui.separator();

        // ── Editor sections (only shown when a preset is loaded) ──────────────
        if let Some(editing) = &mut self.editing {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::CollapsingHeader::new("egui Visuals").default_open(true).show(ui, |ui| {
                    visuals_changed |= visuals_ui(ui, &mut editing.visuals);
                });

                ui.add_space(4.0);

                egui::CollapsingHeader::new("Widgets").default_open(false).show(ui, |ui| {
                    toggle_group_widget_tab(ui, &mut self.widget_tab);

                    ui.separator();

                    let wvc = match self.widget_tab {
                        WidgetTab::Noninteractive => &mut editing.visuals.widgets.noninteractive,
                        WidgetTab::Inactive => &mut editing.visuals.widgets.inactive,
                        WidgetTab::Hovered => &mut editing.visuals.widgets.hovered,
                        WidgetTab::Active => &mut editing.visuals.widgets.active,
                        WidgetTab::Open => &mut editing.visuals.widgets.open,
                    };
                    visuals_changed |= widget_visuals_ui(ui, wvc);
                });

                ui.add_space(4.0);

                egui::CollapsingHeader::new("shadcn Colors").default_open(false).show(ui, |ui| {
                    visuals_changed |= shadcn_ui(ui, &mut editing.shadcn);
                });
            });
        } else {
            label("Select a preset and press Load.", ui);
        }

        // ── Derive update signal ──────────────────────────────────────────────
        if visuals_changed || selection_changed {
            let selected_name = self.selected_name().unwrap_or("default").to_owned();

            Some(SettingsUpdate { selected_name, preset: self.editing.clone() })
        } else {
            None
        }
    }
}

/// Payload returned from [`ThemeManager::show`] when something changed.
pub(crate) struct SettingsUpdate {
    /// New value for `CustomTheme::selected_theme` (persist to settings.json).
    pub(crate) selected_name: String,

    /// Current preset snapshot, ready to apply to `egui::Context`.
    /// `None` when nothing has been loaded yet.
    pub(crate) preset: Option<ThemePreset>,
}

// ─── Section UIs ─────────────────────────────────────────────────────────────

fn visuals_ui(ui: &mut egui::Ui, v: &mut VisualsConfig) -> bool {
    let mut changed = false;

    egui::Grid::new("visuals_grid").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
        label("Dark mode", ui);
        changed |= checkbox(ui, &mut v.dark_mode, "").changed();
        ui.end_row();

        label("Disabled alpha", ui);
        changed |= slider_f32(ui, &mut v.disabled_alpha, 0.0..=1.0).changed();
        ui.end_row();

        label("Button frame", ui);
        changed |= checkbox(ui, &mut v.button_frame, "").changed();
        ui.end_row();

        label("Striped", ui);
        changed |= checkbox(ui, &mut v.striped, "").changed();
        ui.end_row();

        label("Slider trailing fill", ui);
        changed |= checkbox(ui, &mut v.slider_trailing_fill, "").changed();
        ui.end_row();

        label("Window corner radius", ui);
        changed |= slider_u8(ui, &mut v.window_corner_radius, 0..=24).changed();
        ui.end_row();

        label("Menu corner radius", ui);
        changed |= slider_u8(ui, &mut v.menu_corner_radius, 0..=24).changed();
        ui.end_row();

        for (label_, rgba) in [
            ("Window fill", &mut v.window_fill),
            ("Panel fill", &mut v.panel_fill),
            ("Faint bg", &mut v.faint_bg_color),
            ("Extreme bg", &mut v.extreme_bg_color),
            ("Code bg", &mut v.code_bg_color),
            ("Hyperlink", &mut v.hyperlink_color),
            ("Warn fg", &mut v.warn_fg_color),
            ("Error fg", &mut v.error_fg_color),
        ] {
            label(label_, ui);
            changed |= rgba_edit(ui, rgba);
            ui.end_row();
        }

        label("Override text", ui);
        if let Some(c) = v.override_text_color.as_mut() {
            let mut clear = false;

            ui.horizontal(|ui| {
                changed |= rgba_edit(ui, c);

                if ui.add(small_button("✕")).clicked() {
                    clear = true;
                }
            });

            if clear {
                v.override_text_color = None;
                changed = true;
            }
        } else if ui.add(small_button("+ Set")).clicked() {
            v.override_text_color = Some(Rgba::new(255, 255, 255, 255));
            changed = true;
        }
        ui.end_row();

        label("Text edit bg", ui);
        if let Some(c) = v.text_edit_bg_color.as_mut() {
            let mut clear = false;

            ui.horizontal(|ui| {
                changed |= rgba_edit(ui, c);

                if ui.add(small_button("✕")).clicked() {
                    clear = true;
                }
            });

            if clear {
                v.text_edit_bg_color = None;
                changed = true;
            }
        } else if ui.add(small_button("+ Set")).clicked() {
            v.text_edit_bg_color = Some(Rgba::new(255, 255, 255, 255));
            changed = true;
        }
        ui.end_row();

        label("Window stroke", ui);
        changed |= stroke_edit(ui, &mut v.window_stroke);
        ui.end_row();
    });

    changed
}

fn widget_visuals_ui(ui: &mut egui::Ui, w: &mut WidgetVisualsConfig) -> bool {
    let mut changed = false;

    egui::Grid::new("widget_grid").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
        label("bg fill", ui);
        changed |= rgba_edit(ui, &mut w.bg_fill);
        ui.end_row();

        label("weak bg fill", ui);
        changed |= rgba_edit(ui, &mut w.weak_bg_fill);
        ui.end_row();

        label("bg stroke", ui);
        changed |= stroke_edit(ui, &mut w.bg_stroke);
        ui.end_row();

        label("fg stroke", ui);
        changed |= stroke_edit(ui, &mut w.fg_stroke);
        ui.end_row();

        label("corner radius", ui);
        changed |= slider_u8(ui, &mut w.corner_radius, 0..=24).changed();
        ui.end_row();
    });

    changed
}

fn shadcn_ui(ui: &mut egui::Ui, s: &mut ShadcnThemeConfig) -> bool {
    let mut changed = false;

    egui::Grid::new("shadcn_grid").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
        for (label_, rgba) in [
            ("background", &mut s.background),
            ("foreground", &mut s.foreground),
            ("card", &mut s.card),
            ("card foreground", &mut s.card_foreground),
            ("popover", &mut s.popover),
            ("popover foreground", &mut s.popover_foreground),
            ("primary", &mut s.primary),
            ("primary foreground", &mut s.primary_foreground),
            ("secondary", &mut s.secondary),
            ("secondary foreground", &mut s.secondary_foreground),
            ("muted", &mut s.muted),
            ("muted foreground", &mut s.muted_foreground),
            ("accent", &mut s.accent),
            ("accent foreground", &mut s.accent_foreground),
            ("destructive", &mut s.destructive),
            ("destructive foreground", &mut s.destructive_foreground),
            ("border", &mut s.border),
            ("input", &mut s.input),
            ("ring", &mut s.ring),
        ] {
            label(label_, ui);
            changed |= rgba_edit(ui, rgba);
            ui.end_row();
        }

        label("radius", ui);
        changed |= slider_f32(ui, &mut s.radius, 0.0..=32.0).changed();
        ui.end_row();
    });

    changed
}

// ─── Primitive editors ────────────────────────────────────────────────────────

fn rgba_edit(ui: &mut egui::Ui, rgba: &mut Rgba) -> bool {
    let mut color = rgba.to_egui_color32();

    let changed = ui.color_edit_button_srgba(&mut color).changed();

    if changed {
        *rgba = Rgba::from_egui_color32(color);
    }

    changed
}

fn stroke_edit(ui: &mut egui::Ui, stroke: &mut StrokeConfig) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= slider_f32(ui, &mut stroke.width, 0.0..=4.0).changed();
        changed |= rgba_edit(ui, &mut stroke.color);
    });
    changed
}

// ─── Integration helpers ──────────────────────────────────────────────────────

/// Apply this preset to an egui `Context`.
pub(crate) fn apply(preset: &ThemePreset, ctx: &egui::Context) {
    ctx.set_visuals(preset::to_egui_visuals(&preset.visuals));
    ctx.set_style(egui::Style {
        visuals: preset::to_egui_visuals(&preset.visuals),
        ..Default::default()
    });

    ctx.set_shadcn_theme(preset::to_shadcn_theme(&preset.shadcn));
}

fn label(text: impl Into<String>, ui: &mut egui::Ui) -> egui::Response {
    egui_shadcn::Label::new(text).show(ui)
}

fn slider_f32(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> egui::Response {
    let before = *value;

    let mut response = ui
        .horizontal(|ui| {
            let response = ui.add(egui_shadcn::Slider::f32(value, range));
            ui.label(format!("{value:.2}"));

            response
        })
        .inner;

    // lint: Even if it can't be compared correctly, We'll just use it for the `changed` flag, so We'll ignore it.
    #[allow(clippy::float_cmp)]
    if *value != before {
        response.mark_changed(); // egui_shadcn: To avoid no changed bug
    }

    response
}

fn slider_u8(
    ui: &mut egui::Ui,
    value: &mut u8,
    range: std::ops::RangeInclusive<u8>,
) -> egui::Response {
    let before = *value;

    let mut tmp = *value as f32;

    let mut response = ui
        .horizontal(|ui| {
            let response = ui.add(
                egui_shadcn::Slider::f32(&mut tmp, *range.start() as f32..=*range.end() as f32)
                    .step(1.0),
            );
            ui.label(format!("{value}"));
            *value = tmp.round() as u8;

            response
        })
        .inner;

    // egui_shadcn: To avoid no changed bug
    if *value != before {
        response.mark_changed();
    }

    response
}

fn toggle_group_widget_tab(ui: &mut egui::Ui, value: &mut WidgetTab) {
    let Some(mut idx) = WidgetTab::ALL.iter().position(|tab| *tab == *value) else { return };

    egui_shadcn::ToggleGroup::new(
        WidgetTab::ALL.iter().map(|tab| tab.label().to_owned()).collect(),
    )
    .show(ui, &mut idx);

    *value = WidgetTab::ALL[idx];
}
