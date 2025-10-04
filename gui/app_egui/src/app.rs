use crate::{
    dnd::{check_only_table_body, dnd_table_body},
    i18n::{I18nKey, I18nMap},
    log::get_log_dir,
    mod_item::{inherit_reorder_cast, to_patches, ModItem, SortColumn},
};
use eframe::{egui, App, Frame};
use egui::{Checkbox, Separator};
use rayon::prelude::*;
use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc, Mutex},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataMode {
    /// Virtual File System mode.(MO2 etc.)
    Vfs,
    /// Manual mode.
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}

/// Main application state for Mod Manager.
pub struct ModManagerApp {
    /// Execution mode. VFS or not
    pub mode: DataMode,
    /// Target Skyrim runtime when behavior generation. SE or LE
    pub target_runtime: skyrim_data_dir::Runtime,

    /// Windows users can automatically read from the registry, but Unix-based OS users need to set the path manually.
    pub vfs_skyrim_data_dir: String,
    /// Mod list when booting with vfs in MO2.
    ///
    /// (The IDs in this list do not overlap, so Settings can be reused on other PCs.)
    pub vfs_mod_list: Vec<ModItem>,

    /// Skyrim data dir(When use manually specified mode)
    pub skyrim_data_dir: String,
    /// Mod List when using manually.
    ///
    /// (The IDs in this list may be duplicated, so they are absolute paths,
    /// and cannot be reused on other PCs unless the path, including the drive letter, is the same.)
    pub mod_list: Vec<ModItem>,

    /// The directory containing the HKX templates you want to patch.
    ///
    /// Typically this is a directory like `assets/templates`. The actual patch target directory
    /// should be a subdirectory such as `assets/templates/meshes`.
    pub template_dir: String,
    pub output_dir: String,
    /// Output d merge patches & merged json files.(To <Output dir>/.d_merge/patches/.debug)
    pub enable_debug_output: bool,
    pub auto_remove_meshes: bool,
    pub filter_text: String,
    pub sort_column: SortColumn,
    pub sort_asc: bool,
    pub i18n: I18nMap,
    pub log_level: LogLevel,
    pub transparent: bool,
    pub last_window_size: egui::Vec2,
    pub last_window_pos: egui::Pos2,
    pub last_window_maximized: bool,
    pub font_path: Option<PathBuf>,

    // ====================== Non export Settings targets =================================
    //
    pub async_rt: tokio::runtime::Runtime,

    /// Unless the priority order is ascending, moving items will disrupt the order, so lock them.
    pub is_locked: bool,
    /// Global "check all" flag.
    pub check_all: bool,
    pub notification: Arc<Mutex<String>>,

    pub log_lines: Arc<Mutex<Vec<String>>>,
    pub log_watcher_started: bool,
    pub show_log_window: Arc<AtomicBool>,
    /// It exists because mod_info must be loaded automatically only on the first run.
    pub is_first_render: bool,
    pub prev_table_available_width: f32,
    /// Even if no mod info can be retrieved, We want to maintain the check status and display it as empty.
    pub fetch_is_empty: bool,
}

impl Default for ModManagerApp {
    fn default() -> Self {
        Self {
            // == For Settings targets ==
            mode: DataMode::Vfs,
            target_runtime: skyrim_data_dir::Runtime::Se,
            enable_debug_output: false,
            auto_remove_meshes: true,

            vfs_skyrim_data_dir: String::new(),
            vfs_mod_list: vec![],

            skyrim_data_dir: String::new(),
            mod_list: vec![],

            template_dir: String::new(),
            output_dir: String::new(),
            filter_text: String::new(),
            sort_column: SortColumn::Priority,
            sort_asc: true,
            i18n: I18nMap::load_translation(),
            log_level: LogLevel::Debug,
            transparent: true,
            last_window_size: egui::Vec2::ZERO,
            last_window_pos: egui::Pos2::ZERO,
            last_window_maximized: false,
            font_path: None,

            // ============
            async_rt: tokio::runtime::Runtime::new().unwrap(),
            is_locked: false,
            check_all: false,
            log_lines: Arc::new(Mutex::new(Vec::new())),
            log_watcher_started: false,
            show_log_window: Arc::new(AtomicBool::new(false)),
            notification: Arc::new(Mutex::new(String::new())),
            is_first_render: true,
            prev_table_available_width: 0.0,
            fetch_is_empty: true,
        }
    }
}

