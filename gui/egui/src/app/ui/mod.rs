//! App-owned UI panels.

pub(crate) mod bottom_panel;
pub(crate) mod mod_list;
pub(crate) mod top_panels;
pub(crate) mod window;

use d_merge_gui_shared::settings::ui::Theme;

fn themed_top_bottom_panel(
    panel: egui::TopBottomPanel,
    theme: Theme,
    transparent: bool,
) -> egui::TopBottomPanel {
    panel.frame(frame_from_theme(theme, transparent))
}

fn themed_central_panel(
    panel: egui::CentralPanel,
    theme: Theme,
    transparent: bool,
) -> egui::CentralPanel {
    panel.frame(frame_from_theme(theme, transparent))
}

fn frame_from_theme(theme: Theme, transparent: bool) -> egui::Frame {
    use egui::Frame;

    let base = match theme {
        Theme::Light => egui::Color32::WHITE,
        Theme::System | Theme::Dark => egui::Color32::from_rgb(30, 30, 30),
    };

    let fill = if transparent {
        egui::Color32::from_rgb(base.r(), base.g(), base.b()).gamma_multiply(0.6)
    } else {
        base
    };

    Frame::NONE.fill(fill)
}
