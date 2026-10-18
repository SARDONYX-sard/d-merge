//! Mod list: toolbar, drag & drop table and its context menu.

use std::rc::Rc;

use d_merge_gui_shared::{i18n::I18nKey, mod_item::ModItem, settings::mod_list_ui::SortColumn};
use freya::prelude::*;
use freya_extra::{
    dnd_table::{Column, ColumnWidth, DataTable},
    dropdown_menu::DropdownMenu,
};

use crate::{
    actions::{self, row_id},
    state::{AppState, Shared},
    ui::{
        BUTTON_HEIGHT, check_tile, heading, icon, icon_button, icon_only, icons, panel, row,
        separator, sized_button, with_tooltip, wrap_row,
    },
};

/// Returns the mods matching the filter text and column, in priority order.
fn filtered_mods(mods: &[ModItem], filter_text: &str, column: Option<SortColumn>) -> Vec<ModItem> {
    let text = filter_text.trim().to_lowercase();
    if text.is_empty() {
        return mods.to_vec();
    }

    let matches = |m: &&ModItem| match column {
        None => {
            m.id.to_lowercase().contains(&text)
                || m.name.to_lowercase().contains(&text)
                || m.mod_type.as_lower_str().contains(&text)
                || m.site.to_lowercase().contains(&text)
        }
        Some(SortColumn::Id) => m.id.to_lowercase().contains(&text),
        Some(SortColumn::Name) => m.name.to_lowercase().contains(&text),
        Some(SortColumn::ModType) => m.mod_type.as_lower_str().contains(&text),
        Some(SortColumn::Site) => m.site.to_lowercase().contains(&text),
        Some(SortColumn::Priority) => m.priority.to_string().contains(&text),
    };

    mods.iter().filter(matches).cloned().collect()
}

/// Mods shown in the table.
fn visible_mods(app: &AppState) -> Vec<ModItem> {
    if *app.last_fetch_was_empty.read() {
        return vec![];
    }

    let settings = app.settings.read();
    let mut mods = filtered_mods(
        settings.mod_list(),
        &settings.ui.mod_list.filter_text,
        settings.ui.mod_list.filter_column,
    );
    mods.sort_by_key(|item| item.priority);
    mods
}

#[derive(PartialEq)]
pub(crate) struct ModList;

impl Component for ModList {
    fn render(&self) -> impl IntoElement {
        let mut table_height = use_state(|| 300.0_f32);

        panel().height(Size::fill()).content(Content::flex()).child(ModListToolbar).child(
            rect()
                .width(Size::fill())
                .height(Size::flex(1.0))
                .on_sized(move |event: Event<SizedEventData>| {
                    let height = event.area.size.height;
                    if (height - *table_height.peek()).abs() > 0.5 {
                        table_height.set(height);
                    }
                })
                .child(ModTable { max_height: *table_height.read() }),
        )
    }
}

#[derive(PartialEq)]
struct ModListToolbar;

impl Component for ModListToolbar {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();
        let shared = use_consume::<Shared>();

        let is_fetching = *app.fetching.read();
        let mod_list_msg = app.mod_list_msg.read().clone();
        let (filter_active, filter_column, selected_count, total_count) = {
            let settings = app.settings.read();
            let mods = settings.mod_list();
            (
                !settings.ui.mod_list.filter_text.trim().is_empty(),
                settings.ui.mod_list.filter_column,
                mods.iter().filter(|item| item.enabled).count(),
                mods.len(),
            )
        };

        let filter_text = Writable::from_state(app.settings).map(
            |settings| &settings.ui.mod_list.filter_text,
            |settings| &mut settings.ui.mod_list.filter_text,
        );

        let column_select = [
            (None, I18nKey::FilterTextAll),
            (Some(SortColumn::Id), I18nKey::ColumnId),
            (Some(SortColumn::Name), I18nKey::ColumnName),
            (Some(SortColumn::ModType), I18nKey::ColumnModType),
            (Some(SortColumn::Site), I18nKey::ColumnSite),
            (Some(SortColumn::Priority), I18nKey::ColumnPriority),
        ]
        .into_iter()
        .fold(DropdownMenu::new(filter_column), |menu, (column, key)| menu.item(column, app.t(key)))
        .on_change(move |next| {
            let mut settings = app.settings;
            settings.write().ui.mod_list.filter_column = next;
        });

        // -- Reload Button (egui: icon only, disabled + spinner while fetching)
        let reload_button = with_tooltip(
            icon_only(
                if is_fetching { icons::refresh_off() } else { icons::refresh() },
                !is_fetching,
                move |_| actions::update_mod_list(&app, &shared),
            ),
            format!("{} (Ctrl + R)", app.t(I18nKey::ReloadButton)),
        );