impl App for ModManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.start_log_watcher(ctx);
        self.update_window_info(ctx);

        self.ui_execution_mode(ctx);
        self.ui_skyrim_dir(ctx);
        self.ui_output_dir(ctx);
        self.ui_search_panel(ctx);
        self.ui_mod_list(ctx);
        self.ui_notification(ctx);
        self.ui_bottom_panel(ctx);
        self.ui_log_window(ctx);

        self.is_first_render = false;
    }

    // Called when the app is about to close
    //
    // NOTE: Using mem take!
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let settings = crate::settings::AppSettings::from(core::mem::take(self));
        settings.save();
        crate::i18n::I18nMap::save_translation();
    }
}

impl ModManagerApp {
    /// Start watcher for Log viewer.
    fn start_log_watcher(&mut self, ctx: &egui::Context) {
        if !self.log_watcher_started {
            let log_lines = Arc::clone(&self.log_lines);
            let ctx = ctx.clone();

            let log_path = crate::log::get_log_dir(&self.output_dir).join(crate::log::LOG_FILENAME);
            if let Err(err) = crate::log::start_log_tail(&log_path, log_lines, Some(ctx)) {
                tracing::error!("Couldn't start log watcher: {err}");
            };
            self.log_watcher_started = true;
        }
    }

    /// To save settings.
    fn update_window_info(&mut self, ctx: &egui::Context) {
        let (pos, size, maximized) = ctx.input(|i| {
            // NOTE: Accessing self internally causes deadlock
            // Do not write directly to self within the closure
            let mut temp_pos = None;
            let mut temp_size = None;
            let mut temp_maximized = None;

            if let Some(view_info) = i.raw.viewports.get(&egui::ViewportId::ROOT) {
                temp_maximized = Some(view_info.maximized.unwrap_or(false));
                if let Some(outer_rect) = view_info.outer_rect {
                    temp_size = Some(outer_rect.size());
                    temp_pos = Some(outer_rect.min);
                }
            }

            (temp_pos, temp_size, temp_maximized)
        });

        if let Some(pos) = pos {
            self.last_window_pos = pos;
        }
        if let Some(size) = size {
            self.last_window_size = size;
        }
        if let Some(max) = maximized {
            self.last_window_maximized = max;
        }
    }

