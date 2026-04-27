use eframe::egui;

use crate::ui::app_state::{AppState, CurrentView};
use crate::ui::aws_operations::AwsOperations;

/// Renderer for the menu bar of the application
pub struct MenuBarRenderer;

impl MenuBarRenderer {
    /// Render the menu bar inside a ui
    pub fn render_bar(app_state: &mut AppState, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Settings").clicked() {
                    app_state.current_view = CurrentView::Settings;
                    ui.close();
                }
                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            
            ui.menu_button("View", |ui| {
                if ui.button("Filters").clicked() {
                    app_state.current_view = CurrentView::Filter;
                    ui.close();
                }
                if ui.button("Refresh").clicked() {
                    AwsOperations::refresh_buckets(app_state);
                    ui.close();
                }
            });
            
            ui.menu_button("Actions", |ui| {
                if ui.button("Upload").clicked() {
                    AwsOperations::upload_selected(app_state);
                    ui.close();
                }
                if ui.button("Download").clicked() {
                    AwsOperations::download_selected(app_state);
                    ui.close();
                }
                if ui.button("Sync").clicked() {
                    AwsOperations::sync_selected(app_state);
                    ui.close();
                }
            });
            
            ui.menu_button("Help", |ui| {
                if ui.button("About").clicked() {
                    ui.close();
                }
            });
        });
    }
}
