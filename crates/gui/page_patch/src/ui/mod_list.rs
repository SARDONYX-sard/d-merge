//! The mod table and its toolbar. What it shows is decided in [`crate::visible_mods`].

use std::rc::Rc;

use egui_widgets::shadcn::{Button, ComboBox, Icon, TextEdit, theme::ShadcnTheme};
use gui_shared::{
    i18n::I18nKey,
    settings::{
        DataMode,
        pages::patch::{mod_item, mod_list::SortColumn},
    },
};
use mod_info::ModType;

use super::{
    PatchView,
    dnd_table::{check_only_table_body, dnd_table_body},
};
use crate::visible_mods::{self, is_dnd_allowed};

impl PatchView<'_> {
    /// The central panel with the toolbar and the table.
    pub(super) fn ui_mod_list(&mut self, ui: &mut egui::Ui) {
        let rows = self.read_only_rows();
        egui::CentralPanel::default().frame(self.ctx.panel_frame).show(ui, |ui| {
            ui.add_space(10.0);
            self.render_toolbar(ui, rows.is_some());
            self.selected_label(ui, rows.as_ref().map_or(0, |rows| rows.len()));
            self.render_table(ui, rows.as_deref());
        });
    }

    /// The rows of the read-only view, or `None` while the table is the draggable list itself.
    fn read_only_rows(&mut self) -> Option<Rc<[u32]>> {
        let settings = &*self.ctx.settings;
        if is_dnd_allowed(&settings.view_state) {
            self.page.visible.invalidate(); // a drag may reorder the list
            return None;
        }
        Some(self.page.visible.rows(settings.mod_list(), settings.behavior.mode, &settings.view_state))
    }

    fn render_toolbar(&mut self, ui: &mut egui::Ui, locked: bool) {
        ui.horizontal(|ui| {
            let heading = egui::RichText::new(self.ctx.i18n.t(I18nKey::ModsListTitle))
                .color(ShadcnTheme::get(ui.ctx()).foreground)
                .size(24.0)
                .strong();
            ui.label(heading).on_hover_text(self.ctx.i18n.t(I18nKey::ModsListTitleHover));

            ui.separator();
            self.ui_search_bar(ui);

            ui.separator();

            // -- Normalize Button
            if ui
                .add_sized(
                    [90.0, 40.0],
                    Button::new(self.ctx.i18n.t(I18nKey::NormalizeButton)).icon(Icon::TextAlignJustify).full_width(),
                )
                .on_hover_text(self.ctx.i18n.t(I18nKey::NormalizeHover))
                .clicked()
            {
                mod_item::reorder_mods_priorities(self.ctx.settings.mod_list_mut());
                if matches!(self.ctx.settings.behavior.mode, DataMode::Manual) {
                    mod_item::dedup_mods_by_id(self.ctx.settings.mod_list_mut());
                }
                self.page.visible.invalidate();
            }

            // -- Lock Button
            if locked {
                let button =
                    Button::new(self.ctx.i18n.t(I18nKey::LockButton)).icon(Icon::Lock).full_width().destructive();
                let hover_text = self.ctx.i18n.t(I18nKey::LockButtonHover);

                if ui.add_sized([90.0, 40.0], button).on_hover_text(hover_text).clicked() {
                    visible_mods::unlock(&mut self.ctx.settings.view_state);
                }
            } else {
                ui.add_space(98.0);
            }

            ui.separator();
            if self.reload_button(ui) {
                ui.add(egui::Spinner::new());
            }
            ui.colored_label(self.page.list_message.color.resolve(ui.visuals()), &self.page.list_message.message);
        });
    }

    /// Shows `<selected> / <total> selected`, and `<hits> hits` while searching.
    fn selected_label(&self, ui: &mut egui::Ui, hits_count: usize) {
        let selected_count = self.ctx.settings.mod_list().iter().filter(|item| item.enabled).count();
        let total_count = self.ctx.settings.mod_list().len();
        let selected_label = self.ctx.i18n.t(I18nKey::SelectedLabel);

        ui.horizontal(|ui| {
            ui.label(format!("{selected_count} / {total_count} {selected_label}"));

            if !self.ctx.settings.view_state.filter_text.trim().is_empty() {
                ui.separator();
                let hits_label = self.ctx.i18n.t(I18nKey::SearchHitsLabel);
                ui.label(format!("{hits_count} {hits_label}"));
            }
        });
    }

    fn reload_button(&self, ui: &mut egui::Ui) -> bool {
        // -- Reload Button
        let is_fetching = self.page.fetch.is_fetching();
        if is_fetching {
            ui.add_enabled(false, Button::new("").icon(Icon::RefreshCwOff).large());
        } else {
            if ui
                .add(Button::new("").icon(Icon::RefreshCcw).large())
                .on_hover_text(format!("{} (Ctrl + R)", self.ctx.i18n.t(I18nKey::ReloadButton)))
                .clicked()
            {
                self.page.fetch_mods(self.ctx.settings);
            }
        }

        is_fetching
    }

    /// Renders the search bar: filter text field, column selector
    fn ui_search_bar(&mut self, ui: &mut egui::Ui) {
        ui.add(Icon::Search);
        let search_hint = format!("{}...", self.ctx.i18n.t(I18nKey::SearchLabel));
        ui.add(
            TextEdit::singleline(&mut self.ctx.settings.view_state.filter_text)
                .hint_text(search_hint)
                .desired_width(300.0),
        );

        if ui.add_sized([60.0, 40.0], Button::new(self.ctx.i18n.t(I18nKey::ClearButton))).clicked() {
            self.ctx.settings.view_state.filter_text.clear();
        }

        let items = [
            (None, self.ctx.i18n.t(I18nKey::FilterTextAll)),
            (Some(SortColumn::Id), self.ctx.i18n.t(I18nKey::ColumnId)),
            (Some(SortColumn::Name), self.ctx.i18n.t(I18nKey::ColumnName)),
            (Some(SortColumn::ModType), self.ctx.i18n.t(I18nKey::ColumnModType)),
            (Some(SortColumn::Site), self.ctx.i18n.t(I18nKey::ColumnSite)),
            (Some(SortColumn::Priority), self.ctx.i18n.t(I18nKey::ColumnPriority)),
        ];

        let filter_column = &mut self.ctx.settings.view_state.filter_column;
        let mut index = items.iter().position(|(column, _)| column == filter_column).unwrap_or(0);
        if ComboBox::from_id_salt("filter_column")
            .width(100.0)
            .show_index(ui, &mut index, items.len(), |i| items[i].1)
            .changed()
        {
            *filter_column = items[index].0;
        }
    }

    /// Renders the scroll area and [`egui_extras::TableBuilder`].
    fn render_table(&mut self, ui: &mut egui::Ui, rows: Option<&[u32]>) {
        let table_max_height = ui.available_height() * 0.97;
        // The real space of this panel (the window size setting is not the panel width).
        let total_width = ui.available_width();

        let changed_width = (self.page.prev_table_width - total_width).abs() > 0.5;
        if changed_width {
            self.page.prev_table_width = total_width;
        }

        egui::ScrollArea::vertical()
            .max_height(table_max_height)
            // Fill the width instead of shrinking to the table.
            .auto_shrink([false, true])
            .scroll_bar_rect(egui::Rect::everything_above(20.0))
            .show(ui, |ui| {
                const MARGIN: f32 = 8.0;

                ui.add_space(MARGIN * 2.0);
                let rect = ui.available_rect_before_wrap().shrink2(egui::vec2(MARGIN, 0.0));
                let table_width = rect.width();

                // Text-sized columns: `NemesisExt` just fits; the priority header (with the sort
                // arrow) is not squashed.
                let spacing = ui.spacing();
                let padding = spacing.button_padding.x.mul_add(2.0, spacing.item_spacing.x);
                let mod_type_width = text_width(ui, ModType::NemesisExt.as_str())
                    .max(text_width(ui, self.ctx.i18n.t(I18nKey::ColumnModType)))
                    + padding;
                let priority_width =
                    text_width(ui, &format!("{} ▲", self.ctx.i18n.t(I18nKey::ColumnPriority))) + padding;
                const CHECKBOX_WIDTH: f32 = 24.0;
                let (id_width, name_width) = (table_width * 0.20, table_width * 0.28);

                // Site gets the rest. Computed here (not `Column::remainder`): egui_extras keeps a
                // remainder column's width in sync with the window only when it is the last one.
                let gaps = spacing.item_spacing.x * 6.0;
                let site_width =
                    (table_width - (CHECKBOX_WIDTH + id_width + name_width + mod_type_width + priority_width) - gaps)
                        .max(80.0);

                // | checkbox | id | name | mod type | site (fills the rest) | priority |
                // NOTE: The column border is the resize handle: keep every column resizable.
                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    egui_extras::TableBuilder::new(ui)
                        .striped(true)
                        .auto_shrink([false, true])
                        .column(egui_extras::Column::exact(CHECKBOX_WIDTH)) // checkbox
                        .column(Self::resizable_column(id_width, changed_width)) // id
                        .column(Self::resizable_column(name_width, changed_width)) // name
                        .column(Self::resizable_column(mod_type_width, changed_width)) // mod type
                        .column(Self::resizable_column(site_width, changed_width).clip(true)) // site
                        .column(Self::resizable_column(priority_width, changed_width)) // priority
                        .header(20.0, |mut header| self.render_table_header(&mut header, rows))
                        .body(|mut body| {
                            let mut widths = [0.0_f32; 6];
                            widths.clone_from_slice(body.widths());

                            let mod_list: &mut [_] =
                                if self.page.fetch.last_was_empty { &mut [] } else { self.ctx.settings.mod_list_mut() };
                            let anchor = &mut self.page.selection_anchor;
                            match rows {
                                None => dnd_table_body(&mut body, mod_list, widths, anchor),
                                Some(rows) => check_only_table_body(body, rows, mod_list, widths, anchor),
                            }
                        });
                });
            });
    }

    /// Creates a resizable column that also resets to `width`
    /// whenever the table's total width changes.
    ///
    /// `egui_extras::Column` cannot simultaneously support `initial` (user
    /// resizable) and automatic width tracking.  This helper switches to
    /// `exact` for exactly one frame when `changed_width` is `true`, giving
    /// egui a concrete measurement to anchor the new layout.
    fn resizable_column(width: f32, changed_width: bool) -> egui_extras::Column {
        if changed_width { egui_extras::Column::exact(width) } else { egui_extras::Column::initial(width) }
            .resizable(true)
    }

    /// Renders the table header row with sortable column buttons.
    fn render_table_header(&mut self, header: &mut egui_extras::TableRow<'_, '_>, rows: Option<&[u32]>) {
        let i18n = self.ctx.i18n;
        self.checkbox_header_button(header, rows);
        self.header_button(header, i18n.t(I18nKey::ColumnId), SortColumn::Id);
        self.header_button(header, i18n.t(I18nKey::ColumnName), SortColumn::Name);
        self.header_button(header, i18n.t(I18nKey::ColumnModType), SortColumn::ModType);
        self.header_button(header, i18n.t(I18nKey::ColumnSite), SortColumn::Site);
        self.header_button(header, i18n.t(I18nKey::ColumnPriority), SortColumn::Priority);
    }

    /// The "check all / uncheck all" box in the header. It affects the mods on screen: all of them
    /// in the draggable table (`rows` = `None`), else only those of the read-only view.
    fn checkbox_header_button(&mut self, header: &mut egui_extras::TableRow<'_, '_>, rows: Option<&[u32]>) {
        header.col(|ui| {
            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                if ui.add(egui::Checkbox::without_text(&mut self.page.check_all)).clicked() {
                    let check_all = self.page.check_all;
                    let items = self.ctx.settings.mod_list_mut();
                    match rows {
                        None => items.iter_mut().for_each(|item| item.enabled = check_all),
                        Some(rows) => {
                            for &row in rows {
                                if let Some(item) = items.get_mut(row as usize) {
                                    item.enabled = check_all;
                                }
                            }
                        }
                    }
                }
            });
        });
    }

    /// Renders one sortable header button.
    ///
    /// The button label gets a `▲` / `▼` suffix when this column is the
    /// active sort column.
    fn header_button(&mut self, header: &mut egui_extras::TableRow<'_, '_>, label: &str, column: SortColumn) {
        header.col(|ui| {
            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                let text = if self.ctx.settings.view_state.sort_column == column {
                    if self.ctx.settings.view_state.sort_asc { format!("{label} ▲") } else { format!("{label} ▼") }
                } else {
                    label.to_string()
                };

                if ui.button(text).clicked() {
                    visible_mods::toggle_sort(&mut self.ctx.settings.view_state, column);
                }
            });
        });
    }
}

/// Width of `text` drawn as a button label (no wrapping).
fn text_width(ui: &egui::Ui, text: &str) -> f32 {
    egui::WidgetText::from(text)
        .into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, egui::TextStyle::Button)
        .size()
        .x
}