        // Check state of the visible mods. (egui: the checkbox in the table header)
        let visible = visible_mods(&app);
        let all_visible_checked = !visible.is_empty() && visible.iter().all(|item| item.enabled);
        let check_all = check_tile(all_visible_checked, "", move |()| {
            let ids: Vec<String> = visible_mods(&app).into_iter().map(|item| item.id).collect();
            actions::set_enabled(&app, Some(&ids), !all_visible_checked);
        });

        // egui:
        // | Mods | (search)[search......] [Clear] [All v] | [= Normalize] [Lock] | (reload) msg |
        // | 0 / 14 selected                                                                    |
        let toolbar = wrap_row()
            .child(with_tooltip(
                heading(app.t(I18nKey::ModsListTitle)),
                app.t(I18nKey::ModsListTitleHover),
            ))
            .child(separator())
            .child(icon(icons::search(), 18.0))
            .child(
                Input::new(filter_text)
                    .placeholder(format!("{}...", app.t(I18nKey::SearchLabel)))
                    .width(Size::px(300.0)),
            )
            .child(sized_button(app.t(I18nKey::ClearButton), 60.0, move |_| {
                let mut settings = app.settings;
                settings.write().ui.mod_list.filter_text.clear();
            }))
            .child(column_select)
            .child(separator())
            .child(with_tooltip(
                icon_button(
                    Some(icons::normalize()),
                    app.t(I18nKey::NormalizeButton),
                    (110.0, BUTTON_HEIGHT),
                    move |_| actions::normalize(&app),
                ),
                app.t(I18nKey::NormalizeHover),
            ))
            // While filtering, the table is still reorderable(hidden rows keep their place),
            // but pressing this restores the full, unsorted view.
            .child(if filter_active {
                let colors = get_theme_or_default().read().colors.clone();
                with_tooltip(
                    icon_button(
                        Some(icons::lock()),
                        app.t(I18nKey::LockButton),
                        (90.0, BUTTON_HEIGHT),
                        move |_| {
                            let mut settings = app.settings;
                            settings.write().ui.mod_list.filter_text.clear();
                            actions::refresh_table(&app);
                        },
                    )
                    // egui: `ButtonVariant::Destructive`
                    .theme_colors(ButtonColorsThemePartial {
                        background: Some(Preference::Specific(colors.error)),
                        hover_background: Some(Preference::Specific(
                            colors.error.mul_if(true, 0.85),
                        )),
                        color: Some(Preference::Specific(Color::WHITE)),
                        ..Default::default()
                    }),
                    app.t(I18nKey::LockButtonHover),
                )
            } else {
                // egui: `ui.add_space(98.0)`
                rect().width(Size::px(90.0)).into_element()
            })
            .child(separator())
            .child(reload_button)
            .maybe_child(is_fetching.then(|| CircularLoader::new().size(18.0).into_element()))
            .child(
                label()
                    .text(mod_list_msg.text)
                    .map(mod_list_msg.color, |label, color| label.color(color)),
            );

        let selected_label = app.t(I18nKey::SelectedLabel);
        rect().spacing(4.0).width(Size::fill()).child(toolbar).child(
            row()
                .child(check_all)
                .child(label().text(format!("{selected_count} / {total_count} {selected_label}"))),
        )
    }
}

type ModColumn = Column<ModItem>;

fn enabled_cell(item: &ModItem, app: &AppState, visible_ids: Rc<Vec<String>>) -> Element {
    let app = *app;
    let id = item.id.clone();

    rect()
        .height(Size::fill())
        .width(Size::fill())
        .main_align(Alignment::Center)
        .cross_align(Alignment::Center)
        .child(Checkbox::new().selected(item.enabled).size(16.0))
        .on_focus_press(move |_| actions::toggle_mod(&app, &id, &visible_ids))
        .into_element()
}

fn site_cell(item: &ModItem) -> Element {
    if item.site.trim().is_empty() {
        return rect().into_element();
    }

    let url = item.site.clone();
    rect()
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| Platform::get().open_url(url.clone()))
        .child(
            label()
                .text(item.site.clone())
                .font_size(13.0)
                .max_lines(1)
                .text_overflow(TextOverflow::Ellipsis)
                .color(Color::from_rgb(100, 170, 255)),
        )
        .into_element()
}