    fn ui_execution_mode(&mut self, ctx: &egui::Context) {
        let mut panel = egui::TopBottomPanel::top("top_execution_mode");
        if self.transparent {
            panel = panel.frame(egui::Frame::new());
        }

        panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                let vfs_mode_label = self.t(I18nKey::VfsMode).to_string();
                let vfs_mode_hover = self.t(I18nKey::VfsModeHover).to_string();
                let manual_mode_label = self.t(I18nKey::ManualMode).to_string();
                let manual_mode_hover = self.t(I18nKey::ManualModeHover).to_string();

                ui.label(self.t(I18nKey::ExecutionModeLabel));
                if ui
                    .radio_value(&mut self.mode, DataMode::Vfs, vfs_mode_label)
                    .on_hover_text(vfs_mode_hover)
                    .clicked()
                {
                    self.update_mod_list();
                };
                if ui
                    .radio_value(&mut self.mode, DataMode::Manual, manual_mode_label)
                    .on_hover_text(manual_mode_hover)
                    .clicked()
                {
                    self.update_mod_list();
                };

                ui.add(Separator::default().vertical());

                self.ui_target_runtime_box(ui);

                let debug_output_label = self.t(I18nKey::DebugOutput).to_string();
                let debug_output_hover = self.t(I18nKey::DebugOutputHover).to_string();
                ui.checkbox(&mut self.enable_debug_output, debug_output_label)
                    .on_hover_text(debug_output_hover);

                let auto_remove_meshes_label = self.t(I18nKey::AutoRemoveMeshes).to_string();
                let auto_remove_meshes_hover = self.t(I18nKey::AutoRemoveMeshesHover).to_string();
                ui.checkbox(&mut self.auto_remove_meshes, auto_remove_meshes_label)
                    .on_hover_text(auto_remove_meshes_hover);

                ui.add_space(30.0);

                let transparent_label = self.t(I18nKey::Transparent).to_string();
                let transparent_hover = self.t(I18nKey::TransparentHover).to_string();
                ui.checkbox(&mut self.transparent, transparent_label)
                    .on_hover_text(transparent_hover);
            });
        });
    }

    fn ui_target_runtime_box(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(self.t(I18nKey::RuntimeTargetLabel))
                .on_hover_text(self.t(I18nKey::RuntimeTargetHover));

            egui::ComboBox::from_id_salt("skyrim_runtime_target")
                .selected_text(self.target_runtime.as_str())
                .show_ui(ui, |ui| {
                    let runtimes = [
                        (skyrim_data_dir::Runtime::Le, "SkyrimLE"),
                        (skyrim_data_dir::Runtime::Se, "SkyrimSE"),
                        (skyrim_data_dir::Runtime::Vr, "SkyrimVR"),
                    ];

                    for (runtime, label) in runtimes {
                        if ui
                            .selectable_value(&mut self.target_runtime, runtime, label)
                            .changed()
                            && self.mode == DataMode::Vfs
                        {
                            if let Ok(data_dir) =
                                skyrim_data_dir::get_skyrim_data_dir(self.target_runtime)
                            {
                                self.skyrim_data_dir = data_dir.display().to_string();
                            };
                        }
                    }
                });
        });
    }

    /// Skyrim data directory selection panel.
    fn ui_skyrim_dir(&mut self, ctx: &egui::Context) {
        let mut panel = egui::TopBottomPanel::top("top_data_dir");
        if self.transparent {
            panel = panel.frame(egui::Frame::new());
        }

        panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(self.t(I18nKey::SkyrimDataDirLabel)).clicked() {
                    if let Err(err) = open_existing_dir_or_ancestor(self.current_skyrim_data_dir())
                    {
                        self.set_notification(err);
                    };
                };

                self.draw_skyrim_dir_ui(ui);

                if ui
                    .add_sized(
                        [60.0, 40.0],
                        egui::Button::new(self.t(I18nKey::SelectButton)),
                    )
                    .clicked()
                {
                    let dialog = {
                        let default_dir = {
                            let path = Path::new(self.current_skyrim_data_dir());
                            path.canonicalize().ok()
                        };
                        match default_dir {
                            Some(default_dir) => rfd::FileDialog::new().set_directory(default_dir),
                            None => rfd::FileDialog::new(),
                        }
                    };

                    if let Some(dir) = dialog.pick_folder() {
                        match self.mode {
                            DataMode::Vfs => {
                                self.vfs_skyrim_data_dir = dir.display().to_string();
                                self.update_mod_list();
                            }
                            DataMode::Manual => {
                                self.skyrim_data_dir = dir.display().to_string();
                                self.update_mod_list();
                            }
                        };
                    }
                }
            });
        });
    }

    /// Output directory selection panel.
    fn ui_output_dir(&mut self, ctx: &egui::Context) {
        let mut panel = egui::TopBottomPanel::top("top_output_dir");
        if self.transparent {
            panel = panel.frame(egui::Frame::new());
        }

        panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                let output_dir_label = self.t(I18nKey::OutputDirLabel);
                if ui.button(output_dir_label).clicked() {
                    if let Err(err) = open_existing_dir_or_ancestor(Path::new(&self.output_dir)) {
                        self.set_notification(err);
                    };
                };
                let _ = ui.add_sized(
                    [ui.available_width() * 0.9, 40.0],
                    egui::TextEdit::singleline(&mut self.output_dir),
                );

                if ui
                    .add_sized(
                        [60.0, 40.0],
                        egui::Button::new(self.t(I18nKey::SelectButton)),
                    )
                    .clicked()
                {
                    let dialog = if !self.output_dir.is_empty() {
                        // NOTE: For some reason, we can't reach the path correctly without using canonicalize.
                        match find_existing_dir_or_ancestor(&self.output_dir) {
                            Ok(abs_path) => rfd::FileDialog::new().set_directory(abs_path),
                            Err(err) => {
                                self.set_notification(format!(
                                    "Couldn't find output dir or ancestor: {err}"
                                ));
                                return;
                            }
                        }
                    } else {
                        rfd::FileDialog::new()
                    };

                    if let Some(dir) = dialog.pick_folder() {
                        self.output_dir = dir.display().to_string();
                    }
                }
            });
        });
    }

    /// Search & lock panel.
    fn ui_search_panel(&mut self, ctx: &egui::Context) {
        let mut panel = egui::TopBottomPanel::top("top_panel");
        if self.transparent {
            panel = panel.frame(egui::Frame::new());
        }

        panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(self.t(I18nKey::SearchLabel));
                ui.add_sized(
                    [300.0, 40.0],
                    egui::TextEdit::singleline(&mut self.filter_text),
                );

                if ui
                    .add_sized(
                        [60.0, 40.0],
                        egui::Button::new(self.t(I18nKey::ClearButton)),
                    )
                    .clicked()
                {
                    self.filter_text.clear();
                }

                if self.is_locked {
                    let lock_button_response = ui
                        .add_sized([60.0, 40.0], egui::Button::new(self.t(I18nKey::LockButton)))
                        .on_hover_text(self.t(I18nKey::LockButtonHover));
                    if lock_button_response.clicked() {
                        self.unlock_readonly_table();
                    }
                }
            });
        });
    }

    /// Notification bar at bottom.
    fn ui_notification(&self, ctx: &egui::Context) {
        let mut panel = egui::TopBottomPanel::bottom("notification_panel");
        if self.transparent {
            panel = panel.frame(egui::Frame::new());
        }

        panel.show(ctx, |ui| {
            ui.label(self.notification());
        });
    }

    /// Bottom panel with buttons (Log, Patch).
    fn ui_bottom_panel(&mut self, ctx: &egui::Context) {
        let mut panel = egui::TopBottomPanel::bottom("bottom_panel");
        if self.transparent {
            panel = panel.frame(egui::Frame::new());
        }

        panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.ui_log_level_box(ui);

                self.add_button(ui, ctx, I18nKey::LogDir, |s, _| {
                    if let Err(err) = open_existing_dir_or_ancestor(get_log_dir(&s.output_dir)) {
                        s.set_notification(err);
                    }
                });
                self.add_button(ui, ctx, I18nKey::LogButton, |s, _| {
                    s.show_log_window
                        .fetch_xor(true, std::sync::atomic::Ordering::Relaxed); // Intended: toggle
                });
                self.add_button(ui, ctx, I18nKey::NotificationClearButton, |s, _| {
                    s.clear_notification();
                });
                self.add_button(ui, ctx, I18nKey::PatchButton, |s, _| {
                    s.patch();
                });
            });
        });
    }

    /// Add bottom button
    fn add_button<F>(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, key: I18nKey, f: F)
    where
        F: FnOnce(&mut Self, &egui::Context),
    {
        if ui
            .add_sized([120.0, 40.0], egui::Button::new(self.t(key)))
            .clicked()
        {
            f(self, ctx);
        }
    }

    fn ui_log_level_box(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label("Log Level");

            egui::ComboBox::from_id_salt("log_level")
                .selected_text(self.log_level.as_str())
                .show_ui(ui, |ui| {
                    let levels: [(LogLevel, &'static str); 5] = [
                        (LogLevel::Error, "Error"),
                        (LogLevel::Warn, "Warn"),
                        (LogLevel::Info, "Info"),
                        (LogLevel::Debug, "Debug"),
                        (LogLevel::Trace, "Trace"),
                    ];

                    for (level, label) in levels {
                        if ui
                            .selectable_value(&mut self.log_level, level, label)
                            .changed()
                        {
                            tracing_rotation::change_level(level.as_str()).unwrap();
                        }
                    }
                });
        });
    }

    /// Deferred log viewer window.
    fn ui_log_window(&self, ctx: &egui::Context) {
        if self
            .show_log_window
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            let show_log_window = Arc::clone(&self.show_log_window);
            let log_lines = Arc::clone(&self.log_lines);
            let clear_button_name = self.t(I18nKey::ClearButton).to_string();

            ctx.show_viewport_deferred(
                egui::ViewportId::from_hash_of("log_viewer"),
                egui::ViewportBuilder {
                    title: Some("Log viewer".to_string()),
                    inner_size: Some(egui::Vec2::new(1300.0, 800.0)),
                    resizable: Some(true),
                    ..Default::default()
                },
                move |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Deferred,
                        "This egui backend doesn't support multiple viewports"
                    );

                    egui::CentralPanel::default()
                        .frame(egui::Frame::new())
                        .show(ctx, |ui| {
                            ui.horizontal(|ui| {
                                if ui.button(clear_button_name.as_str()).clicked() {
                                    log_lines.lock().unwrap().clear();
                                }

                                ui.button("Copy").clicked().then(|| {
                                    let text = log_lines.lock().unwrap().join("\n");
                                    ui.ctx().copy_text(text);
                                });
                            });

                            egui::ScrollArea::vertical()
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    let text = log_lines.lock().unwrap().join("\n");
                                    ui.label(text);
                                });
                        });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        show_log_window.store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                },
            );
        }
    }
}

