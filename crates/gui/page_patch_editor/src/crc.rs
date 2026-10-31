//! CRC32 tool: Skyrim's CRC32 of a text, and of every animation path in a Nemesis patch mod.
//!
//! ```text
//! Text / path:  [meshes\actors\character\animations\mymod ]  3692944883  0xdc1dddf3  [Copy]
//! ─────────────────────────────────────────────────────────────────────────────────
//! Nemesis mod:  [D:/mods/mymod                             ] [Select]      [Filter  ]
//! ▼ meshes\actors\character\animations\mymod                3692944883  0xdc1dddf3
//!       ground_bite.hkx                                      3191128947  0xbe34c373
//! ```
//!
//! # What is hashed (like Skyrim / Nemesis)
//! - a directory: its path from `meshes\`, lowercase, `\` separated
//! - a file: its lowercase stem (`ground_bite.hkx` -> `ground_bite`)
//!
//! ref: <https://www.nexusmods.com/skyrim/articles/50508/>

use std::path::{Path, PathBuf};

use egui_widgets::shadcn::{Button, TextEdit};
use gui_shared::i18n::{I18nKey, I18nMap};

/// CRC32 tool state.
#[derive(Debug, Default)]
pub(crate) struct CrcTool {
    text: String,
    mod_dir: String,
    filter: String,
    /// Directories with `.hkx` files under `mod_dir`, loaded for `loaded_for`.
    dirs: Vec<HkxDir>,
    loaded_for: Option<String>,
}

/// One directory with its `.hkx` files.
#[derive(Debug, Clone, PartialEq, Eq)]
struct HkxDir {
    /// e.g. `meshes\actors\character\animations\mymod`
    path: String,
    crc: u32,
    /// `(file name, crc of its lowercase stem)`
    files: Vec<(String, u32)>,
}

impl CrcTool {
    /// Draws the tool (fills the rest of `ui`).
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        self.handle_drop(ui.ctx());
        self.text_row(ui, i18n);
        ui.separator();
        self.mod_row(ui, i18n);
        ui.add_space(4.0);

        if self.loaded_for.as_deref() != Some(self.mod_dir.as_str()) {
            self.dirs = load_hkx_dirs(Path::new(&self.mod_dir));
            self.loaded_for = Some(self.mod_dir.clone());
        }
        if self.mod_dir.is_empty() {
            return;
        }
        if self.dirs.is_empty() {
            ui.weak(i18n.t(I18nKey::CrcNoHkx));
            return;
        }
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| self.tree(ui, i18n));
    }

    /// `Text / path: [..] 3692944883 0xdc1dddf3 [Copy]`
    fn text_row(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        ui.horizontal(|ui| {
            ui.label(i18n.t(I18nKey::CrcTextLabel)).on_hover_text(i18n.t(I18nKey::CrcTextHover));
            let crc = (!self.text.is_empty()).then(|| text_crc(&self.text));
            // Right-to-left: the result takes its width first, the input gets the rest.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(crc) = crc {
                    crc_cells(ui, crc, i18n);
                }
                ui.add(
                    TextEdit::singleline(&mut self.text)
                        .hint_text(i18n.t(I18nKey::CrcTextHint))
                        .desired_width(ui.available_width()),
                )
                .on_hover_text(i18n.t(I18nKey::CrcTextHover));
            });
        });
    }

    /// `Nemesis mod: [..] [Select]   [Filter]`
    fn mod_row(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        ui.horizontal(|ui| {
            ui.label(i18n.t(I18nKey::CrcModDirLabel));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    TextEdit::singleline(&mut self.filter)
                        .hint_text(i18n.t(I18nKey::CrcFilterHint))
                        .desired_width(160.0),
                );
                if ui.add(Button::new(i18n.t(I18nKey::SelectButton))).clicked()
                    && let Some(dir) =
                        rfd::FileDialog::new().set_directory(&self.mod_dir).pick_folder()
                {
                    self.mod_dir = dir.display().to_string();
                }
                ui.add(
                    TextEdit::singleline(&mut self.mod_dir)
                        .hint_text(i18n.t(I18nKey::CrcModDirHint))
                        .desired_width(ui.available_width()),
                );
            });
        });
    }

    /// One collapsible row per directory, its files inside.
    fn tree(&self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let filter = self.filter.to_lowercase();
        for dir in &self.dirs {
            let dir_matches = dir.path.to_lowercase().contains(&filter);
            let files: Vec<_> = dir
                .files
                .iter()
                .filter(|(name, _)| dir_matches || name.to_lowercase().contains(&filter))
                .collect();
            if files.is_empty() {
                continue;
            }

            let id = ui.make_persistent_id(&dir.path);
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
                .show_header(ui, |ui| row(ui, &dir.path, dir.crc, i18n, true))
                .body(|ui| {
                    for (name, crc) in files {
                        row(ui, name, *crc, i18n, false);
                    }
                });
        }
    }

    /// A dropped directory becomes the mod directory; anything else goes into the text box.
    fn handle_drop(&mut self, ctx: &egui::Context) {
        let dropped: Option<PathBuf> =
            ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()));
        let Some(path) = dropped else { return };
        if path.is_dir() {
            self.mod_dir = path.display().to_string();
        } else {
            self.text =
                path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        }
    }
}

