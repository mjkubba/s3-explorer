use eframe::egui;

use crate::ui::app_state::{AppState, CurrentView};
use crate::ui::aws_operations::AwsOperations;

/// Renderer for the menu bar of the application
pub struct MenuBarRenderer;

impl MenuBarRenderer {
    /// Render the menu bar
    pub fn render(app_state: &mut AppState, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Settings").clicked() {
                        app_state.current_view = CurrentView::Settings;
                        ui.close_menu();
                    }
                    
                    if ui.button("Exit").clicked() {
                        std::process::exit(0);
                    }
                });
                
                ui.menu_button("View", |ui| {
                    if ui.button("Filters").clicked() {
                        app_state.current_view = CurrentView::Filter;
                        ui.close_menu();
                    }
                    
                    if ui.button("Refresh").clicked() {
                        AwsOperations::refresh_buckets(app_state);
                        ui.close_menu();
                    }
                });
                
                ui.menu_button("Actions", |ui| {
                    if ui.button("Upload").clicked() {
                        AwsOperations::upload_selected(app_state);
                        ui.close_menu();
                    }
                    
                    if ui.button("Download").clicked() {
                        AwsOperations::download_selected(app_state);
                        ui.close_menu();
                    }
                    
                    if ui.button("Sync").clicked() {
                        AwsOperations::sync_selected(app_state);
                        ui.close_menu();
                    }
                });
                
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        // Show about dialog
                        ui.close_menu();
                    }
                });
            });
        });
    }
}