impl ModManagerApp {
    /// Central panel with mod list table.
    fn ui_mod_list(&mut self, ctx: &egui::Context) {
        let mut panel = egui::CentralPanel::default();
        if self.transparent {
            panel = panel.frame(egui::Frame::new());
        }

        panel.show(ctx, |ui| {
            ui.heading(self.t(I18nKey::ModsListTitle));
            ui.separator();

            // 1. Filter
            let mut filtered = self.filtered_mods();

            // 2. Sort
            self.sort_mods(&mut filtered);

            // 3. Decide if DnD is allowed
            let dnd_allowed = self.is_dnd_allowed();
            self.is_locked = !dnd_allowed;

            // 4. Render
            self.render_table(ui, &filtered, dnd_allowed);

            ui.add_space(10.0);
        });
    }

    /// Filter cloned  mods according to current search text.
    fn filtered_mods(&self) -> Vec<ModItem> {
        self.mod_list()
            .par_iter()
            .filter(|&m| {
                let q = self.filter_text.to_lowercase();
                q.trim().is_empty()
                    || m.id.to_lowercase().contains(&q)
                    || m.name.to_lowercase().contains(&q)
                    || m.site.to_lowercase().contains(&q)
            })
            .cloned()
            .collect()
    }

    /// Sort mods according to current sort settings.
    fn sort_mods(&self, mods: &mut [ModItem]) {
        mods.sort_by(|a, b| {
            let ord = match self.sort_column {
                SortColumn::Id => a.id.cmp(&b.id),
                SortColumn::Name => a.name.cmp(&b.name),
                SortColumn::Site => a.site.cmp(&b.site),
                SortColumn::Priority => a.priority.cmp(&b.priority),
            };
            if self.sort_asc {
                ord
            } else {
                ord.reverse()
            }
        });
    }

