//! Mod list central panel: table rendering, sorting, and filtering.
//!
//! # Layout
//! The entire remaining window area below the top panels is a single
//! [`egui::CentralPanel`] containing a [`egui_extras::TableBuilder`] with
//! six columns:
//!
//! | # | content       | default width |
//! |---|---------------|---------------|
//! | 1 | checkbox      | auto          |
//! | 2 | id            | 20 %          |
//! | 3 | name          | 30 %          |
//! | 4 | mod type      |  7 %          |
//! | 5 | site          | 30 %          |
//! | 6 | priority      |  3 %          |
//!
//! # DnD vs read-only mode
//! Drag-and-drop reordering is only active when all three conditions hold:
//! - no filter text is entered,
//! - sort column is [`SortColumn::Priority`],
//! - sort direction is ascending.
//!
//! When DnD is inactive the table falls back to a checkbox-only view via
//! [`crate::ui::dnd_table::check_only_table_body`], and [`App::is_locked`]
//! is set so the lock button appears in the search panel.
//!
//! # Column auto-resize
//! [`egui_extras::Column`] does not support simultaneous `initial` width  when the window is resized.  [`App::resizable_column`]
//! works around this by briefly switching to `exact` width for the single
//! frame on which the available width changes, then returning to `initial`.

use d_merge_gui_shared::{
    fetch::FetchState,
    i18n::I18nKey,
    mod_item::{self, ModItem},
    settings::mod_list_ui::SortColumn,
};
use egui::Button;
use rayon::prelude::*;

use crate::{
    app::App,
    ui::dnd_table::{check_only_table_body, dnd_table_body},
};

