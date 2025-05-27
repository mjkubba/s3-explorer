use eframe::egui;

use crate::ui::app_state::AppState;

/// Renderer for the status bar of the application
pub struct StatusBarRenderer;

impl StatusBarRenderer {
    /// Render the status bar
    pub fn render(app_state: &mut AppState, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if app_state.status_is_error {
                    ui.colored_label(egui::Color32::RED, &app_state.status_message);
                } else {
                    ui.label(&app_state.status_message);
                }
            });
        });
    }
}