    /// Returns true if drag-and-drop reordering is currently allowed.
    fn is_dnd_allowed(&self) -> bool {
        self.filter_text.trim().is_empty()
            && self.sort_column == SortColumn::Priority
            && self.sort_asc
    }
}

impl ModManagerApp {
    /// Render mods table (with headers + rows).
    fn render_table(&mut self, ui: &mut egui::Ui, filtered_mods: &[ModItem], editable: bool) {
        let table_max_height = ui.available_height() * 0.9;
        let total_width = ui.available_width();

        let changed_width = (self.prev_table_available_width - total_width).abs() > 0.5; // ignore 0.5px diff
        if changed_width {
            self.prev_table_available_width = total_width;
        }

        egui::ScrollArea::vertical()
            .max_height(table_max_height)
            .max_width(total_width)
            .scroll_bar_rect(egui::Rect::everything_above(20.0)) // The scroll bar was too long, so I shortened it.
            .show(ui, |ui| {
                egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .column(egui_extras::Column::auto().resizable(true)) // 1/5: checkbox
                    .column(Self::resizable_column(total_width, 0.20, changed_width)) // 2/5: id
                    .column(Self::resizable_column(total_width, 0.30, changed_width)) // 3/5: name
                    .column(Self::resizable_column(total_width, 0.40, changed_width)) // 4/5: site
                    .column(egui_extras::Column::remainder().resizable(true)) // 5/5: priority
                    .header(20.0, |mut header| self.render_table_header(&mut header))
                    .body(|mut body| {
                        let mut widths = [0.0; 5]; // 5 ==  column count
                        widths.clone_from_slice(body.widths());

                        let mod_list = if self.fetch_is_empty {
                            &mut vec![] // Apply dummy to preserve check state.
                        } else {
                            self.mod_list_mut()
                        };

                        if editable {
                            dnd_table_body(body.ui_mut(), mod_list, widths);
                        } else {
                            check_only_table_body(&mut body, filtered_mods, mod_list, widths);
                        }
                    });
            });
    }

