use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use eframe::{App, Frame, egui};
use egui::{Checkbox, Color32, Separator};
use parking_lot::RwLock;
use rayon::prelude::*;

use crate::{
    i18n::{I18nKey, I18nMap},
    log::get_log_dir,
    mod_item::{ModItem, SortColumn, inherit_reorder_cast, to_patches},
    ui::{
        confirm::{ConfirmAction, ConfirmDialog},
        dnd_table::{check_only_table_body, dnd_table_body},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DataMode {
    /// Virtual File System mode.(MO2 etc.)
    Vfs,
    /// Manual mode.
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}

/// Represents the state of a mod list fetching task.
#[derive(Debug)]
pub(crate) enum FetchState {
    /// No fetch is in progress.
    Idle,
    /// A background worker thread is currently fetching
    Fetching,

    /// Successfully fetched and the result is non-empty.
    Done { elapsed: std::time::Duration },

    /// Fetch succeeded but returned zero items.
    Empty { elapsed: std::time::Duration },

    /// Fetch failed with an error.
    Error { elapsed: std::time::Duration },
}

/// Main application state for Mod Manager.
pub(crate) struct ModManagerApp {
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
    /// If true, generates a FNIS.esp(dummy ESP) file.
    pub generate_fnis_esp: bool,

    /// Once the mod list has been updated, enable all mods and run the patch once.
    pub auto_run: bool,
    pub filter_column: Option<SortColumn>,
    pub filter_text: String,
    pub font_path: Option<PathBuf>,
    pub i18n: Arc<I18nMap>,
    pub last_window_maximized: bool,
    pub last_window_pos: egui::Pos2,
    pub last_window_size: egui::Vec2,
    pub log_level: LogLevel,
    pub sort_asc: bool,
    pub sort_column: SortColumn,
    pub transparent: bool,

    // ====================== Non export Settings targets =================================
    //
    pub async_rt: tokio::runtime::Runtime,
    pub confirm_dialog: ConfirmDialog,

    /// It exists because mod_info must be loaded automatically only on the first run.
    pub is_first_render: bool,
    pub show_help: bool,

    /// Unless the priority order is ascending, moving items will disrupt the order, so lock them.
    pub is_locked: bool,
    /// Global "check all" flag.
    pub check_all: bool,
    pub prev_table_available_width: f32,

    /// mod list fetching message with color
    pub mod_list_msg: (String, Color32),
    pub notify: (String, egui::Color32),

    pub log_lines: Arc<RwLock<Vec<String>>>,
    pub log_watcher_started: bool,
    pub show_log_window: Arc<AtomicBool>,

    /// Represents the current state of the mod list fetching process.
    ///
    /// Even if no mod info can be retrieved, we want to maintain the
    /// check status and display it as empty.
    pub fetch_state: Arc<RwLock<FetchState>>,
    pub fetched_mod_info: Arc<RwLock<Vec<mod_info::ModInfo>>>,
    pub last_fetch_was_empty: bool,

    /// Patch status
    pub patch_status: Arc<RwLock<Option<nemesis_merge::Status>>>,
    pub patch_start_time: Option<std::time::Instant>,
}

impl Default for ModManagerApp {
    fn default() -> Self {
        Self {
            // == For Settings targets ==
            mode: DataMode::Vfs,
            target_runtime: skyrim_data_dir::Runtime::Se,
            enable_debug_output: false,
            auto_remove_meshes: true,
            generate_fnis_esp: false,

            vfs_skyrim_data_dir: String::new(),
            vfs_mod_list: vec![],

            skyrim_data_dir: String::new(),
            mod_list: vec![],

            template_dir: String::new(),
            output_dir: String::new(),
            filter_text: String::new(),
            filter_column: None,
            sort_column: SortColumn::Priority,
            sort_asc: true,
            i18n: Arc::new(I18nMap::load_translation()),
            log_level: LogLevel::Debug,
            auto_run: false,
            transparent: true,
            last_window_size: egui::Vec2::ZERO,
            last_window_pos: egui::Pos2::ZERO,
            last_window_maximized: false,
            font_path: None,

            // ====================== Non export Settings targets =================================
            //
            async_rt: tokio::runtime::Runtime::new().unwrap(),
            confirm_dialog: ConfirmDialog::default(),

            is_first_render: true,
            show_help: false,

            is_locked: false,
            check_all: false,

            log_lines: Arc::new(RwLock::new(Vec::new())),
            log_watcher_started: false,
            show_log_window: Arc::new(AtomicBool::new(false)),
            prev_table_available_width: 0.0,

            mod_list_msg: (String::new(), egui::Color32::WHITE),
            notify: (String::new(), egui::Color32::WHITE),

            fetch_state: Arc::new(RwLock::new(FetchState::Idle)),
            fetched_mod_info: Arc::new(RwLock::new(vec![])),
            last_fetch_was_empty: false,

            patch_status: Arc::new(RwLock::new(None)),
            patch_start_time: None,
        }
    }
}

impl App for ModManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.poll_fetch_result();
        self.poll_patch_result();
        self.start_log_watcher(ctx);
        self.update_window_info(ctx);

        self.ui_execution_mode(ctx);
        self.ui_skyrim_dir(ctx);
        self.ui_output_dir(ctx);
        self.ui_search_panel(ctx);

        self.ui_notification(ctx);
        // NOTE: TopBottomPanel must be added before any other panels.
        // The first added panel becomes the outermost (front-most), and the last
        // becomes the innermost. Central panels must always be added last.
        // See: https://docs.rs/flatbox/0.1.0/flatbox/egui/struct.TopBottomPanel.html
        self.ui_bottom_panel(ctx);
        self.ui_mod_list(ctx);
        self.ui_log_window(ctx);
        self.ui_help_window(ctx);
        self.ui_show_confirm(ctx);

        self.is_first_render = false;
    }

    // Called when the app is about to close
    //
    // NOTE: Using mem take!
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let settings = crate::settings::AppSettings::from(core::mem::take(self));
        settings.save();
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

                let generate_fnis_esp_label = self.t(I18nKey::GenerateFnisEspLabel).to_string();
                let generate_fnis_esp_hover = self.t(I18nKey::GenerateFnisEspHover).to_string();
                ui.checkbox(&mut self.generate_fnis_esp, generate_fnis_esp_label)
                    .on_hover_text(generate_fnis_esp_hover);

                ui.add_space(30.0);

                let transparent_label = self.t(I18nKey::Transparent).to_string();
                let transparent_hover = self.t(I18nKey::TransparentHover).to_string();
                ui.checkbox(&mut self.transparent, transparent_label)
                    .on_hover_text(transparent_hover);

                let auto_run_label = self.t(I18nKey::AutoRun).to_string();
                let auto_run_hover = self.t(I18nKey::AutoRunHover).to_string();
                ui.checkbox(&mut self.auto_run, auto_run_label).on_hover_text(auto_run_hover);
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
                        if ui.selectable_value(&mut self.target_runtime, runtime, label).changed()
                            && self.mode == DataMode::Vfs
                        {
                            #[cfg(target_os = "windows")]
                            self.update_vfs_skyrim_data_dir_by_reg();
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
                if ui.button(self.t(I18nKey::SkyrimDataDirLabel)).clicked()
                    && let Err(err) = open_existing_dir_or_ancestor(self.current_skyrim_data_dir())
                {
                    self.set_colored_notify(err, Color32::RED);
                };

                self.draw_skyrim_dir_ui(ui);

                // NOTE: Due to the need to read the registry, it only works on Windows and is meaningless unless in VFS state.
                #[cfg(target_os = "windows")]
                if self.mode == DataMode::Vfs
                    && ui
                        .add_sized(
                            [60.0, 40.0],
                            egui::Button::new(self.t(I18nKey::AutoDetectButton)),
                        )
                        .on_hover_text(self.t(I18nKey::AutoDetectHover))
                        .clicked()
                {
                    self.update_vfs_skyrim_data_dir_by_reg();
                }

                if ui
                    .add_sized([60.0, 40.0], egui::Button::new(self.t(I18nKey::SelectButton)))
                    .clicked()
                {
                    let dialog = match find_existing_dir_or_ancestor(self.current_skyrim_data_dir())
                    {
                        Ok(abs_path) => rfd::FileDialog::new().set_directory(abs_path),
                        Err(_) => rfd::FileDialog::new(),
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
                if ui.button(output_dir_label).clicked()
                    && let Err(err) = open_existing_dir_or_ancestor(Path::new(&self.output_dir))
                {
                    self.set_colored_notify(err, Color32::RED);
                };
                let text_line = egui::TextEdit::singleline(&mut self.output_dir);
                let text_line = if self.transparent {
                    text_line.background_color(egui::Color32::TRANSPARENT)
                } else {
                    text_line
                };
                let _ = ui.add_sized([ui.available_width() * 0.9, 40.0], text_line);

                if ui
                    .add_sized([60.0, 40.0], egui::Button::new(self.t(I18nKey::SelectButton)))
                    .clicked()
                {
                    let dialog = if !self.output_dir.is_empty() {
                        // NOTE: For some reason, we can't reach the path correctly without using canonicalize.
                        match find_existing_dir_or_ancestor(&self.output_dir) {
                            Ok(abs_path) => rfd::FileDialog::new().set_directory(abs_path),
                            Err(err) => {
                                let err = format!("Couldn't find output dir or ancestor: {err}");
                                self.set_colored_notify(err, Color32::RED);
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

                let text_line = egui::TextEdit::singleline(&mut self.filter_text);
                let text_line = if self.transparent {
                    text_line.background_color(egui::Color32::TRANSPARENT)
                } else {
                    text_line
                };
                ui.add_sized([300.0, 40.0], text_line);

                if ui
                    .add_sized([60.0, 40.0], egui::Button::new(self.t(I18nKey::ClearButton)))
                    .clicked()
                {
                    self.filter_text.clear();
                }

                // Pre-fetch labels to avoid simultaneous borrow of self
                let all_label = self.t(I18nKey::FilterTextAll).to_string();
                let id_label = self.t(I18nKey::ColumnId).to_string();
                let name_label = self.t(I18nKey::ColumnName).to_string();
                let mod_type_label = self.t(I18nKey::ColumnModType).to_string();
                let site_label = self.t(I18nKey::ColumnSite).to_string();
                let priority_label = self.t(I18nKey::ColumnPriority).to_string();

                let selected_text = match self.filter_column {
                    None => all_label.clone(),
                    Some(SortColumn::Id) => id_label.clone(),
                    Some(SortColumn::Name) => name_label.clone(),
                    Some(SortColumn::ModType) => mod_type_label.clone(),
                    Some(SortColumn::Site) => site_label.clone(),
                    Some(SortColumn::Priority) => priority_label.clone(),
                };

                egui::ComboBox::from_id_salt("filter_column").selected_text(selected_text).show_ui(
                    ui,
                    |ui| {
                        ui.selectable_value(&mut self.filter_column, None, &all_label);
                        ui.selectable_value(
                            &mut self.filter_column,
                            Some(SortColumn::Id),
                            &id_label,
                        );
                        ui.selectable_value(
                            &mut self.filter_column,
                            Some(SortColumn::Name),
                            &name_label,
                        );
                        ui.selectable_value(
                            &mut self.filter_column,
                            Some(SortColumn::ModType),
                            &mod_type_label,
                        );
                        ui.selectable_value(
                            &mut self.filter_column,
                            Some(SortColumn::Site),
                            &site_label,
                        );
                        ui.selectable_value(
                            &mut self.filter_column,
                            Some(SortColumn::Priority),
                            &priority_label,
                        );
                    },
                );

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
            ui.colored_label(self.notify.1, &self.notify.0);
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
                        s.set_colored_notify(err, Color32::RED);
                    }
                });
                self.add_button(ui, ctx, I18nKey::LogButton, |s, _| {
                    s.show_log_window.fetch_xor(true, Ordering::Relaxed); // Intended: toggle
                });
                self.add_button(ui, ctx, I18nKey::NotificationClearButton, |s, _| {
                    s.clear_notification();
                });

                let is_fetching = matches!(*self.fetch_state.read(), FetchState::Fetching);
                ui.add_enabled_ui(!is_fetching, |ui| {
                    if ui
                        .add_sized(
                            [120.0, 40.0],
                            egui::Button::new(if is_fetching {
                                self.t(I18nKey::PatchFetchingButton)
                            } else {
                                self.t(I18nKey::PatchButton)
                            }),
                        )
                        .clicked()
                    {
                        self.patch();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_sized([30.0, 40.0], egui::Button::new(self.t(I18nKey::HelpButton)))
                        .clicked()
                    {
                        self.show_help = true;
                    }
                });
            });
        });
    }

    /// Add bottom button
    fn add_button<F>(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, key: I18nKey, f: F)
    where
        F: FnOnce(&mut Self, &egui::Context),
    {
        if ui.add_sized([120.0, 40.0], egui::Button::new(self.t(key))).clicked() {
            f(self, ctx);
        }
    }

    fn ui_log_level_box(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label(self.t(I18nKey::LogLevelLabel));

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
                        if ui.selectable_value(&mut self.log_level, level, label).changed() {
                            tracing_rotation::change_level(level.as_str()).unwrap();
                        }
                    }
                });
        });
    }

    /// Deferred log viewer window.
    fn ui_log_window(&self, ctx: &egui::Context) {
        if self.show_log_window.load(Ordering::Relaxed) {
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

                    egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button(clear_button_name.as_str()).clicked() {
                                log_lines.write().clear();
                            }

                            ui.button("Copy").clicked().then(|| {
                                let text = log_lines.read().join("\n");
                                ui.ctx().copy_text(text);
                            });
                        });

                        egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                            let text = log_lines.read().join("\n");
                            ui.label(text);
                        });
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        show_log_window.store(false, Ordering::Relaxed);
                    }
                },
            );
        }
    }
}

