#![allow(clippy::unwrap_used, clippy::expect_used)]
use eframe::{egui, App, Frame};
use egui::{Align, Button, Checkbox};
use egui_extras::{Column, TableBuilder};
use mod_info::GetModsInfo as _;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use rayon::prelude::*;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread,
    time::Duration,
};

#[derive(Clone, PartialEq, Eq, Hash)]
struct ModItem {
    enabled: bool,
    path: String,
    name: String,
    site: String,
    priority: usize,
}

fn from_mod_infos(mod_infos: Vec<mod_info::ModInfo>) -> Vec<ModItem> {
    mod_infos
        .into_par_iter()
        .enumerate()
        .map(|(i, mi)| ModItem {
            enabled: true,
            path: mi.id,
            name: mi.name,
            site: mi.site,
            priority: i,
        })
        .collect()
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum SortColumn {
    Path,
    Name,
    Site,
    Priority,
}

struct ModManagerApp {
    skyrim_data_dir: String,
    output_dir: String,
    mod_list: Vec<ModItem>,
    filter_text: String,
    sort_column: SortColumn,
    sort_asc: bool,
    is_locked: bool,
    log_lines: Arc<Mutex<Vec<String>>>,
    notification: Option<String>,
    /// All checkbox enabled
    check_all: bool,
    show_log_window: Arc<AtomicBool>,
    log_watcher_started: bool,
}

impl Default for ModManagerApp {
    fn default() -> Self {
        Self {
            skyrim_data_dir: String::new(),
            output_dir: String::new(),
            mod_list: vec![],
            filter_text: String::new(),
            sort_column: SortColumn::Priority,
            sort_asc: true,
            is_locked: false,
            log_lines: Arc::new(Mutex::new(Vec::new())),
            notification: None,
            check_all: false, // 初期状態で全てのチェックを外す
            show_log_window: Arc::new(AtomicBool::new(false)),
            log_watcher_started: false,
        }
    }
}

impl ModManagerApp {
    fn toggle_sort(&mut self, column: SortColumn) {
        if self.sort_column == column {
            self.sort_asc = !self.sort_asc;
        } else {
            self.sort_column = column;
            self.sort_asc = true;
            self.filter_text.clear();
        }
    }

    fn header_button(&mut self, ui: &mut egui::Ui, text: &str, column: SortColumn) {
        let mut label = text.to_string();
        if self.sort_column == column {
            label.push_str(if self.sort_asc { " ▲" } else { " ▼" });
        }
        if ui.add(Button::selectable(false, label)).clicked() {
            self.toggle_sort(column);
        }
    }

    fn toggle_check_all(&mut self) {
        // 全てのenabledを切り替え
        for mod_item in &mut self.mod_list {
            mod_item.enabled = self.check_all;
        }
    }

    fn draw_skyrim_dir_ui(&mut self, ui: &mut egui::Ui) {
        if self.skyrim_data_dir.is_empty() {
            // 初期値を取得
            let dir = match skyrim_data_dir::get_skyrim_data_dir(skyrim_data_dir::Runtime::Se) {
                Ok(dir) => dir,
                Err(err) => {
                    self.notification = Some(format!("Error: reading mod info: {err}"));
                    return;
                }
            };

            let dir_str = dir.display().to_string();

            // 入力欄
            ui.add_sized(
                [ui.available_width() * 0.9, 40.0],
                egui::TextEdit::singleline(&mut self.skyrim_data_dir).hint_text(&dir_str),
            );

            // mods 情報取得
            if self.mod_list.is_empty() {
                self.load_mods(&format!("{dir_str}/Nemesis_Engine/mod/*/info.ini"));
            }
        } else {
            // 入力欄
            let response = ui.add_sized(
                [ui.available_width() * 0.9, 40.0],
                egui::TextEdit::singleline(&mut self.skyrim_data_dir),
            );
            if response.changed() || response.lost_focus() {
                let pattern = format!("{}/Nemesis_Engine/mod/*/info.ini", self.skyrim_data_dir);
                self.load_mods(&pattern);
            }
        }
    }

    fn load_mods(&mut self, pattern: &str) {
        match mod_info::ModsInfo::get_all(pattern) {
            Ok(mods) => {
                self.mod_list = from_mod_infos(mods);
            }
            Err(err) => {
                self.notification = Some(format!("Error: reading mod info: {err}"));
            }
        }
    }
}

impl App for ModManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // --- 上部検索 ---
        egui::TopBottomPanel::top("top_data_dir").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Skyrim Data dir:");

                self.draw_skyrim_dir_ui(ui);

                // dialog button
                if ui
                    .add_sized(
                        [ui.available_width() * 0.06, 40.0],
                        egui::Button::new("Open"),
                    )
                    .clicked()
                {
                    let dialog = if !self.skyrim_data_dir.is_empty() {
                        rfd::FileDialog::new().set_directory(&self.skyrim_data_dir)
                    } else {
                        rfd::FileDialog::new()
                    };

                    if let Some(dir) = dialog.pick_folder() {
                        self.load_mods(&dir.display().to_string());
                    }
                }
            });
        });

        egui::TopBottomPanel::top("top_output_dir").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Output Dir");
                let _response = ui.add_sized(
                    [ui.available_width() * 0.9, 40.0],
                    egui::TextEdit::singleline(&mut self.output_dir),
                );

                // dialog button
                if ui
                    .add_sized(
                        [ui.available_width() * 0.06, 40.0],
                        egui::Button::new("Open"),
                    )
                    .clicked()
                {
                    let dialog = if !self.output_dir.is_empty() {
                        rfd::FileDialog::new().set_directory(&self.output_dir)
                    } else {
                        rfd::FileDialog::new()
                    };

                    if let Some(dir) = dialog.pick_folder() {
                        self.output_dir = dir.display().to_string();
                    }
                }
            });
        });

        // --- 上部検索 ---
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // 検索ラベルと検索欄
                ui.label("検索:");
                ui.add_sized([300.0, 40.0], egui::TextEdit::singleline(&mut self.filter_text));

                // 検索クリアボタン
                if ui
                    .add_sized([60.0, 40.0], egui::Button::new("クリア"))
                    .clicked()
                {
                    self.filter_text.clear();
                }

                // ロックボタンを右端に配置
                if self.is_locked {
                    ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0); // アイテム間のスペースを調整
                    let lock_btn = ui.add_sized([60.0, 40.0], Button::new("🔒"))
                        .on_hover_text("優先度(昇順)以外での行移動はロックされます。\n解除するにはこれをクリック。");
                    if lock_btn.clicked() {
                        self.sort_asc = true;
                        self.sort_column = SortColumn::Priority;
                    }
                }
            });
        });

        // --- 中央MODリスト ---
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("MOD 一覧");
            ui.separator();

            // フィルタ処理
            let mut filtered: Vec<ModItem> = self
                .mod_list
                .iter()
                .filter(|&m| {
                    self.filter_text.trim().is_empty()
                        || m.path
                            .to_lowercase()
                            .contains(&self.filter_text.to_lowercase())
                        || m.name
                            .to_lowercase()
                            .contains(&self.filter_text.to_lowercase())
                        || m.site
                            .to_lowercase()
                            .contains(&self.filter_text.to_lowercase())
                })
                .cloned()
                .collect();

            // ソート
            filtered.sort_by(|a, b| {
                let ord = match self.sort_column {
                    SortColumn::Path => a.path.cmp(&b.path),
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

            let dnd_allowed = self.filter_text.trim().is_empty()
                && self.sort_column == SortColumn::Priority
                && self.sort_asc;
            self.is_locked = !dnd_allowed;

            // D&D
            let available_width = ui.available_width();
            let w_checkbox = available_width * 0.07;
            let w_path = available_width * 0.25;
            let w_name = available_width * 0.18;
            let w_site = available_width * 0.30;
            let w_priority = available_width * 0.09;

            let _response = TableBuilder::new(ui)
                .resizable(true)
                .columns(Column::exact(w_checkbox), 1)
                .columns(Column::exact(w_path), 1)
                .columns(Column::exact(w_name), 1)
                .columns(Column::exact(w_site), 1)
                .columns(Column::exact(w_priority), 1)
                .header(25.0, |mut row| {
                    row.col(|ui| {
                        // "Check All" チェックボックスを追加
                        if ui
                            .add(Checkbox::without_text(&mut self.check_all))
                            .clicked()
                        {
                            self.toggle_check_all();
                        }
                    });
                    row.col(|ui| {
                        self.header_button(ui, "パス", SortColumn::Path);
                    });
                    row.col(|ui| {
                        self.header_button(ui, "MOD名", SortColumn::Name);
                    });
                    row.col(|ui| {
                        self.header_button(ui, "サイト", SortColumn::Site);
                    });
                    row.col(|ui| {
                        self.header_button(ui, "優先度", SortColumn::Priority);
                    });
                })
                .body(|mut body| {
                    if dnd_allowed {
                        let widths = body.widths().to_vec();
                        let response = dnd_ui(body.ui_mut(), &mut self.mod_list, &widths);

                        // 優先度を並び順にリセット
                        if response.final_update().is_some() {
                            self.mod_list
                                .par_iter_mut()
                                .enumerate()
                                .for_each(|(i, item)| {
                                    item.priority = i + 1;
                                });
                        }
                    } else {
                        // D&D 無効、filtered で表示のみ
                        for item in &mut filtered {
                            body.row(20.0, |mut row| {
                                row.col(|ui| {
                                    ui.add(Checkbox::without_text(&mut item.enabled));
                                });
                                row.col(|ui| {
                                    label_with_hover(ui, &item.path, w_path);
                                });
                                row.col(|ui| {
                                    label_with_hover(ui, &item.name, w_name);
                                });
                                row.col(|ui| {
                                    hyperlink_with_hover(ui, &item.site, w_site);
                                });
                                row.col(|ui| {
                                    ui.label(item.priority.to_string());
                                });
                            });
                        }
                    }
                });

            ui.add_space(10.0);
        });

        // --- 通知欄 ---
        egui::TopBottomPanel::bottom("notification_panel")
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(self.notification.clone().unwrap_or_default());
            });

        // --- 下部ボタン ---
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add_sized([120.0, 40.0], egui::Button::new("Log Dir"))
                        .clicked()
                    {
                        if let Err(err) = open::that_detached("logs") {
                            self.notification = Some(err.to_string());
                        }
                    }
                    if ui
                        .add_sized([120.0, 40.0], egui::Button::new("Log"))
                        .clicked()
                    {
                        // 外部ウィンドウでLogViewer起動
                        self.show_log_window
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                        if !self.log_watcher_started {
                            let log_lines = Arc::clone(&self.log_lines);
                            start_log_watcher(log_lines);
                            self.log_watcher_started = true; // 二重起動防止
                        }
                    }
                    if ui
                        .add_sized([120.0, 40.0], egui::Button::new("Patch"))
                        .clicked()
                    {
                        // TODO: need tokio rt in App
                        // if let Err(err) = nemesis_merge::behavior_gen(ids, config) {
                        //     self.notification = Some(err.to_string());
                        // };
                        self.notification = Some("Patch 処理開始 (未実装)".into());
                    }
                });
            });

        if self
            .show_log_window
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            let show_log_window = Arc::clone(&self.show_log_window);
            let log_lines = Arc::clone(&self.log_lines);

            ctx.show_viewport_deferred(
                egui::ViewportId::from_hash_of("deferred_viewport"),
                egui::ViewportBuilder::default()
                    .with_title("Log viewer")
                    .with_min_inner_size([800.0, 500.0]),
                move |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Deferred,
                        "This egui backend doesn't support multiple viewports"
                    );

                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::ScrollArea::vertical()
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                tracing::info!("Hello from deferred viewport");

                                for line in log_lines.lock().unwrap().iter() {
                                    ui.label(line);
                                }

                                tracing::info!("Hello from deferred viewport");
                            });
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        // Tell parent to close us.
                        show_log_window.store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                },
            );
        }
    }
}