    /// Create a resizable column that also auto-adjusts once when the table width changes.
    ///
    /// `egui_extras::Column` does not simultaneously support both automatic resizing based
    /// on `available_width` and user-initiated resizing via `.resizable(true)`. This helper
    /// implements a hack that enables this by making the width exact only momentarily during resizing.
    fn resizable_column(total_width: f32, ratio: f32, changed_width: bool) -> egui_extras::Column {
        let width = total_width * ratio;

        if changed_width {
            egui_extras::Column::exact(width)
        } else {
            egui_extras::Column::initial(width)
        }
        .resizable(true)
    }

    /// Render table header (column titles with sort toggles).
    fn render_table_header(&mut self, header: &mut egui_extras::TableRow<'_, '_>) {
        let path_label = self.t(I18nKey::ColumnId).to_string();
        let name_label = self.t(I18nKey::ColumnName).to_string();
        let site_label = self.t(I18nKey::ColumnSite).to_string();
        let priority_label = self.t(I18nKey::ColumnPriority).to_string();

        self.checkbox_header_button(header);
        self.header_button(header, &path_label, SortColumn::Id);
        self.header_button(header, &name_label, SortColumn::Name);
        self.header_button(header, &site_label, SortColumn::Site);
        self.header_button(header, &priority_label, SortColumn::Priority);
    }

    /// Check all mods header button.
    fn checkbox_header_button(&mut self, header: &mut egui_extras::TableRow<'_, '_>) {
        header.col(|ui| {
            if ui
                .add(Checkbox::without_text(&mut self.check_all))
                .clicked()
            {
                let filtered_ids: Vec<String> = self
                    .mod_list()
                    .par_iter()
                    .filter(|m| {
                        self.filter_text.trim().is_empty()
                            || m.id
                                .to_lowercase()
                                .contains(&self.filter_text.to_lowercase())
                            || m.name
                                .to_lowercase()
                                .contains(&self.filter_text.to_lowercase())
                            || m.site
                                .to_lowercase()
                                .contains(&self.filter_text.to_lowercase())
                    })
                    .map(|item| item.id.clone())
                    .collect();

                // Update filtered's enabled state to match self.check_all
                let check_all = self.check_all;
                for filtered_id in filtered_ids {
                    if let Some(orig_item) = self
                        .mod_list_mut()
                        .par_iter_mut()
                        .find_any(|o| o.id == filtered_id)
                    {
                        orig_item.enabled = check_all;
                    }
                }
            }
        });
    }

    /// Helper: render one header button with sort logic.
    fn header_button(
        &mut self,
        header: &mut egui_extras::TableRow<'_, '_>,
        label: &str,
        column: SortColumn,
    ) {
        header.col(|ui| {
            let text = match self.sort_column {
                _ if self.sort_column == column && self.sort_asc => format!("{label} ▲"),
                _ if self.sort_column == column && !self.sort_asc => format!("{label} ▼"),
                _ => label.to_string(),
            };

            if ui.button(text).clicked() {
                self.toggle_sort(column);
            }
        });
    }
}

// mod info loader
impl ModManagerApp {
    /// - vfs -> self.vfs_skyrim_data_dir
    /// - others -> self.skyrim_data_dir
    const fn current_skyrim_data_dir(&self) -> &str {
        match self.mode {
            DataMode::Vfs => self.vfs_skyrim_data_dir.as_str(),
            DataMode::Manual => self.skyrim_data_dir.as_str(),
        }
    }

    /// - vfs -> vfs_mod_list
    /// - others -> mod_list
    const fn mod_list(&self) -> &Vec<ModItem> {
        match self.mode {
            DataMode::Vfs => &self.vfs_mod_list,
            DataMode::Manual => &self.mod_list,
        }
    }