// help modal
impl ModManagerApp {
    fn ui_help_window(&mut self, ctx: &egui::Context) {
        if !self.show_help {
            return;
        }

        // Extract all strings/values needed inside the closure upfront
        // to avoid borrowing `self` inside the `Window::show` closure.
        let window_title = self.t(I18nKey::HelpButton).to_string();
        let issue_report_label = self.t(I18nKey::IssueReportButton).to_string();
        let issue_report_hover = self.t(I18nKey::IssueReportHover).to_string();
        let write_i18n_label = self.t(I18nKey::WriteNewI18nJsonButton).to_string();
        let write_i18n_hover = self.t(I18nKey::WriteNewI18nJsonHover).to_string();
        let issue_url = crate::app::create_issue_link(self);

        let mut show = true;
        let mut issue_clicked = false;
        let mut write_i18n_clicked = false;

        egui::Window::new(window_title)
            .open(&mut show)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                // ── App info ──────────────────────────────────────────
                ui.vertical_centered(|ui| {
                    ui.heading(format!("D Merge v{}", env!("CARGO_PKG_VERSION")));
                });
                ui.add_space(8.0);

                // ── Links ─────────────────────────────────────────────
                ui.separator();
                ui.add_space(4.0);

                egui::Grid::new("help_info_grid").num_columns(2).spacing([8.0, 6.0]).show(
                    ui,
                    |ui| {
                        ui.label("Author:");
                        ui.label(env!("CARGO_PKG_AUTHORS"));
                        ui.end_row();

                        ui.label("License:");
                        const LICENSE_URL: &str = concat!(
                            env!("CARGO_PKG_REPOSITORY"),
                            "/blob/",
                            env!("CARGO_PKG_VERSION"),
                            "/LICENSE"
                        );
                        ui.hyperlink_to(env!("CARGO_PKG_LICENSE"), LICENSE_URL)
                            .on_hover_text(LICENSE_URL);
                        ui.end_row();

                        const SOURCE_CODE_URL: &str = concat!(
                            env!("CARGO_PKG_REPOSITORY"),
                            "/tree/",
                            env!("CARGO_PKG_VERSION"),
                        );
                        ui.label("Source Code:");
                        ui.hyperlink_to("GitHub", SOURCE_CODE_URL).on_hover_text(SOURCE_CODE_URL);
                        ui.end_row();

                        ui.label("Change Log:");
                        const CHANGELOG_URL: &str =
                            concat!(env!("CARGO_PKG_REPOSITORY"), "/blob/main/CHANGELOG.md");
                        ui.hyperlink_to("CHANGELOG.md", CHANGELOG_URL).on_hover_text(CHANGELOG_URL);
                        ui.end_row();

                        const MOD_TEST_URL: &str = concat!(
                            env!("CARGO_PKG_REPOSITORY"),
                            "/blob/",
                            env!("CARGO_PKG_VERSION"),
                            "/docs/test_status.md"
                        );
                        ui.label("Mod Test Status:");
                        ui.hyperlink_to("test_status.md", MOD_TEST_URL).on_hover_text(MOD_TEST_URL);
                        ui.add_space(4.0);
                        ui.end_row();
                    },
                );
                ui.add_space(8.0);

                // ── Bug Report ────────────────────────────────────────
                ui.separator();
                ui.add_space(4.0);

                ui.label("Bug Report:");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    const ISSUE_URL: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/issues");
                    ui.hyperlink_to("See Issues", ISSUE_URL).on_hover_text(ISSUE_URL);
                    if ui.button(&issue_report_label).on_hover_text(&issue_report_hover).clicked() {
                        issue_clicked = true;
                    }
                });
                ui.add_space(8.0);

                // ── Tooling ───────────────────────────────────────────
                ui.separator();
                ui.add_space(4.0);

                ui.label("Tooling:");
                ui.add_space(4.0);
                if ui.button(&write_i18n_label).on_hover_text(&write_i18n_hover).clicked() {
                    write_i18n_clicked = true;
                }
                ui.add_space(8.0);
            });

        // Apply deferred mutations after the closure has released its borrow
        if !show {
            self.show_help = false;
        }
        if issue_clicked {
            ctx.open_url(egui::OpenUrl { url: issue_url, new_tab: true });
        }
        if write_i18n_clicked {
            self.confirm_dialog.open(write_i18n_hover, ConfirmAction::WriteI18nJson);
        }
    }

    fn ui_show_confirm(&mut self, ctx: &egui::Context) {
        // Render confirm dialog and capture the confirmed action outside the closure
        let confirmed_action = {
            let mut result = None;
            self.confirm_dialog.show(ctx, |action| {
                result = Some(action);
            });
            result
        };
        if let Some(action) = confirmed_action {
            match action {
                ConfirmAction::WriteI18nJson => match I18nMap::save_translation() {
                    Ok(()) => self
                        .set_colored_notify(format!("OK. Wrote {}", I18nMap::FILE), Color32::GREEN),
                    Err(err) => self.set_colored_notify(err.to_string(), Color32::RED),
                },
            }
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
            ui.horizontal(|ui| {
                ui.heading(self.t(I18nKey::ModsListTitle));

                if ui
                    .add(egui::Button::new(self.t(I18nKey::NormalizeButton)))
                    .on_hover_text(self.t(I18nKey::NormalizeHover))
                    .clicked()
                {
                    crate::mod_item::reorder_mods_priorities(self.mod_list_mut());
                }

                let is_fetching = matches!(*self.fetch_state.read(), FetchState::Fetching);
                if ui
                    .add_enabled(
                        !is_fetching,
                        egui::Button::new(format!("🔄 {}", self.t(I18nKey::ReloadButton))),
                    )
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

    /// Filter cloned  mods according to current search text.
    fn filtered_mod_ids(&self) -> Vec<ModItem> {
        if self.filter_text.trim().is_empty() {
            return self.mod_list().par_iter().cloned().collect(); // unused when DnD grid
        }

        // read only(but checkable grid)
        let text = self.filter_text.trim().to_lowercase();
        let matches_filter = |m: &&ModItem| match self.filter_column {
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
        self.mod_list().par_iter().filter(matches_filter).cloned().collect()
    }

    /// Sort mods according to current sort settings.
    fn sort_filtered_mods(&self, mods: &mut [ModItem]) {
        mods.par_sort_unstable_by(|a, b| {
            let ord = match self.sort_column {
                SortColumn::Id => a.id.cmp(&b.id),
                SortColumn::Name => a.name.cmp(&b.name),
                SortColumn::ModType => a.mod_type.cmp(&b.mod_type),
                SortColumn::Site => a.site.cmp(&b.site),
                SortColumn::Priority => a.priority.cmp(&b.priority),
            };
            if self.sort_asc { ord } else { ord.reverse() }
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
                    .column(egui_extras::Column::auto().resizable(true)) // 1/5: checkbox
                    .column(Self::resizable_column(total_width, 0.20, changed_width)) // 2/6: id
                    .column(Self::resizable_column(total_width, 0.30, changed_width)) // 3/6: name
                    .column(Self::resizable_column(total_width, 0.07, changed_width)) // 4/6: mod type(FNIS/Nemesis)
                    .column(Self::resizable_column(total_width, 0.30, changed_width)) // 5/6: site
                    .column(Self::resizable_column(total_width, 0.03, changed_width)) // 6/6: priority
                    .header(20.0, |mut header| self.render_table_header(&mut header))
                    .body(|mut body| {
                        let mut widths = [0.0; 6]; // 6 ==  column count
                        widths.clone_from_slice(body.widths());
                        let mod_list = if self.last_fetch_was_empty {
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

    /// Check all mods header button.
    fn checkbox_header_button(&mut self, header: &mut egui_extras::TableRow<'_, '_>) {
        header.col(|ui| {
            if ui.add(Checkbox::without_text(&mut self.check_all)).clicked() {
                let check_all = self.check_all;

                let filtered_ids: rapidhash::fast::RapidHashSet<_> =
                    self.filtered_mod_ids().into_par_iter().map(|m| m.id).collect();
                // If nothing has been searched for, everything is displayed, so everything is subject to checking.
                let is_empty_filtered_ids = filtered_ids.is_empty();

                self.mod_list_mut().par_iter_mut().for_each(|item| {
                    if is_empty_filtered_ids || filtered_ids.contains(&item.id) {
                        item.enabled = check_all;
                    }
                });
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
            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| {
                    let text = match self.sort_column {
                        _ if self.sort_column == column && self.sort_asc => format!("{label} ▲"),
                        _ if self.sort_column == column && !self.sort_asc => format!("{label} ▼"),
                        _ => label.to_string(),
                    };

                    if ui.button(text).clicked() {
                        self.toggle_sort(column);
                    }
                },
            );
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
        let changed = match self.mode {
            DataMode::Vfs => {
                if self.is_first_render && self.vfs_skyrim_data_dir.trim().is_empty() {
                    self.update_vfs_skyrim_data_dir_by_reg();
                    return;
                }

                let line = egui::TextEdit::singleline(&mut self.vfs_skyrim_data_dir);
                let line = if self.transparent {
                    line.background_color(egui::Color32::TRANSPARENT)
                } else {
                    line
                };

                ui.add_sized([ui.available_width() * 0.85, 40.0], line).changed()
            }
            DataMode::Manual => {
                let line = egui::TextEdit::singleline(&mut self.skyrim_data_dir)
                    .hint_text("D:\\GAME\\ModOrganizer Skyrim SE\\mods\\*");
                let line = if self.transparent {
                    line.background_color(egui::Color32::TRANSPARENT)
                } else {
                    line
                };

                ui.add_sized([ui.available_width() * 0.9, 40.0], line).changed()
            }
        };

        if self.is_first_render || changed {
            self.update_mod_list();
        }
    }

    /// Automatically detect the Skyrim Data directory based on the selected output format. This uses the Steam registry, so it will only work if you have launched Skyrim at least once.
    ///
    /// # Note
    /// Window only.
    fn update_vfs_skyrim_data_dir_by_reg(&mut self) {
        match skyrim_data_dir::get_skyrim_data_dir(self.target_runtime) {
            Ok(dir) => {
                let new_vfs_skyrim_data_dir = dir.display().to_string();
                if self.vfs_skyrim_data_dir != new_vfs_skyrim_data_dir {
                    self.vfs_skyrim_data_dir = new_vfs_skyrim_data_dir;
                    self.update_mod_list();
                }
            }
            Err(err) => {
                tracing::error!(%err);
                #[cfg(target_os = "windows")]
                let err_msg = self.t(I18nKey::NotifyErrWindowsRegistryNotFound).to_string();
                #[cfg(not(target_os = "windows"))]
                let err_msg = self.t(I18nKey::NotifyErrPlatformNotSupported).to_string();

                self.set_colored_notify(err_msg, Color32::RED);
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
        tracing::debug!("`update_mod_list` has been called.");

        self.mod_list_msg =
            (self.t(I18nKey::ModsListFetchStateFetching).to_string(), crate::i18n::EGUI_RIGHT_BLUE);
        *self.fetch_state.write() = FetchState::Fetching;

        let start_time = std::time::Instant::now();
        let dir = self.current_skyrim_data_dir().to_owned();
        let use_vfs = self.mode == DataMode::Vfs;

        let state = Arc::clone(&self.fetch_state);
        let fetched_mod_info = Arc::clone(&self.fetched_mod_info);

        std::thread::spawn(move || {
            // NOTE: If the number of rayon threads reaches the CPU limit, the UI will freeze,
            // so be careful not to let that happen internally.
            let new_state = match mod_info::get_all(&dir, use_vfs) {
                Ok(mod_info) => {
                    if mod_info.is_empty() {
                        FetchState::Empty { elapsed: start_time.elapsed() }
                    } else {
                        *fetched_mod_info.write() = mod_info;
                        FetchState::Done { elapsed: start_time.elapsed() }
                    }
                }
                Err(e) => {
                    tracing::error!(%e, "mod_info::get_all error");
                    FetchState::Error { elapsed: start_time.elapsed() }
                }
            };

            *state.write() = new_state;
        });
    }

    /// Polls for worker results and updates the fetch state.
    ///
    /// Outdated worker results (due to a forced fetch) are ignored.
    /// Only results matching the latest `fetch_generation` are applied.
    fn poll_fetch_result(&mut self) {
        let Some(state) = self.fetch_state.try_read() else {
            return;
        };

        match *state {
            FetchState::Done { elapsed } => {
                let elapsed_secs = elapsed.as_secs_f32();
                drop(state);

                let mod_info = core::mem::take(&mut *self.fetched_mod_info.write());
                let new_mods = inherit_reorder_cast(self.mod_list(), mod_info);
                self.check_all = new_mods.par_iter().all(|m| m.enabled);
                *self.mod_list_mut() = new_mods;

                *self.fetch_state.write() = FetchState::Idle; // NOTE: If we don't include this, it will go here every time, and the mod list will be overwritten unintentionally.
                self.last_fetch_was_empty = false;

                self.mod_list_msg = (
                    format!("{} ({elapsed_secs:.2} s)", self.t(I18nKey::ModsListFetchStateDone)),
                    Color32::GREEN,
                );

                if self.auto_run {
                    self.mod_list_mut().par_iter_mut().for_each(|m| m.enabled = true);
                    self.patch();
                }
            }
            FetchState::Empty { elapsed } => {
                let elapsed_secs = elapsed.as_secs_f32();
                drop(state);

                *self.fetch_state.write() = FetchState::Idle;
                self.last_fetch_was_empty = true;

                self.mod_list_msg = (
                    format!("{} ({elapsed_secs:.2} s)", self.t(I18nKey::ModsListFetchStateEmpty)),
                    Color32::WHITE,
                );
            }
            FetchState::Error { elapsed } => {
                let elapsed_secs = elapsed.as_secs_f32();
                drop(state);

                *self.fetch_state.write() = FetchState::Idle;

                self.mod_list_msg = (
                    format!("{} ({elapsed_secs:.2} s)", self.t(I18nKey::ModsListFetchStateError)),
                    Color32::RED,
                );
            }
            FetchState::Fetching | FetchState::Idle => {}
        }
    }
}

impl ModManagerApp {
    /// Set message to notification
    pub(crate) fn set_notify<S: Into<String>>(&mut self, msg: S) {
        self.set_colored_notify(msg, Color32::WHITE);
    }

    /// Set message to notification with color
    pub(crate) fn set_colored_notify<S>(&mut self, msg: S, color: egui::Color32)
    where
        S: Into<String>,
    {
        self.notify = (msg.into(), color);
    }

    pub(crate) fn clear_notification(&mut self) {
        self.notify.0.clear();
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
    fn patch(&mut self) {
        self.patch_start_time = Some(std::time::Instant::now());
        *self.patch_status.write() = None;

        let patches = match self.mode {
            DataMode::Vfs => to_patches(&self.vfs_skyrim_data_dir, true, &self.vfs_mod_list),
            DataMode::Manual => to_patches(&self.skyrim_data_dir, false, &self.mod_list),
        };
        let is_debug_mode = self.enable_debug_output;

        if self.auto_remove_meshes {
            self.remove_meshes_dir_all();
        }

        let patch_status = Arc::clone(&self.patch_status);

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
                    *patch_status.write() = Some(status);
                })),
                hack_options: Some(nemesis_merge::HackOptions {
                    cast_ragdoll_event: true,
                    bone_weight_outside_hkparam: true,
                }),
                debug: nemesis_merge::DebugOptions {
                    output_patch_json: is_debug_mode,
                    output_merged_json: is_debug_mode,
                    output_merged_xml: is_debug_mode,
                },
                skyrim_data_dir_glob: Some(self.current_skyrim_data_dir().to_string()),
                generate_fnis_esp: self.generate_fnis_esp,
            },
        ));
    }

    /// Removes the auto `<output dir>/meshes` or `<output dir>/.d_merge/debug` directories with a safety warning if output_dir equals Skyrim data dir.
    fn remove_meshes_dir_all(&mut self) {
        let output_dir = self.output_dir.clone();
        let skyrim_data_directory = self.current_skyrim_data_dir();

        if nemesis_merge::cache_remover::is_dangerous_remove(&output_dir, skyrim_data_directory) {
            tracing::warn!(
                "0/6: The `auto remove meshes` option is checked, but the output directory is the Skyrim data directory.\nSince deleting meshes in that location risks destroying mods, the process was skipped."
            );
        } else {
            self.set_notify(format!(
                "0/6: {} `{output_dir}/meshes`",
                self.t(I18nKey::RemovingMeshesMessage)
            ));

            nemesis_merge::cache_remover::remove_meshes_dir_all(output_dir);
        }
    }

    fn poll_patch_result(&mut self) {
        // By accessing `start_time` first, we minimize the cost of `RwLock::read` as much as possible
        let Some(start_time) = self.patch_start_time else {
            return;
        };
        let Some(Some(status)) = self.patch_status.try_read().as_deref().cloned() else {
            return;
        };

        if matches!(status, nemesis_merge::Status::Done) {
            self.patch_start_time = None;
            *self.patch_status.write() = None;
        }

        let color = crate::i18n::status_to_color(&status);
        let msg = crate::i18n::status_to_text(status, &self.i18n, start_time);
        self.set_colored_notify(msg, color);
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
                ));
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

fn create_issue_link(app: &ModManagerApp) -> String {
    use std::borrow::Cow;

    let skyrim_runtime = match app.target_runtime {
        skyrim_data_dir::Runtime::Le => gh_issue_link::SkyrimRuntime::Le,
        skyrim_data_dir::Runtime::Se => gh_issue_link::SkyrimRuntime::Se,
        skyrim_data_dir::Runtime::Vr => gh_issue_link::SkyrimRuntime::Vr,
    };

    let skyrim_data_dir: Option<Cow<'_, Path>> = if app.vfs_skyrim_data_dir.trim().is_empty() {
        skyrim_data_dir::get_skyrim_data_dir(app.target_runtime).ok().map(Cow::Owned)
    } else {
        Some(Path::new(&app.vfs_skyrim_data_dir).into())
    };

    let skyrim_version = skyrim_data_dir.and_then(|skyrim_data_dir| {
        let exe = match skyrim_runtime {
            gh_issue_link::SkyrimRuntime::Le => "TESV.exe",
            gh_issue_link::SkyrimRuntime::Se => "SkyrimSE.exe",
            gh_issue_link::SkyrimRuntime::Vr => "SkyrimVR.exe",
        };
        let exe_path = skyrim_data_dir.parent()?.join(exe);

        gh_issue_link::version::get_file_version(exe_path).map(|ver| ver.to_string()).ok()
    });
    gh_issue_link::new_gh_issue_link(
        env!("CARGO_PKG_VERSION"),
        skyrim_runtime,
        skyrim_version.as_deref(),
    )
}
