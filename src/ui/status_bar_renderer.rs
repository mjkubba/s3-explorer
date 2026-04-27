use eframe::egui;

use crate::ui::app_state::AppState;

/// Renderer for the status bar of the application
pub struct StatusBarRenderer;

impl StatusBarRenderer {
    /// Render the status bar inside a ui
    pub fn render_bar(app_state: &mut AppState, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if app_state.status_is_error {
                ui.colored_label(egui::Color32::RED, &app_state.status_message);
            } else {
                ui.label(&app_state.status_message);
            }
        });
    }
}