    /// - vfs -> vfs_mod_list
    /// - others -> mod_list
    const fn mod_list_mut(&mut self) -> &mut Vec<ModItem> {
        match self.mode {
            DataMode::Vfs => &mut self.vfs_mod_list,
            DataMode::Manual => &mut self.mod_list,
        }
    }

    /// Unlocks the table for editing.(Asc Priority & Clear filter)
    fn unlock_readonly_table(&mut self) {
        self.sort_asc = true;
        self.sort_column = SortColumn::Priority;
        self.filter_text.clear();
    }

    #[inline]
    fn toggle_sort(&mut self, column: SortColumn) {
        if self.sort_column == column {
            self.sort_asc = !self.sort_asc;
        } else {
            self.sort_column = column;
            self.sort_asc = true;
            self.filter_text.clear();
        }
    }

    fn draw_skyrim_dir_ui(&mut self, ui: &mut egui::Ui) {
        if self.mode == DataMode::Vfs {
            let dir = match skyrim_data_dir::get_skyrim_data_dir(self.target_runtime) {
                Ok(dir) => dir,
                Err(_err) => {
                    #[cfg(target_os = "windows")]
                    let exe_suffix = if self.target_runtime == skyrim_data_dir::Runtime::Se {
                        "SE"
                    } else {
                        ""
                    };

                    #[cfg(target_os = "windows")]
                    let err_msg = format!(
                        "Error: Could not find Skyrim{exe_suffix}.exe path in the Windows registry: {_err}\n\
                        If you are not using the Steam version of Skyrim, please specify the Skyrim data directory manually."
                    );

                    #[cfg(not(target_os = "windows"))]
                    let err_msg = "NOTE: `get_skyrim_data_dir` is not supported on this platform(Linux, MacOs). Please specify the Skyrim data directory manually.".to_string();
                    self.set_notification(err_msg);

                    PathBuf::new()
                }
            };
            let dir_str = dir.display().to_string();

            if self.vfs_skyrim_data_dir.trim().is_empty() {
                self.vfs_skyrim_data_dir = dir_str;
            }

            let response = ui.add_sized(
                [ui.available_width() * 0.9, 40.0],
                egui::TextEdit::singleline(&mut self.vfs_skyrim_data_dir),
            );

            if self.is_first_render || response.changed() {
                self.update_mod_list();
            }
        } else {
            let response = ui.add_sized(
                [ui.available_width() * 0.9, 40.0],
                egui::TextEdit::singleline(&mut self.skyrim_data_dir)
                    .hint_text("D:\\GAME\\ModOrganizer Skyrim SE\\mods\\*"),
            );

            if self.is_first_render || response.changed() {
                self.update_mod_list();
            }
        }
    }

    /// Update mod info based on file search according to the current mode (vfs or manual).
    ///
    /// # Note
    /// The only difference between vfs and manual is the id.
    /// For manual, due to the possibility of duplicates, the path up to the Nemesis ID (e.g., `aaaa`) becomes the id, but vfs uses the Nemesis ID directly.
    ///
    /// This allows vfs mode to maintain the check state on a different PC.
    fn update_mod_list(&mut self) {
        match mod_info::get_all(self.current_skyrim_data_dir(), self.mode == DataMode::Vfs) {
            Ok(new_mods) => {
                let is_empty = new_mods.is_empty();
                self.fetch_is_empty = is_empty;
                if is_empty {
                    return; // To preserve check state even if empty
                }

                let new_mods = inherit_reorder_cast(self.mod_list(), new_mods);
                let _ = core::mem::replace(self.mod_list_mut(), new_mods);
            }
            Err(err) => {
                let err_title = self.t(I18nKey::ErrorReadingModInfo);
                self.set_notification(format!("{err_title} {err}"));
            }
        }
    }
}

impl ModManagerApp {
    /// Set message to notification
    pub fn set_notification<S: Into<String>>(&self, msg: S) {
        if let Ok(mut guard) = self.notification.lock() {
            let msg = msg.into();
            tracing::info!("{msg}");
            *guard = msg;
        }
    }

    pub fn clear_notification(&self) {
        if let Ok(mut guard) = self.notification.lock() {
            guard.clear();
        }
    }

