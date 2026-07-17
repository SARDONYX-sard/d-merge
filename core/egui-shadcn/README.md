# egui-shadcn

[shadcn/ui](https://ui.shadcn.com)-inspired widget library for [egui](https://github.com/emilk/egui).

60+ beautifully styled, ready-to-use components with built-in light and dark theming. Drop-in replacements for native egui widgets plus higher-level components like dialogs, date pickers, sidebars, and editor-ready controls.

**[Live Demo](https://pjankiewicz.github.io/egui-shadcn/)** (runs in your browser via WebAssembly)

## Quick start

```toml
[dependencies]
egui-shadcn = "0.1"
```

```rust
// Set up the theme (e.g. in your eframe CreationContext)
egui_shadcn::setup_fonts(&cc.egui_ctx);
let theme = egui_shadcn::theme::shadcn_theme_light::light();
egui_shadcn::ShadcnThemeExt::set_shadcn_theme(&cc.egui_ctx, theme);

// Use components
egui_shadcn::Button::new("Click me").show(ui);
ui.add(egui_shadcn::Switch::new(&mut value).label("Dark mode"));
ui.add(egui_shadcn::Input::new(&mut text).placeholder("Type here..."));
ui.add(egui_shadcn::Select::new(&mut selected, &options).placeholder("Pick one..."));
```

## Components

| Category       | Widgets                                                                                                                                        |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| **Inputs**     | Button, Checkbox, ColorSwatch, Input, InputOtp, Radio, RadioGroup, Select, Slider, Switch, Textarea, Toggle, ToggleGroup, Combobox, DatePicker |
| **Layout**     | Accordion, AspectRatio, Card, Collapsible, Resizable, ScrollArea, Separator, StatusBar, Tabs, Toolbar, Flex                                    |
| **Overlay**    | AlertDialog, Command, ContextMenu, Dialog, Drawer, DropdownMenu, HoverCard, Menubar, NavigationMenu, Popover, Sheet, Tooltip                   |
| **Feedback**   | Alert, Badge, Progress, Skeleton, Spinner, Toast                                                                                               |
| **Data**       | Avatar, Breadcrumb, Calendar, Carousel, Pagination, Sidebar, Table                                                                             |
| **Typography** | Typography, Label, Kbd                                                                                                                         |
| **Grouping**   | ButtonGroup, InputGroup, FieldGroup, FieldSet, FieldLegend, FieldDescription, PropertyGrid, PropertyRow                                        |
| **Icons**      | 1600+ Lucide icons via `LucideIcon`                                                                                                            |

## Theming

Built-in light and dark themes:

```rust
let light = egui_shadcn::theme::shadcn_theme_light::light();
let dark = egui_shadcn::theme::shadcn_theme_dark::dark();
egui_shadcn::ShadcnThemeExt::set_shadcn_theme(ctx, dark);
```

## Examples

Run locally:

```sh
cargo run --example demo
cargo run --example shadcn_demo
cargo run --example component_dashboard
```

Or try the [live web demo](https://pjankiewicz.github.io/egui-shadcn/).

## License

MIT