fn dnd_ui(ui: &mut egui::Ui, items: &mut [ModItem], widths: &[f32]) -> egui_dnd::DragDropResponse {
    egui_dnd::dnd(ui, "mod_list_dnd").show_vec(items, |ui, item, handle, _state| {
        ui.horizontal(|ui| {
            ui.add_sized([widths[0], 30.0], Checkbox::without_text(&mut item.enabled));

            handle.ui(ui, |ui| {
                label_with_hover(ui, &item.path, widths[1]);
            });
            label_with_hover(ui, &item.name, widths[2]);
            hyperlink_with_hover(ui, &item.site, widths[3]);
            ui.label(item.priority.to_string());
        });
    })
}

// --- ユーティリティ ---
fn label_with_hover(ui: &mut egui::Ui, text: &str, width: f32) {
    let truncated = truncate_to_width(ui, text, width);
    let display = if truncated.is_empty() {
        " ".repeat((width / 6.0).max(3.0) as usize)
    } else {
        truncated
    };
    ui.add_sized([width, 20.0], egui::Label::new(display).halign(Align::LEFT))
        .on_hover_text(text);
}

fn hyperlink_with_hover(ui: &mut egui::Ui, url: &str, width: f32) {
    let truncated = truncate_to_width(ui, url, width);
    ui.add_sized(
        [width, 20.0],
        egui::Hyperlink::from_label_and_url(truncated, url),
    )
    .on_hover_text(url);
}