    /// Get notification message
    pub fn notification(&self) -> String {
        self.notification
            .lock()
            .ok()
            .map(|s| s.clone())
            .unwrap_or_default()
    }
}

// i18n
impl ModManagerApp {
    /// Translate given key or fallback to default English.
    #[inline]
    fn t(&self, key: I18nKey) -> &str {
        self.i18n.t(key)
    }
}

impl ModManagerApp {
    fn patch(&self) {
        let patches = match self.mode {
            DataMode::Vfs => to_patches(&self.vfs_skyrim_data_dir, true, &self.vfs_mod_list),
            DataMode::Manual => to_patches(&self.skyrim_data_dir, false, &self.mod_list),
        };
        let is_debug_mode = self.enable_debug_output;

        if self.auto_remove_meshes {
            self.remove_meshes_dir_all();
        }

        let notify = Arc::clone(&self.notification);
        self.async_rt.spawn(nemesis_merge::behavior_gen(
            patches,
            nemesis_merge::Config {
                resource_dir: self.template_dir.clone().into(),
                output_dir: self.output_dir.clone().into(),
                output_target: match self.target_runtime {
                    skyrim_data_dir::Runtime::Le => nemesis_merge::OutPutTarget::SkyrimLe,
                    skyrim_data_dir::Runtime::Se | skyrim_data_dir::Runtime::Vr => {
                        nemesis_merge::OutPutTarget::SkyrimSe
                    }
                },
                status_report: Some(Box::new(move |status| {
                    let mut n = notify.lock().unwrap();
                    *n = status.to_string();
                })),
                hack_options: Some(nemesis_merge::HackOptions {
                    cast_ragdoll_event: true,
                }),
                debug: nemesis_merge::DebugOptions {
                    output_patch_json: is_debug_mode,
                    output_merged_json: is_debug_mode,
                    output_merged_xml: false,
                },
                skyrim_data_dir_glob: Some(self.current_skyrim_data_dir().to_string()),
            },
        ));
    }

    /// Removes the auto `<output dir>/meshes` or `<output dir>/.d_merge/debug` directories with a safety warning if output_dir equals Skyrim data dir.
    fn remove_meshes_dir_all(&self) {
        let output_dir = &self.output_dir;
        let skyrim_data_directory = self.current_skyrim_data_dir();

        let is_dangerous_remove = Path::new(output_dir)
            .canonicalize()
            .unwrap_or_else(|_| Path::new(output_dir).to_path_buf())
            == Path::new(skyrim_data_directory)
                .canonicalize()
                .unwrap_or_else(|_| Path::new(skyrim_data_directory).to_path_buf());

        if is_dangerous_remove {
            let warn = "0/5: The `auto remove meshes` option is checked, but the output directory is the Skyrim data directory.\nSince deleting meshes in that location risks destroying mods, the process was skipped.";
            tracing::warn!("{warn}");
        } else {
            self.set_notification(format!("0/5:  Removing `{output_dir}/meshes` directory..."));
            crate::cache_remover::remove_meshes_dir_all(output_dir);
        }
    }
}

/// Walks up the path hierarchy until an existing directory is found.
///
/// # Returns
/// * `Ok(PathBuf)` with the existing directory path
/// * `Err(String)` if no existing directory is found
fn find_existing_dir_or_ancestor<P>(dir: P) -> Result<PathBuf, String>
where
    P: AsRef<Path>,
{
    let dir = dir.as_ref();

    let mut current = dir;

    while !current.exists() {
        match current.parent() {
            Some(parent) => current = parent,
            None => {
                return Err(format!(
                    "No existing directory found in path hierarchy({})",
                    dir.display()
                ))
            }
        }
    }

    current
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize path({}): {e}", dir.display()))
}

/// Opens the given directory or the closest existing parent directory.
///
/// # Returns
/// * `Ok(())` if a directory was successfully opened.
/// * `Err(String)` if no existing directory could be opened.
fn open_existing_dir_or_ancestor<P>(dir: P) -> Result<(), String>
where
    P: AsRef<Path>,
{
    let dir = dir.as_ref();

    let abs_dir = find_existing_dir_or_ancestor(dir)?;
    open::that_detached(abs_dir)
        .map_err(|e| format!("Failed to open directory({}: {e}", dir.display()))
}