fn columns(app: &AppState, visible_ids: &Rc<Vec<String>>) -> Vec<ModColumn> {
    let app = *app;
    let visible_ids = Rc::clone(visible_ids);

    vec![
        ModColumn::new("enabled", "", move |item| {
            enabled_cell(item, &app, Rc::clone(&visible_ids))
        })
        .width(ColumnWidth::Px(44.0))
        .resizable(false)
        .sort_by_key(|item| item.enabled),
        ModColumn::text("id", app.t(I18nKey::ColumnId), |item| item.id.clone())
            .width(ColumnWidth::Ratio(0.20))
            .sort_by_key(|item| item.id.clone()),
        ModColumn::text("name", app.t(I18nKey::ColumnName), |item| item.name.clone())
            .width(ColumnWidth::Ratio(0.30))
            .sort_by_key(|item| item.name.clone()),
        ModColumn::text("mod_type", app.t(I18nKey::ColumnModType), |item| {
            item.mod_type.as_str().to_string()
        })
        .width(ColumnWidth::Ratio(0.09))
        .sort_by_key(|item| item.mod_type),
        ModColumn::new("site", app.t(I18nKey::ColumnSite), site_cell)
            .width(ColumnWidth::Ratio(0.27))
            .sort_by_key(|item| item.site.clone()),
        ModColumn::text("priority", app.t(I18nKey::ColumnPriority), |item| {
            item.priority.to_string()
        })
        .sort_by_key(|item| item.priority),
    ]
}

#[derive(PartialEq)]
struct ModTable {
    max_height: f32,
}

impl Component for ModTable {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();
        let generation = *app.table_generation.read();

        let rows = visible_mods(&app);
        let visible_ids: Rc<Vec<String>> =
            Rc::new(rows.iter().map(|item| item.id.clone()).collect());

        // NOTE: `max_height` is the height of the body. The header(about 48px) is drawn above it.
        let max_height = (self.max_height - 60.0).max(80.0);
        let table = DataTable::new(columns(&app, &visible_ids), rows, |item| row_id(&item.id))
            .reorderable_rows()
            .reorderable_columns()
            .default_sort("priority", true)
            .on_rows_reorder(move |ids| actions::apply_row_order(&app, ids))
            .table(move |table| table.max_height(max_height));

        let mut context_menu = app.context_menu;

        // NOTE: The table keeps its own row order and sort. Changing the key re-creates it.
        rect()
            .key(generation)
            .width(Size::fill())
            .on_pointer_down(move |event: Event<PointerEventData>| {
                if event.button() == Some(MouseButton::Right) {
                    let (x, y) = event.global_location().to_tuple();
                    context_menu.set(Some((x, y)));
                }
            })
            .child(table)
    }
}

/// Right-click menu of the table: enable/disable all or the visible mods.
#[derive(PartialEq)]
pub(crate) struct TableContextMenu;

impl Component for TableContextMenu {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();
        let mut menu = app.context_menu;
        let mut popup_area = use_state(|| None::<Area>);

        let Some((x, y)) = *menu.read() else {
            return rect().into_element();
        };

        let total = app.settings.read().mod_list().len();
        let visible_ids: Vec<String> = visible_mods(&app).into_iter().map(|item| item.id).collect();
        let shown = visible_ids.len();

        let mut items: Vec<(String, Option<Vec<String>>, bool)> =
            vec![("Enable all".to_string(), None, true), ("Disable all".to_string(), None, false)];
        if shown != total {
            items.push((format!("Enable visible ({shown})"), Some(visible_ids.clone()), true));
            items.push((format!("Disable visible ({shown})"), Some(visible_ids), false));
        }

        let entries = items.into_iter().map(|(title, scope, enabled)| {
            MenuItem::new()
                .on_press(move |_| {
                    actions::set_enabled(&app, scope.as_deref(), enabled);
                    menu.set(None);
                })
                .child(title)
                .into_element()
        });

        rect()
            .position(Position::new_global().top(0.0).left(0.0))
            .width(Size::fill())
            .height(Size::fill())
            .layer(Layer::Overlay)
            .on_global_pointer_up(move |event: Event<PointerEventData>| {
                let point = event.global_location();
                let inside =
                    popup_area.read().is_some_and(|area| contains_point(area, point.x, point.y));
                if !inside && event.button() != Some(MouseButton::Right) {
                    menu.set(None);
                }
            })
            .child(
                rect()
                    .position(Position::new_global().top(y as f32).left(x as f32))
                    .on_sized(move |event: Event<SizedEventData>| {
                        popup_area.set(Some(event.area));
                    })
                    .child(Menu::new().children(entries)),
            )
            .into_element()
    }
}

fn contains_point(area: Area, x: f64, y: f64) -> bool {
    let left = f64::from(area.origin.x);
    let top = f64::from(area.origin.y);
    let right = left + f64::from(area.size.width);
    let bottom = top + f64::from(area.size.height);

    x >= left && x <= right && y >= top && y <= bottom
}