fn truncate_to_width(ui: &egui::Ui, text: &str, width: f32) -> String {
    let fonts = ui.fonts(|f| f.clone());
    let galley = fonts.layout_no_wrap(
        text.to_string(),
        egui::TextStyle::Body.resolve(ui.style()),
        ui.style().visuals.text_color(),
    );

    if galley.size().x <= width {
        return text.to_string();
    }

    let mut truncated = String::new();
    for ch in text.chars() {
        let test = format!("{truncated}{ch}...");
        let galley = fonts.layout_no_wrap(
            test.clone(),
            egui::TextStyle::Body.resolve(ui.style()),
            ui.style().visuals.text_color(),
        );
        if galley.size().x > width {
            truncated.push_str("...");
            break;
        }
        truncated.push(ch);
    }
    truncated
}

// --- ログ監視 ---
fn start_log_watcher(log_lines: Arc<Mutex<Vec<String>>>) {
    let log_path = Path::new("./logs/d_merge.log");
    thread::spawn(move || {
        let file = File::open(log_path).unwrap_or_else(|_| File::create(log_path).unwrap());
        let reader = BufReader::new(file);

        for line in reader.lines().map_while(Result::ok) {
            log_lines.lock().unwrap().push(line);
        }

        let (tx, rx) = std::sync::mpsc::channel();

        let config = Config::default().with_poll_interval(Duration::from_secs(1));
        let mut watcher: RecommendedWatcher =
            Watcher::new(tx, config).expect("Watcher create failed");
        watcher
            .watch(log_path, RecursiveMode::NonRecursive)
            .expect("Watch failed");

        loop {
            if rx.recv().is_ok() {
                if let Ok(file) = File::open(log_path) {
                    let reader = BufReader::new(file);
                    let lines: Vec<_> = reader.lines().map_while(Result::ok).collect();
                    *log_lines.lock().unwrap() = lines;
                }
            }
        }
    });
}

// --- フォント ---
fn setup_custom_fonts(ctx: &egui::Context) {
    use egui::{FontData, FontDefinitions};
    let mut fonts = FontDefinitions::default();

    if let Ok(font) = std::fs::read("c:/Windows/Fonts/msyh.ttc") {
        fonts
            .font_data
            .insert("sys_font".to_owned(), FontData::from_owned(font).into());

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "sys_font".to_owned());

        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "sys_font".to_owned());
    }

    ctx.set_fonts(fonts);
}

fn main() -> Result<(), eframe::Error> {
    let _ = tracing_rotation::init("./logs", "d_merge.log");

    tracing_rotation::change_level("info").unwrap();

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "D Merge",
        options,
        Box::new(|cc| {
            setup_custom_fonts(&cc.egui_ctx);
            Ok(Box::new(ModManagerApp::default()))
        }),
    )
}
