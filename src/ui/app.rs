use eframe::egui;
use eframe::epi;
use log::info;

use super::bucket_view::BucketView;
use super::folder_list::FolderList;
use super::settings::SettingsView;
use super::progress::ProgressView;
use super::filter_view::FilterView;

/// Main application state
pub struct S3SyncApp {
    folder_list: FolderList,
    bucket_view: BucketView,
    settings_view: SettingsView,
    progress_view: ProgressView,
    filter_view: Option<FilterView>,
    current_view: CurrentView,
    show_progress: bool,
}

/// Enum to track which view is currently active
enum CurrentView {
    Main,
    Settings,
    Progress,
    Filters,
}

impl Default for S3SyncApp {
    fn default() -> Self {
        info!("Initializing S3Sync application");
        
        Self {
            folder_list: FolderList::default(),
            bucket_view: BucketView::default(),
            settings_view: SettingsView::default(),
            progress_view: ProgressView::default(),
            filter_view: None,
            current_view: CurrentView::Main,
            show_progress: false,
        }
    }
}

impl epi::App for S3SyncApp {
    fn name(&self) -> &str {
        "S3Sync"
    }
    
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        match self.current_view {
            CurrentView::Main => self.render_main_view(ctx),
            CurrentView::Settings => self.render_settings_view(ctx),
            CurrentView::Progress => self.render_progress_view(ctx),
            CurrentView::Filters => self.render_filters_view(ctx),
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
                        std::process::exit(0);
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
                    
                    if ui.button("Filters").clicked() {
                        self.current_view = CurrentView::Filters;
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
                
                // Add a button to show/hide progress
                if ui.button(if self.show_progress { "Hide Progress" } else { "Show Progress" }).clicked() {
                    self.show_progress = !self.show_progress;
                }
                
                // Add a button to show filters
                if ui.button("Filters").clicked() {
                    self.current_view = CurrentView::Filters;
                }
            });
        });
        
        // Show progress panel if needed
        if self.show_progress {
            egui::Window::new("Progress")
                .collapsible(true)
                .resizable(true)
                .default_size([400.0, 300.0])
                .show(ctx, |ui| {
                    self.progress_view.ui(ui);
                    
                    if ui.button("Close").clicked() {
                        self.show_progress = false;
                    }
                });
        }
    }
    
    /// Render the progress view
    fn render_progress_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Back to Main").clicked() {
                self.current_view = CurrentView::Main;
            }
            
            self.progress_view.ui(ui);
        });
    }
    
    /// Render the filters view
    fn render_filters_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Back to Main").clicked() {
                self.current_view = CurrentView::Main;
            }
            
            if let Some(filter_view) = &mut self.filter_view {
                filter_view.ui(ui);
            } else {
                ui.label("Filter view not initialized");
            }
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
