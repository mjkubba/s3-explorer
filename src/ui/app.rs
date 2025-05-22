use eframe::egui;
use eframe::epi;
use log::{info, error};
use std::sync::{Arc, Mutex};
use tokio::runtime::Handle;

use super::bucket_view::BucketView;
use super::folder_list::FolderList;
use super::settings::SettingsView;
use super::progress::ProgressView;
use super::filter_view::FilterView;
use crate::aws::auth::AwsAuth;
use crate::config::credentials::CredentialManager;

/// Main application state
pub struct S3SyncApp {
    folder_list: FolderList,
    bucket_view: BucketView,
    settings_view: SettingsView,
    progress_view: ProgressView,
    filter_view: Option<FilterView>,
    current_view: CurrentView,
    show_progress: bool,
    aws_auth: Arc<Mutex<AwsAuth>>,
    status_message: Option<(String, egui::Color32)>,
    rt: Arc<Handle>,
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
        
        // Try to load credentials from keyring
        let mut aws_auth = AwsAuth::default();
        let has_credentials = Self::load_credentials_from_keyring(&mut aws_auth);
        
        if has_credentials {
            info!("Loaded AWS credentials from keyring");
        } else {
            info!("No AWS credentials found in keyring");
        }
        
        let aws_auth = Arc::new(Mutex::new(aws_auth));
        
        Self {
            folder_list: FolderList::default(),
            bucket_view: BucketView::default(),
            settings_view: SettingsView::default(),
            progress_view: ProgressView::default(),
            filter_view: None,
            current_view: CurrentView::Main,
            show_progress: false,
            aws_auth,
            status_message: None,
            rt: Arc::new(Handle::current()),
        }
    }
}

impl S3SyncApp {
    /// Load AWS credentials from the system keyring
    fn load_credentials_from_keyring(auth: &mut AwsAuth) -> bool {
        match (CredentialManager::load_access_key(), CredentialManager::load_secret_key()) {
            (Ok(access_key), Ok(secret_key)) if !access_key.is_empty() && !secret_key.is_empty() => {
                auth.update_credentials(access_key, secret_key, auth.region().to_string());
                true
            },
            _ => false,
        }
    }
    
    /// Save AWS credentials to the system keyring
    fn save_credentials_to_keyring(&self, access_key: &str, secret_key: &str) -> bool {
        match CredentialManager::save_credentials(access_key, secret_key) {
            Ok(_) => true,
            Err(e) => {
                error!("Failed to save credentials to keyring: {}", e);
                false
            }
        }
    }
    
    /// Load buckets from AWS
    fn load_buckets(&mut self) {
        let auth_clone = self.aws_auth.clone();
        let rt = self.rt.clone();
        
        // Set loading state
        self.bucket_view.set_loading(true);
        self.set_status_info("Loading buckets...");
        
        // Spawn a task to load buckets
        let ctx = egui::Context::default();
        
        rt.spawn(async move {
            // Create a new BucketView just for this operation
            let mut bucket_view = BucketView::default();
            let result = bucket_view.load_buckets(auth_clone).await;
            
            // Request a repaint to update the UI
            ctx.request_repaint();
            
            // Return the result
            result
        });
    }
    
    /// Set an informational status message
    fn set_status_info(&mut self, message: &str) {
        self.status_message = Some((message.to_string(), egui::Color32::from_rgb(0, 128, 255)));
    }
    
    /// Set an error status message
    fn set_status_error(&mut self, message: &str) {
        self.status_message = Some((message.to_string(), egui::Color32::RED));
    }
    
    /// Set a success status message
    fn set_status_success(&mut self, message: &str) {
        self.status_message = Some((message.to_string(), egui::Color32::GREEN));
    }
    
    /// Clear the status message
    fn clear_status(&mut self) {
        self.status_message = None;
    }
    
    /// Render the main view
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
                
                ui.menu_button("View", |ui| {
                    if ui.button("Filters").clicked() {
                        self.current_view = CurrentView::Filters;
                        ui.close_menu();
                    }
                    
                    if ui.button("Progress").clicked() {
                        self.current_view = CurrentView::Progress;
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
                if let Some((message, color)) = &self.status_message {
                    ui.colored_label(*color, message);
                } else {
                    ui.label("Status: Ready");
                }
                
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
            
            // Render settings UI
            self.settings_view.ui(ui);
            
            // Add a save credentials button
            if ui.button("Save AWS Credentials").clicked() {
                let access_key = self.settings_view.aws_access_key();
                let secret_key = self.settings_view.aws_secret_key();
                let region = self.settings_view.aws_region();
                
                if access_key.is_empty() || secret_key.is_empty() {
                    self.set_status_error("Access key and secret key cannot be empty");
                } else {
                    // Update the auth object
                    {
                        let mut auth = self.aws_auth.lock().unwrap();
                        auth.update_credentials(access_key.clone(), secret_key.clone(), region.clone());
                    }
                    
                    // Save to keyring
                    if self.save_credentials_to_keyring(&access_key, &secret_key) {
                        self.set_status_success("AWS credentials saved successfully");
                        
                        // Test the credentials and load buckets
                        self.test_credentials_and_load_buckets();
                    } else {
                        self.set_status_error("Failed to save AWS credentials to keyring");
                    }
                }
            }
            
            // Add a test credentials button
            if ui.button("Test AWS Credentials").clicked() {
                self.test_credentials_and_load_buckets();
            }
        });
    }
    
    /// Test AWS credentials and load buckets if successful
    fn test_credentials_and_load_buckets(&mut self) {
        let auth_clone = self.aws_auth.clone();
        let rt = self.rt.clone();
        
        // Set loading state
        self.set_status_info("Testing AWS credentials...");
        
        // Spawn a task to test credentials
        let ctx = egui::Context::default();
        
        rt.spawn(async move {
            // Clone the auth to avoid holding the lock across await points
            let mut auth_clone_inner = {
                let auth_guard = auth_clone.lock().unwrap();
                auth_guard.clone()
            };
            
            // Now use the cloned auth object
            let result = auth_clone_inner.test_credentials().await;
            
            match result {
                Ok(_) => {
                    info!("AWS credentials validated successfully");
                    // TODO: Update status message to success
                    // TODO: Load buckets
                },
                Err(err) => {
                    error!("AWS credential validation failed: {}", err);
                    // TODO: Update status message to error
                }
            }
            
            // Request a repaint to update the UI
            ctx.request_repaint();
        });
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
    
    fn setup(&mut self, _ctx: &egui::Context, _frame: &epi::Frame, _storage: Option<&dyn epi::Storage>) {
        // Load buckets on startup if we have credentials
        let auth = self.aws_auth.lock().unwrap();
        if !auth.is_empty() {
            drop(auth); // Release the lock before async operation
            self.load_buckets();
        }
    }
}