impl App {
    /// Renders the central panel containing the mod list table.
    pub(crate) fn ui_mod_list(&mut self, ctx: &egui::Context) {
        let mut panel = egui::CentralPanel::default();
        if self.settings.ui.transparent {
            panel = panel.frame(egui::Frame::new());
        }

        panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(self.t(I18nKey::ModsListTitle));

                ui.add_space(50.0);
                self.ui_search_panel(ui);
                ui.separator();

                if self.is_locked {
                    let button = Button::new(self.i18n.t(I18nKey::LockButton));
                    let hover_text = self.i18n.t(I18nKey::LockButtonHover);

                    if ui.add_sized([60.0, 40.0], button).on_hover_text(hover_text).clicked() {
                        self.unlock_readonly_table();
                    }
                }

                if ui
                    .add_sized([60.0, 40.0], Button::new(self.t(I18nKey::NormalizeButton)))
                    .on_hover_text(self.t(I18nKey::NormalizeHover))
                    .clicked()
                {
                    mod_item::reorder_mods_priorities(self.settings.mod_list_mut());
                }

                let is_fetching = matches!(*self.fetch_state.read(), FetchState::Fetching);
                if ui
                    .add_enabled_ui(!is_fetching, |ui| {
                        ui.add_sized(
                            [60.0, 40.0],
                            Button::new(format!("🔄 {}", self.t(I18nKey::ReloadButton))),
                        )
                    })
                    .inner
                    .clicked()
                {
                    self.update_mod_list();
                }

                ui.add_visible(is_fetching, egui::Spinner::new());
                ui.colored_label(self.mod_list_msg.1, self.mod_list_msg.0.clone());
            });

            ui.separator();

            let mut filtered = self.filtered_mod_ids();
            self.sort_filtered_mods(&mut filtered);

            let dnd_allowed = self.is_dnd_allowed();
            self.is_locked = !dnd_allowed;

            self.render_table(ui, &filtered, dnd_allowed);
        });
    }

    /// Renders the search bar: filter text field, column selector, and lock button.
    pub(crate) fn ui_search_panel(&mut self, ui: &mut egui::Ui) {
        ui.label(self.t(I18nKey::SearchLabel));

        let text_line = egui::TextEdit::singleline(&mut self.settings.ui.mod_list.filter_text);
        let text_line = if self.settings.ui.transparent {
            text_line.background_color(egui::Color32::TRANSPARENT)
        } else {
            text_line
        };
        ui.add_sized([300.0, 40.0], text_line);

        if ui.add_sized([60.0, 40.0], Button::new(self.t(I18nKey::ClearButton))).clicked() {
            self.settings.ui.mod_list.filter_text.clear();
        }

        let all_label = self.i18n.t(I18nKey::FilterTextAll);
        let id_label = self.i18n.t(I18nKey::ColumnId);
        let name_label = self.i18n.t(I18nKey::ColumnName);
        let mod_type_label = self.i18n.t(I18nKey::ColumnModType);
        let site_label = self.i18n.t(I18nKey::ColumnSite);
        let priority_label = self.i18n.t(I18nKey::ColumnPriority);

        let selected_text = match self.settings.ui.mod_list.filter_column {
            None => all_label,
            Some(SortColumn::Id) => id_label,
            Some(SortColumn::Name) => name_label,
            Some(SortColumn::ModType) => mod_type_label,
            Some(SortColumn::Site) => site_label,
            Some(SortColumn::Priority) => priority_label,
        };

        let items = [
            (None, all_label),
            (Some(SortColumn::Id), id_label),
            (Some(SortColumn::Name), name_label),
            (Some(SortColumn::ModType), mod_type_label),
            (Some(SortColumn::Site), site_label),
            (Some(SortColumn::Priority), priority_label),
        ];

        egui::ComboBox::from_id_salt("filter_column").selected_text(selected_text).show_ui(
            ui,
            |ui| {
                for (value, label) in items {
                    ui.selectable_value(&mut self.settings.ui.mod_list.filter_column, value, label);
                }
            },
        );
    }

    /// Toggles or changes the active sort column.
    ///
    /// Clicking the same column reverses direction; clicking a different column resets to ascending..
    pub(crate) fn toggle_sort(&mut self, column: SortColumn) {
        if self.settings.ui.mod_list.sort_column == column {
            self.settings.ui.mod_list.sort_asc = !self.settings.ui.mod_list.sort_asc;
        } else {
            self.settings.ui.mod_list.sort_column = column;
            self.settings.ui.mod_list.sort_asc = true;
        }
    }

    /// Resets sort and filter state so the table becomes DnD-editable again.
    ///
    /// Called when the user clicks the lock button in the search panel.
    /// DnD reordering requires ascending-priority sort with no active filter
    /// (see [`App::is_dnd_allowed`]).
    fn unlock_readonly_table(&mut self) {
        self.settings.ui.mod_list.sort_asc = true;
        self.settings.ui.mod_list.sort_column = SortColumn::Priority;
        self.settings.ui.mod_list.filter_text.clear();
    }

    /// Returns the subset of mods that match the current filter text and column.
    ///
    /// When the filter is empty the full list is returned (cloned in parallel).
    /// Filtering uses `par_iter` so large lists do not block the frame.
    fn filtered_mod_ids(&self) -> Vec<ModItem> {
        if self.settings.ui.mod_list.filter_text.trim().is_empty() {
            return self.settings.mod_list().par_iter().cloned().collect();
        }

        let text = self.settings.ui.mod_list.filter_text.trim().to_lowercase();
        let matches_filter = |m: &&ModItem| match self.settings.ui.mod_list.filter_column {
            None => {
                m.id.to_lowercase().contains(&text)
                    || m.name.to_lowercase().contains(&text)
                    || m.site.to_lowercase().contains(&text)
            }
            Some(SortColumn::Id) => m.id.to_lowercase().contains(&text),
            Some(SortColumn::Name) => m.name.to_lowercase().contains(&text),
            Some(SortColumn::ModType) => m.mod_type.as_str().contains(&text),
            Some(SortColumn::Site) => m.site.to_lowercase().contains(&text),
            Some(SortColumn::Priority) => m.priority.to_string().contains(&text),
        };
        self.settings.mod_list().par_iter().filter(matches_filter).cloned().collect()
    }

    /// Sorts `mods` in-place according to the current sort column and direction.
    fn sort_filtered_mods(&self, mods: &mut [ModItem]) {
        mods.par_sort_unstable_by(|a, b| {
            let ord = match self.settings.ui.mod_list.sort_column {
                SortColumn::Id => a.id.cmp(&b.id),
                SortColumn::Name => a.name.cmp(&b.name),
                SortColumn::ModType => a.mod_type.cmp(&b.mod_type),
                SortColumn::Site => a.site.cmp(&b.site),
                SortColumn::Priority => a.priority.cmp(&b.priority),
            };
            if self.settings.ui.mod_list.sort_asc { ord } else { ord.reverse() }
        });
    }

    /// Returns `true` when drag-and-drop reordering is currently allowed.
    ///
    /// DnD mutates the underlying list by position, so it is only safe when
    /// the displayed order matches the stored priority order exactly.
    fn is_dnd_allowed(&self) -> bool {
        self.settings.ui.mod_list.filter_text.trim().is_empty()
            && self.settings.ui.mod_list.sort_column == SortColumn::Priority
            && self.settings.ui.mod_list.sort_asc
    }

    /// Renders the scroll area and [`egui_extras::TableBuilder`].
    fn render_table(&mut self, ui: &mut egui::Ui, filtered_mods: &[ModItem], editable: bool) {
        let table_max_height = ui.available_height() * 0.97;
        let total_width = ui.available_width();

        let changed_width = (self.prev_table_available_width - total_width).abs() > 0.5;
        if changed_width {
            self.prev_table_available_width = total_width;
        }

        egui::ScrollArea::vertical()
            .max_height(table_max_height)
            .max_width(total_width)
            .scroll_bar_rect(egui::Rect::everything_above(20.0))
            .show(ui, |ui| {
                egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .column(egui_extras::Column::auto().resizable(true))
                    .column(Self::resizable_column(total_width, 0.20, changed_width))
                    .column(Self::resizable_column(total_width, 0.30, changed_width))
                    .column(Self::resizable_column(total_width, 0.07, changed_width))
                    .column(Self::resizable_column(total_width, 0.30, changed_width))
                    .column(Self::resizable_column(total_width, 0.03, changed_width))
                    .header(20.0, |mut header| self.render_table_header(&mut header))
                    .body(|mut body| {
                        let mut widths = [0.0_f32; 6];
                        widths.clone_from_slice(body.widths());

                        let mod_list = if self.last_fetch_was_empty {
                            &mut vec![]
                        } else {
                            self.settings.mod_list_mut()
                        };

                        if editable {
                            dnd_table_body(body.ui_mut(), mod_list, widths);
                        } else {
                            check_only_table_body(&mut body, filtered_mods, mod_list, widths);
                        }
                    });
            });
    }

    /// Creates a resizable column that also resets to its ratio-based width
    /// whenever the table's total width changes.
    ///
    /// `egui_extras::Column` cannot simultaneously support `initial` (user
    /// resizable) and automatic width tracking.  This helper switches to
    /// `exact` for exactly one frame when `changed_width` is `true`, giving
    /// egui a concrete measurement to anchor the new layout.
    fn resizable_column(total_width: f32, ratio: f32, changed_width: bool) -> egui_extras::Column {
        let width = total_width * ratio;
        if changed_width {
            egui_extras::Column::exact(width)
        } else {
            egui_extras::Column::initial(width)
        }
        .resizable(true)
    }

    /// Renders the table header row with sortable column buttons.
    fn render_table_header(&mut self, header: &mut egui_extras::TableRow<'_, '_>) {
        let path_label = self.t(I18nKey::ColumnId).to_string();
        let name_label = self.t(I18nKey::ColumnName).to_string();
        let mod_type_label = self.t(I18nKey::ColumnModType).to_string();
        let site_label = self.t(I18nKey::ColumnSite).to_string();
        let priority_label = self.t(I18nKey::ColumnPriority).to_string();

        self.checkbox_header_button(header);
        self.header_button(header, &path_label, SortColumn::Id);
        self.header_button(header, &name_label, SortColumn::Name);
        self.header_button(header, &mod_type_label, SortColumn::ModType);
        self.header_button(header, &site_label, SortColumn::Site);
        self.header_button(header, &priority_label, SortColumn::Priority);
    }

    /// Renders the "check all / uncheck all" checkbox in the header.
    ///
    /// Only mods currently visible in the filtered view are affected.
    /// If the filter is empty, all mods are toggled.
    fn checkbox_header_button(&mut self, header: &mut egui_extras::TableRow<'_, '_>) {
        header.col(|ui| {
            if ui.add(egui::Checkbox::without_text(&mut self.check_all)).clicked() {
                let check_all = self.check_all;

                let filtered_ids: rapidhash::fast::RapidHashSet<_> =
                    self.filtered_mod_ids().into_par_iter().map(|m| m.id).collect();
                let is_unfiltered = filtered_ids.is_empty();

                self.settings.mod_list_mut().par_iter_mut().for_each(|item| {
                    if is_unfiltered || filtered_ids.contains(&item.id) {
                        item.enabled = check_all;
                    }
                });
            }
        });
    }

    /// Renders one sortable header button.
    ///
    /// The button label gets a `▲` / `▼` suffix when this column is the
    /// active sort column.
    fn header_button(
        &mut self,
        header: &mut egui_extras::TableRow<'_, '_>,
        label: &str,
        column: SortColumn,
    ) {
        header.col(|ui| {
            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| {
                    let text = if self.settings.ui.mod_list.sort_column == column {
                        if self.settings.ui.mod_list.sort_asc {
                            format!("{label} ▲")
                        } else {
                            format!("{label} ▼")
                        }
                    } else {
                        label.to_string()
                    };

                    if ui.button(text).clicked() {
                        self.toggle_sort(column);
                    }
                },
            );
        });
    }
}
