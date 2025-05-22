use eframe::egui;
use eframe::epi;
use log::{info, error, debug};
use std::sync::{Arc, Mutex};
use tokio::runtime::Handle;
use std::sync::mpsc;

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
    status_tx: mpsc::Sender<StatusMessage>,
    status_rx: mpsc::Receiver<StatusMessage>,
    pending_bucket_selection: Option<String>,
}

/// Status message for communication between threads
enum StatusMessage {
    Info(String),
    Success(String),
    Error(String),
    BucketList(Vec<String>),
    ObjectList(Vec<super::bucket_view::S3Object>),
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
        
        // Create a channel for status messages
        let (status_tx, status_rx) = mpsc::channel();
        
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
            status_tx,
            status_rx,
            pending_bucket_selection: None,
        }
    }
}

impl S3SyncApp {
    /// Load AWS credentials from the system keyring
    fn load_credentials_from_keyring(auth: &mut AwsAuth) -> bool {
        debug!("Attempting to load credentials from keyring");
        match (CredentialManager::load_access_key(), CredentialManager::load_secret_key()) {
            (Ok(access_key), Ok(secret_key)) if !access_key.is_empty() && !secret_key.is_empty() => {
                debug!("Found credentials in keyring");
                auth.update_credentials(access_key, secret_key, auth.region().to_string());
                true
            },
            (Ok(access_key), Ok(_)) if access_key.is_empty() => {
                debug!("No access key found in keyring");
                false
            },
            (Ok(_), Ok(secret_key)) if secret_key.is_empty() => {
                debug!("No secret key found in keyring");
                false
            },
            (Err(e), _) => {
                debug!("Error loading access key from keyring: {}", e);
                false
            },
            (_, Err(e)) => {
                debug!("Error loading secret key from keyring: {}", e);
                false
            },
            _ => {
                debug!("No credentials found in keyring");
                false
            }
        }
    }
    