/// `name ......... 3191128947  0xbe34c373  [Copy]`
fn row(ui: &mut egui::Ui, name: &str, crc: u32, i18n: &I18nMap, is_dir: bool) {
    ui.horizontal(|ui| {
        if is_dir {
            ui.strong(name);
        } else {
            ui.label(name);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            crc_cells(ui, crc, i18n);
        });
    });
}

/// `[Copy] 0xbe34c373 3191128947` (drawn right to left). Copies the decimal value.
fn crc_cells(ui: &mut egui::Ui, crc: u32, i18n: &I18nMap) {
    if ui.small_button(i18n.t(I18nKey::CrcCopy)).clicked() {
        ui.ctx().copy_text(crc.to_string());
    }
    ui.monospace(format!("{crc:#010x}"));
    ui.monospace(format!("{crc:>10}"));
}

/// CRC32 of what the user typed: lowercase, `/` -> `\`, a trailing `.hkx` removed.
fn text_crc(text: &str) -> u32 {
    let normalized = text.trim().to_lowercase().replace('/', "\\");
    let normalized = normalized.strip_suffix(".hkx").unwrap_or(&normalized);
    skyrim_crc::calc_crc32(normalized)
}

/// Every directory under `root` that has `.hkx` files (sorted by path).
fn load_hkx_dirs(root: &Path) -> Vec<HkxDir> {
    let mut dirs = Vec::new();
    if root.as_os_str().is_empty() || !root.is_dir() {
        return dirs;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut files = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("hkx"))
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                let name = entry.file_name().to_string_lossy().into_owned();
                files.push((name, skyrim_crc::calc_crc32(&stem.to_lowercase())));
            }
        }
        if !files.is_empty() {
            files.sort();
            let path = game_path(root, &dir);
            dirs.push(HkxDir { crc: skyrim_crc::calc_crc32(&path), path, files });
        }
    }
    dirs.sort_by(|a, b| a.path.cmp(&b.path));
    dirs
}

/// The path Skyrim hashes: from the `meshes` component on (else relative to `root`),
/// lowercase, `\` separated.
///
/// e.g. `D:/mods/mymod/meshes/actors/character/animations/mymod`
/// -> `meshes\actors\character\animations\mymod`
fn game_path(root: &Path, dir: &Path) -> String {
    let parts: Vec<String> =
        dir.components().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).collect();
    let from_meshes = parts.iter().rposition(|part| part == "meshes").map(|at| &parts[at..]);
    match from_meshes {
        Some(parts) => parts.join("\\"),
        None => dir
            .strip_prefix(root)
            .unwrap_or(dir)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
            .collect::<Vec<_>>()
            .join("\\"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_like_skyrim() {
        assert_eq!(text_crc("meshes/actors/dragon/animations"), 0xdc1d_ddf3);
        assert_eq!(text_crc("Ground_Bite.hkx"), 0xbe34_c373);

        let root = Path::new("D:/mods/mymod");
        let dir = Path::new("D:/mods/mymod/Meshes/Actors/Dragon/Animations");
        assert_eq!(game_path(root, dir), r"meshes\actors\dragon\animations");
    }
}
