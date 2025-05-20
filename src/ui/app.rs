use eframe::egui;
use log::info;

use super::bucket_view::BucketView;
use super::folder_list::FolderList;
use super::settings::SettingsView;

/// Main application state
pub struct S3SyncApp {
    folder_list: FolderList,
    bucket_view: BucketView,
    settings_view: SettingsView,
    current_view: CurrentView,
}

/// Enum to track which view is currently active
enum CurrentView {
    Main,
    Settings,
}

impl S3SyncApp {
    /// Create a new instance of the application
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set up custom fonts if needed
        // cc.egui_ctx.set_fonts(...);
        
        info!("Initializing S3Sync application");
        
        Self {
            folder_list: FolderList::default(),
            bucket_view: BucketView::default(),
            settings_view: SettingsView::default(),
            current_view: CurrentView::Main,
        }
    }
}

impl eframe::App for S3SyncApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.current_view {
            CurrentView::Main => self.render_main_view(ctx),
            CurrentView::Settings => self.render_settings_view(ctx),
        }
    }
}

impl S3SyncApp {
    /// Render the main application view with folder list and bucket view
    fn render_main_view(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Settings").clicked() {
                        self.current_view = CurrentView::Settings;
                        ui.close_menu();
                    }
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                
                ui.menu_button("Sync", |ui| {
                    if ui.button("Sync All").clicked() {
                        // TODO: Implement sync all functionality
                        ui.close_menu();
                    }
                    if ui.button("Stop All").clicked() {
                        // TODO: Implement stop all functionality
                        ui.close_menu();
                    }
                });
                
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        // TODO: Show about dialog
                        ui.close_menu();
                    }
                });
            });
        });
        
        egui::SidePanel::left("folder_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                self.folder_list.ui(ui);
            });
            
        egui::CentralPanel::default().show(ctx, |ui| {
            self.bucket_view.ui(ui);
        });
        
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Status: Ready");
            });
        });
    }
    
    /// Render the settings view
    fn render_settings_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Back to Main").clicked() {
                self.current_view = CurrentView::Main;
            }
            
            self.settings_view.ui(ui);
        });
    }
}
