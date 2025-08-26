use crate::{
    dnd::{check_only_table_body, dnd_table_body},
    i18n::I18nKey,
    mod_item::{from_mod_infos, ModItem, SortColumn},
};
use eframe::{egui, App, Frame};
use egui::{Checkbox, Separator};
use rayon::prelude::*;
use std::{
    borrow::Cow,
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
    pub i18n: std::collections::HashMap<I18nKey, Cow<'static, str>>,
    pub log_level: LogLevel,
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
    pub is_first_render: bool,
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
            i18n: std::collections::HashMap::new(),
            log_level: LogLevel::Debug,
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
    }
}

impl ModManagerApp {
    /// Start watcher for Log viewer.
    fn start_log_watcher(&mut self, ctx: &egui::Context) {
        if !self.log_watcher_started {
            let log_lines = Arc::clone(&self.log_lines);
            let ctx = ctx.clone();

            crate::log::start_log_tail(log_lines, Some(ctx));
            self.log_watcher_started = true;
        }
    }

    /// To save settings.
    fn update_window_info(&mut self, ctx: &egui::Context) {
        let rect = ctx.screen_rect();
        self.last_window_size = rect.size();
        // self.last_window_pos = rect.left_top(); // TODO: Get current window position.

        ctx.viewport(|state| {
            self.last_window_maximized = state.builder.maximized.unwrap_or_default();
        });
    }