    /// Save AWS credentials to the system keyring
    fn save_credentials_to_keyring(&self, access_key: &str, secret_key: &str) -> bool {
        debug!("Saving credentials to keyring");
        match CredentialManager::save_credentials(access_key, secret_key) {
            Ok(_) => {
                debug!("Successfully saved credentials to keyring");
                true
            },
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
        let tx = self.status_tx.clone();
        
        // Set loading state
        self.bucket_view.set_loading(true);
        self.set_status_info("Loading buckets...");
        
        // Spawn a task to load buckets
        rt.spawn(async move {
            debug!("Starting async task to load buckets");
            // Create a new BucketView just for this operation
            let mut bucket_view = BucketView::default();
            match bucket_view.load_buckets(auth_clone).await {
                Ok(buckets) => {
                    debug!("Successfully loaded {} buckets", buckets.len());
                    let _ = tx.send(StatusMessage::Success(format!("Loaded {} buckets", buckets.len())));
                    let _ = tx.send(StatusMessage::BucketList(buckets));
                },
                Err(e) => {
                    error!("Failed to load buckets: {}", e);
                    let _ = tx.send(StatusMessage::Error(format!("Failed to load buckets: {}", e)));
                }
            }
        });
    }
    
    /// Load objects from a bucket
    fn load_objects(&mut self, bucket_name: &str) {
        let auth_clone = self.aws_auth.clone();
        let rt = self.rt.clone();
        let tx = self.status_tx.clone();
        let bucket = bucket_name.to_string();
        
        // Set loading state
        self.bucket_view.set_loading(true);
        self.set_status_info(&format!("Loading objects from {}...", bucket_name));
        
        // Spawn a task to load objects
        rt.spawn(async move {
            debug!("Starting async task to load objects from bucket: {}", bucket);
            // Create a new BucketView just for this operation
            let mut bucket_view = BucketView::default();
            match bucket_view.load_objects(auth_clone, &bucket).await {
                Ok(objects) => {
                    debug!("Successfully loaded {} objects from bucket {}", objects.len(), bucket);
                    let _ = tx.send(StatusMessage::Success(format!("Loaded {} objects from {}", objects.len(), bucket)));
                    let _ = tx.send(StatusMessage::ObjectList(objects));
                },
                Err(e) => {
                    error!("Failed to load objects from bucket {}: {}", bucket, e);
                    let _ = tx.send(StatusMessage::Error(format!("Failed to load objects from {}: {}", bucket, e)));
                }
            }
        });
    }
    
    /// Process any pending status messages
    fn process_status_messages(&mut self) {
        while let Ok(msg) = self.status_rx.try_recv() {
            match msg {
                StatusMessage::Info(text) => self.set_status_info(&text),
                StatusMessage::Success(text) => self.set_status_success(&text),
                StatusMessage::Error(text) => self.set_status_error(&text),
                StatusMessage::BucketList(buckets) => self.bucket_view.set_buckets(buckets),
                StatusMessage::ObjectList(objects) => self.bucket_view.set_objects(objects),
            }
        }
        
        // Clear loading state if there are no more messages
        if self.bucket_view.is_loading() && self.status_rx.try_recv().is_err() {
            self.bucket_view.set_loading(false);
        }
        
        // Process any pending bucket selection
        if let Some(bucket) = self.pending_bucket_selection.take() {
            *self.bucket_view.selected_bucket_mut() = Some(bucket.clone());
            self.load_objects(&bucket);
        }
    }
    
    /// Set an informational status message
    fn set_status_info(&mut self, message: &str) {
        debug!("Status info: {}", message);
        self.status_message = Some((message.to_string(), egui::Color32::from_rgb(0, 128, 255)));
    }
    
    /// Set an error status message
    fn set_status_error(&mut self, message: &str) {
        error!("Status error: {}", message);
        self.status_message = Some((message.to_string(), egui::Color32::RED));
    }
    
    /// Set a success status message
    fn set_status_success(&mut self, message: &str) {
        info!("Status success: {}", message);
        self.status_message = Some((message.to_string(), egui::Color32::GREEN));
    }
    
    /// Clear the status message
    fn clear_status(&mut self) {
        debug!("Clearing status message");
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
            // Render the bucket view
            ui.vertical(|ui| {
                ui.heading("S3 Buckets");
                
                // Show loading indicator or error message
                if self.bucket_view.is_loading() {
                    ui.horizontal(|ui| {
                        ui.label("⏳ Loading...");
                    });
                } else if let Some(error) = self.bucket_view.error_message() {
                    ui.colored_label(egui::Color32::RED, error);
                    if ui.button("Clear Error").clicked() {
                        self.bucket_view.clear_error();
                    }
                }
                
                // Bucket selection
                ui.horizontal(|ui| {
                    let mut selected_bucket = self.bucket_view.selected_bucket();
                    let buckets = self.bucket_view.buckets().to_vec(); // Clone to avoid borrow issues
                    
                    egui::ComboBox::from_label("Select Bucket")
                        .selected_text(selected_bucket.as_deref().unwrap_or("No bucket selected"))
                        .show_ui(ui, |ui| {
                            for bucket in &buckets {
                                let bucket_str = bucket.clone();
                                if ui.selectable_label(selected_bucket.as_deref() == Some(bucket), bucket).clicked() {
                                    // Store the selection for processing after the UI rendering
                                    self.pending_bucket_selection = Some(bucket_str);
                                }
                            }
                        });
                        
                    if ui.button("Refresh Buckets").clicked() {
                        self.load_buckets();
                    }
                    
                    if let Some(bucket) = self.bucket_view.selected_bucket() {
                        if ui.button("Load Objects").clicked() {
                            self.load_objects(&bucket);
                        }
                    }
                });
                
                ui.separator();
                
                // Filter
                ui.horizontal(|ui| {
                    ui.label("Filter:");
                    ui.text_edit_singleline(self.bucket_view.filter_mut());
                    
                    if ui.button("Clear").clicked() {
                        self.bucket_view.clear_filter();
                    }
                });
                
                ui.separator();
                
                // Object list
                if self.bucket_view.selected_bucket().is_some() {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // Table header
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Name").strong());
                            ui.add_space(200.0);
                            ui.label(egui::RichText::new("Size").strong());
                            ui.add_space(100.0);
                            ui.label(egui::RichText::new("Last Modified").strong());
                        });
                        
                        ui.separator();
                        
                        // Table rows
                        let filter = self.bucket_view.filter().to_lowercase();
                        let objects = self.bucket_view.objects().to_vec(); // Clone to avoid borrow issues
                        
                        if objects.is_empty() {
                            ui.label("No objects found in this bucket");
                        } else {
                            for object in &objects {
                                if !filter.is_empty() && !object.key.to_lowercase().contains(&filter) {
                                    continue;
                                }
                                
                                ui.horizontal(|ui| {
                                    let icon = if object.is_directory { "📁 " } else { "📄 " };
                                    ui.label(format!("{}{}", icon, object.key));
                                    ui.add_space(200.0 - object.key.len() as f32 * 7.0);
                                    
                                    let size_str = if object.is_directory {
                                        "-".to_string()
                                    } else {
                                        super::bucket_view::format_size(object.size)
                                    };
                                    
                                    ui.label(size_str);
                                    ui.add_space(100.0);
                                    ui.label(&object.last_modified);
                                });
                                
                                ui.separator();
                            }
                        }
                    });
                } else {
                    ui.label("Select a bucket to view objects");
                }
            });
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
        let tx = self.status_tx.clone();
        
        // Set loading state
        self.set_status_info("Testing AWS credentials...");
        
        // Spawn a task to test credentials
        rt.spawn(async move {
            debug!("Starting async task to test credentials");
            // Clone the auth to avoid holding the lock across await points
            let mut auth_clone_inner = {
                let auth_guard = auth_clone.lock().unwrap();
                auth_guard.clone()
            };
            
            // Now use the cloned auth object
            match auth_clone_inner.test_credentials().await {
                Ok(_) => {
                    info!("AWS credentials validated successfully");
                    let _ = tx.send(StatusMessage::Success("AWS credentials validated successfully".to_string()));
                    
                    // Now load buckets
                    debug!("Loading buckets after successful credential validation");
                    let mut bucket_view = BucketView::default();
                    match bucket_view.load_buckets(auth_clone).await {
                        Ok(buckets) => {
                            debug!("Successfully loaded {} buckets", buckets.len());
                            let _ = tx.send(StatusMessage::Success(format!("Loaded {} buckets", buckets.len())));
                            let _ = tx.send(StatusMessage::BucketList(buckets));
                        },
                        Err(e) => {
                            error!("Failed to load buckets: {}", e);
                            let _ = tx.send(StatusMessage::Error(format!("Failed to load buckets: {}", e)));
                        }
                    }
                },
                Err(err) => {
                    error!("AWS credential validation failed: {}", err);
                    let _ = tx.send(StatusMessage::Error(format!("AWS credential validation failed: {}", err)));
                }
            }
        });
    }
}

impl epi::App for S3SyncApp {
    fn name(&self) -> &str {
        "S3Sync"
    }
    
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        // Process any pending status messages
        self.process_status_messages();
        
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
