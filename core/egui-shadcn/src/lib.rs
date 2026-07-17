//! egui-shadcn: shadcn/ui-inspired widgets for egui.
#![allow(
    clippy::module_inception,
    clippy::too_many_arguments,
    clippy::new_without_default,
    clippy::if_same_then_else,
    clippy::let_and_return,
    clippy::manual_div_ceil,
    clippy::bool_comparison,
    clippy::needless_range_loop,
    clippy::collapsible_if,
    unused
)]

pub mod icons;
pub mod layout;
pub mod paint;
pub mod theme;
pub mod tokens;
pub mod widgets;

pub use egui_flex::FlexAlign;
pub use egui_flex::FlexItem;
pub use egui_flex::FlexJustify;
pub use icons::lucide_icon::LucideIcon;
pub use icons::paint_icon::paint_icon;
pub use icons::paint_icon::paint_icon_svg;
pub use layout::center::center;
pub use layout::flex::Flex;
pub use layout::flex_instance::FlexInst;
pub use theme::setup_fonts::setup_fonts;
pub use theme::shadcn_theme::ShadcnTheme;
pub use theme::shadcn_theme_ext::ShadcnThemeExt;
pub use tokens::alert_variant::AlertVariant;
pub use tokens::badge_variant::BadgeVariant;
pub use tokens::button_variant::ButtonVariant;
pub use tokens::component_size::ComponentSize;
pub use tokens::item_variant::ItemVariant;
pub use tokens::sheet_side::SheetSide;
pub use tokens::toast_variant::ToastVariant;
pub use tokens::toggle_variant::ToggleVariant;
pub use tokens::typography_variant::TypographyVariant;
pub use widgets::accordion::accordion::Accordion;
pub use widgets::alert::alert::Alert;
pub use widgets::alert_dialog::alert_dialog::AlertDialog;
pub use widgets::alert_dialog::alert_dialog_show::AlertDialogResult;
pub use widgets::area_chart::area_chart::AreaChart;
pub use widgets::area_chart::area_chart::AreaSeries;
pub use widgets::aspect_ratio::aspect_ratio::AspectRatio;
pub use widgets::avatar::avatar::Avatar;
pub use widgets::badge::badge::Badge;
pub use widgets::breadcrumb::breadcrumb::Breadcrumb;
pub use widgets::button::button::Button;
pub use widgets::button_group::button_group::ButtonGroup;
pub use widgets::calendar::calendar::Calendar;
pub use widgets::card::card::Card;
pub use widgets::carousel::carousel::Carousel;
pub use widgets::checkbox::checkbox::Checkbox;
pub use widgets::collapsible::collapsible::Collapsible;
pub use widgets::color_swatch::color_swatch::ColorSwatch;
pub use widgets::combobox::combobox::Combobox;
pub use widgets::command::command::Command;
pub use widgets::context_menu::context_menu::ContextMenu;
pub use widgets::date_picker::date_picker::DatePicker;
pub use widgets::date_picker::date_picker_state::DatePickerState;
pub use widgets::dialog::dialog::Dialog;
pub use widgets::drawer::drawer::Drawer;
pub use widgets::dropdown_menu::dropdown_menu::DropdownMenu;
pub use widgets::dropdown_menu::dropdown_menu::MenuItem;
pub use widgets::empty::empty::Empty;
pub use widgets::field::field_description::FieldDescription;
pub use widgets::field::field_group::FieldGroup;
pub use widgets::field::field_legend::FieldLegend;
pub use widgets::field::field_set::FieldSet;
pub use widgets::hover_card::hover_card::HoverCard;
pub use widgets::input::input::Input;
pub use widgets::input_group::input_group::InputGroup;
pub use widgets::input_otp::input_otp::InputOtp;
pub use widgets::item::item::Item;
pub use widgets::kbd::kbd::Kbd;
pub use widgets::label::label::Label;
pub use widgets::menubar::menubar::Menubar;
pub use widgets::navigation_menu::navigation_menu::NavigationMenu;
pub use widgets::number_input::number_input::NumberInput;
pub use widgets::pagination::pagination::Pagination;
pub use widgets::popover::popover::Popover;
pub use widgets::progress::progress::Progress;
pub use widgets::property_grid::property_grid::PropertyGrid;
pub use widgets::property_grid::property_row::PropertyRow;
pub use widgets::radio::radio::Radio;
pub use widgets::radio_group::radio_group::RadioGroup;
pub use widgets::resizable::resizable::Resizable;
pub use widgets::scroll_area::scroll_area::ScrollArea;
pub use widgets::select::select::Select;
pub use widgets::select::select::SelectValue;
pub use widgets::separator::separator::Separator;
pub use widgets::sheet::sheet::Sheet;
pub use widgets::sidebar::sidebar::Sidebar;
pub use widgets::skeleton::skeleton::Skeleton;
pub use widgets::slider::slider::Slider;
pub use widgets::spinner::spinner::Spinner;
pub use widgets::status_bar::status_bar::StatusBar;
pub use widgets::switch::switch::Switch;
pub use widgets::table::table::Table;
pub use widgets::tabs::tabs::IconTabs;
pub use widgets::tabs::tabs::TabEntry;
pub use widgets::tabs::tabs::Tabs;
pub use widgets::textarea::textarea::Textarea;
pub use widgets::toast::toast_entry::ToastEntry;
pub use widgets::toast::toast_state::ToastState;
pub use widgets::toggle::toggle::Toggle;
pub use widgets::toggle_group::toggle_group::ToggleGroup;
pub use widgets::toolbar::toolbar::Toolbar;
pub use widgets::tooltip::tooltip::Tooltip;
pub use widgets::typography::typography::Typography;