    fn ui_execution_mode(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_execution_mode").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let vfs_mode_label = self.t(I18nKey::VfsMode).to_string();
                let vfs_mode_hover = self.t(I18nKey::VfsModeHover).to_string();
                let manual_mode_label = self.t(I18nKey::ManualMode).to_string();
                let manual_mode_hover = self.t(I18nKey::ManualModeHover).to_string();

                ui.label(self.t(I18nKey::ExecutionModeLabel));
                ui.radio_value(&mut self.mode, DataMode::Vfs, vfs_mode_label)
                    .on_hover_text(vfs_mode_hover);
                ui.radio_value(&mut self.mode, DataMode::Manual, manual_mode_label)
                    .on_hover_text(manual_mode_hover);

                ui.add(Separator::default().vertical());

                let debug_output_label = self.t(I18nKey::DebugOutput).to_string();
                let debug_output_hover = self.t(I18nKey::DebugOutputHover).to_string();
                ui.checkbox(&mut self.enable_debug_output, debug_output_label)
                    .on_hover_text(debug_output_hover);

                let auto_remove_meshes_label = self.t(I18nKey::AutoRemoveMeshes).to_string();
                let auto_remove_meshes_hover = self.t(I18nKey::AutoRemoveMeshesHover).to_string();
                ui.checkbox(&mut self.auto_remove_meshes, auto_remove_meshes_label)
                    .on_hover_text(auto_remove_meshes_hover);
            });
        });
    }

    /// Skyrim data directory selection panel.
    fn ui_skyrim_dir(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_data_dir").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(self.t(I18nKey::SkyrimDataDirLabel));
                self.draw_skyrim_dir_ui(ui);

                if ui
                    .add_sized(
                        [ui.available_width() * 0.06, 40.0],
                        egui::Button::new(self.t(I18nKey::OpenButton)),
                    )
                    .clicked()
                {
                    let dir = match self.mode {
                        DataMode::Vfs => &self.vfs_skyrim_data_dir,
                        DataMode::Manual => &self.skyrim_data_dir,
                    };
                    let dialog = if !dir.is_empty() {
                        rfd::FileDialog::new().set_directory(dir)
                    } else {
                        rfd::FileDialog::new()
                    };

                    if let Some(dir) = dialog.pick_folder() {
                        match self.mode {
                            DataMode::Vfs => self.update_vfs_mod_list(&dir.display().to_string()),
                            DataMode::Manual => self.update_mod_list(&dir.display().to_string()),
                        };
                    }
                }
            });
        });
    }

    /// Output directory selection panel.
    fn ui_output_dir(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_output_dir").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let output_dir_label = self.t(I18nKey::OutputDirLabel);
                if ui.button(output_dir_label).clicked() {
                    let path = Path::new(&self.output_dir);
                    let _ = std::fs::create_dir_all(path);
                    if let Ok(abs_dir) = path.canonicalize() {
                        if let Err(err) = open::that_detached(abs_dir) {
                            self.set_notification(format!("Couldn't  open output dir: {err}"));
                        };
                    }
                };
                let _ = ui.add_sized(
                    [ui.available_width() * 0.9, 40.0],
                    egui::TextEdit::singleline(&mut self.output_dir),
                );

                if ui
                    .add_sized(
                        [ui.available_width() * 0.06, 40.0],
                        egui::Button::new(self.t(I18nKey::OpenButton)),
                    )
                    .clicked()
                {
                    let dialog = if !self.output_dir.is_empty() {
                        // NOTE: For some reason, we can't reach the path correctly without using canonicalize.
                        let _ = std::fs::create_dir_all(&self.output_dir);
                        let path = std::path::Path::new(&self.output_dir);
                        rfd::FileDialog::new().set_directory(path.canonicalize().unwrap())
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
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
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
                    ui.add_space(ui.available_width() - 60.0);
                    let lock_button_response = ui
                        .add_sized([60.0, 40.0], egui::Button::new("🔒"))
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
        egui::TopBottomPanel::bottom("notification_panel").show(ctx, |ui| {
            ui.label(self.notification());
        });
    }

    /// Bottom panel with buttons (Log, Patch).
    fn ui_bottom_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.ui_log_level_box(ui);

                if ui
                    .add_sized([120.0, 40.0], egui::Button::new(self.t(I18nKey::LogDir)))
                    .clicked()
                {
                    let path = std::path::Path::new(crate::log::LOG_DIR);
                    let path = path.canonicalize().unwrap_or_default();
                    if let Err(err) = open::that_detached(path) {
                        self.set_notification(err.to_string());
                    }
                }
                if ui
                    .add_sized([120.0, 40.0], egui::Button::new(self.t(I18nKey::LogButton)))
                    .clicked()
                {
                    self.show_log_window
                        .fetch_xor(true, std::sync::atomic::Ordering::Relaxed); // toggle
                }

                if ui
                    .add_sized(
                        [120.0, 40.0],
                        egui::Button::new(self.t(I18nKey::NotificationClearButton)),
                    )
                    .clicked()
                {
                    self.clear_notification();
                }

                if ui
                    .add_sized(
                        [120.0, 40.0],
                        egui::Button::new(self.t(I18nKey::PatchButton)),
                    )
                    .clicked()
                {
                    self.patch();
                }
            });
        });
    }

    fn ui_log_level_box(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label("Log Level");

            egui::ComboBox::from_id_salt("log_level")
                .selected_text(format!("{:?}", self.log_level))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(&mut self.log_level, LogLevel::Error, "Error")
                        .changed()
                    {
                        tracing_rotation::change_level("error").unwrap();
                    }
                    if ui
                        .selectable_value(&mut self.log_level, LogLevel::Warn, "Warn")
                        .changed()
                    {
                        tracing_rotation::change_level("warn").unwrap();
                    }
                    if ui
                        .selectable_value(&mut self.log_level, LogLevel::Info, "Info")
                        .changed()
                    {
                        tracing_rotation::change_level("info").unwrap();
                    }
                    if ui
                        .selectable_value(&mut self.log_level, LogLevel::Debug, "Debug")
                        .changed()
                    {
                        tracing_rotation::change_level("debug").unwrap();
                    }
                    if ui
                        .selectable_value(&mut self.log_level, LogLevel::Trace, "Trace")
                        .changed()
                    {
                        tracing_rotation::change_level("trace").unwrap();
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

                    egui::CentralPanel::default().show(ctx, |ui| {
                        if ui.button("Clear").clicked() {
                            log_lines.lock().unwrap().clear();
                        }

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
        egui::CentralPanel::default().show(ctx, |ui| {
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
        let table_height = ui.available_height() * 0.8;

        egui::ScrollArea::vertical()
            .max_height(table_height)
            .show(ui, |ui| {
                egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .column(egui_extras::Column::auto().resizable(true)) // checkbox
                    .column(egui_extras::Column::initial(200.0).resizable(true)) // id
                    .column(egui_extras::Column::initial(200.0).resizable(true)) // name
                    .column(egui_extras::Column::initial(300.0).resizable(true)) // site
                    .column(egui_extras::Column::initial(100.0).resizable(true)) // priority
                    .header(20.0, |mut header| self.render_table_header(&mut header))
                    .body(|mut body| {
                        let mut widths = [0.0; 5];
                        widths.clone_from_slice(body.widths());

                        match editable {
                            true => dnd_table_body(body.ui_mut(), self.mod_list_mut(), widths),
                            false => check_only_table_body(
                                &mut body,
                                filtered_mods,
                                self.mod_list_mut(),
                                widths,
                            ),
                        }
                    });
            });
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
                _ if self.sort_column == column && self.sort_asc => format!("{} ▲", label),
                _ if self.sort_column == column && !self.sort_asc => format!("{} ▼", label),
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
    const fn mod_list(&self) -> &Vec<ModItem> {
        match self.mode {
            DataMode::Vfs => &self.vfs_mod_list,
            DataMode::Manual => &self.mod_list,
        }
    }

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
                    // NOTE: Unsupported Unix `get_skyrim_data_dir`
                    #[cfg(target_os = "windows")]
                    {
                        let err_msg = format!("Error: Reading skyrim data dir: {_err}");
                        self.set_notification(err_msg);
                    }
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

            let pattern = format!("{}/Nemesis_Engine/mod/*/info.ini", self.vfs_skyrim_data_dir);

            if self.is_first_render {
                self.update_vfs_mod_list(&pattern);
            }

            if self.vfs_mod_list.is_empty() || response.changed() {
                self.update_vfs_mod_list(&pattern);
            }
        } else {
            let response = ui.add_sized(
                [ui.available_width() * 0.9, 40.0],
                egui::TextEdit::singleline(&mut self.skyrim_data_dir)
                    .hint_text("D:\\GAME\\ModOrganizer Skyrim SE\\mods\\*"),
            );
            if self.mod_list().is_empty() || response.changed() {
                let pattern = format!("{}/Nemesis_Engine/mod/*/info.ini", self.skyrim_data_dir);
                self.update_mod_list(&pattern);
            }
        }
    }

    fn update_mod_list(&mut self, pattern: &str) {
        use mod_info::GetModsInfo as _;
        match mod_info::ModsInfo::get_all(pattern) {
            Ok(mods) => {
                // Turn the IDs of previously enabled mods into a HashSet
                let enabled_ids: std::collections::HashSet<&str> = self
                    .mod_list
                    .par_iter()
                    .filter(|m| m.enabled)
                    .map(|m| m.id.as_str())
                    .collect();

                // take over enabled for new mods
                let new_mods: Vec<_> = from_mod_infos(mods)
                    .into_par_iter()
                    .map(|mut m| {
                        if enabled_ids.contains(m.id.as_str()) {
                            m.enabled = true;
                        }
                        m
                    })
                    .collect();

                let _ = core::mem::replace(&mut self.mod_list, new_mods);
            }
            Err(err) => {
                let err_title = self.t(I18nKey::ErrorReadingModInfo);
                self.set_notification(format!("{err_title} {err}"));
            }
        }
    }

    /// # Note
    /// The only difference between vfs and manual is the id.
    /// For manual, due to the possibility of duplicates, the path up to the Nemesis ID (e.g., `aaaa`) becomes the id, but vfs uses the Nemesis ID directly.
    ///
    /// This allows vfs mode to maintain the check state on a different PC.
    fn update_vfs_mod_list(&mut self, pattern: &str) {
        if let Some(mods) = self.get_vfs_mod_list(pattern) {
            // Turn the IDs of previously enabled mods into a HashSet
            let enabled_ids: std::collections::HashSet<&str> = self
                .vfs_mod_list
                .par_iter()
                .filter(|m| m.enabled)
                .map(|m| m.id.as_str())
                .collect();

            // take over enabled for new mods
            let new_mods: Vec<_> = mods
                .into_par_iter()
                .map(|mut m| {
                    if enabled_ids.contains(m.id.as_str()) {
                        m.enabled = true;
                    }
                    m
                })
                .collect();

            let _ = core::mem::replace(&mut self.vfs_mod_list, new_mods);
        };
    }

    fn get_vfs_mod_list(&self, pattern: &str) -> Option<Vec<ModItem>> {
        use mod_info::GetModsInfo as _;
        match mod_info::ModsInfo::vfs_get_all(pattern) {
            Ok(mods) => Some(from_mod_infos(mods)),
            Err(err) => {
                let err_title = self.t(I18nKey::ErrorReadingModInfo);
                self.set_notification(format!("{err_title} {err}"));
                None
            }
        }
    }
}

impl ModManagerApp {
    /// Set message to notification
    pub fn set_notification<S: Into<String>>(&self, msg: S) {
        if let Ok(mut guard) = self.notification.lock() {
            *guard = msg.into();
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
        self.i18n
            .get(&key)
            .map_or_else(|| key.default_eng(), |s| s.as_ref())
    }
}

impl ModManagerApp {
    fn patch(&self) {
        // mod Items to nemesis_path
        let data_dir = match self.mode {
            DataMode::Vfs => &self.vfs_skyrim_data_dir,
            DataMode::Manual => &self.skyrim_data_dir,
        };

        let nemesis_paths = match self.mode {
            DataMode::Vfs => self
                .vfs_mod_list
                .par_iter()
                .filter(|item| item.enabled)
                .map(|item| {
                    let mut path = PathBuf::new();
                    path.push(data_dir);
                    path.push("Nemesis_Engine");
                    path.push("mod");
                    path.push(&item.id);
                    path
                })
                .collect(),
            DataMode::Manual => self
                .mod_list
                .par_iter()
                .filter(|item| item.enabled)
                .map(|item| {
                    let mut path = PathBuf::new();
                    path.push(&item.id);
                    path
                })
                .collect(),
        };
        let is_debug_mode = self.enable_debug_output;

        if self.auto_remove_meshes {
            let meshes_path = Path::new(&self.output_dir).join("meshes");
            let debug_path = Path::new(&self.output_dir).join(".d_merge").join(".debug");
            rayon::join(
                || {
                    let _ = remove_if_exists(meshes_path);
                },
                || {
                    let _ = remove_if_exists(debug_path);
                },
            );
        }

        let notify = Arc::clone(&self.notification);
        self.async_rt.spawn(nemesis_merge::behavior_gen(
            nemesis_paths,
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
            },
        ));
    }
}

/// Removes a directory if it exists, with debug logging.
///
/// # Why need this?
/// This is because the presence of a previous hkx may leave unintended changes behind.
///
/// # Reasons for not using `std::fs::remove_dir_all`
/// For some reason, egui on MO2 throws an error saying the path doesn't exist when I try to use `std::remove_dir_all`,
/// so I manually perform a recursive deletion starting from the end.
fn remove_if_exists(path: impl AsRef<Path>) -> std::io::Result<()> {
    use std::fs;

    let path = path.as_ref();

    if !path.exists() {
        return Ok(());
    }

    // Get the contents
    let entries: Vec<PathBuf> = fs::read_dir(path)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();

    // directories recursively deleted in parallel, files deleted in parallel
    entries.par_iter().for_each(|entry_path| {
        if entry_path.is_dir() {
            let _ = remove_if_exists(entry_path); // recursion
        } else {
            let _ = fs::remove_file(entry_path);
        }
    });

    // remove itself after deleting the contents
    fs::remove_dir(path)?;
    Ok(())
}
