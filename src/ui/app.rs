use eframe::egui;
use eframe::epi;
use log::{info, error, debug};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use tokio::runtime::Handle;
use std::sync::mpsc;

use super::bucket_view::BucketView;
use super::folder_list::FolderList;
use super::settings::SettingsView;
use super::progress::ProgressView;
use super::filter_view::FilterView;
use super::folder_content::FolderContent;
use crate::aws::auth::AwsAuth;
use crate::config::credentials::CredentialManager;
use crate::aws::transfer::TransferManager;
use crate::aws::transfer::TransferProgress;

/// Main application state
pub struct S3SyncApp {
    folder_list: FolderList,
    bucket_view: BucketView,
    folder_content: FolderContent,
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
            folder_content: FolderContent::default(),
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
            (Err(e), _) | (_, Err(e)) => {
                error!("Error loading credentials from keyring: {}", e);
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
                debug!("Credentials saved to keyring");
                true
            },
            Err(e) => {
                error!("Error saving credentials to keyring: {}", e);
                false
            }
        }
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
                
                // Check if a folder is selected and load its contents
                if let Some(index) = self.folder_list.selected_index {
                    if index < self.folder_list.folders.len() {
                        let folder_path = &self.folder_list.folders[index].path;
                        self.folder_content.load_folder(folder_path);
                    }
                }
            });
            
        egui::CentralPanel::default().show(ctx, |ui| {
            // Split the central panel into two parts
            egui::TopBottomPanel::top("bucket_panel")
                .resizable(true)
                .default_height(ui.available_height() / 2.0)
                .show_inside(ui, |ui| {
                    ui.heading("S3 Bucket Objects");
                    
                    // Bucket selector
                    ui.horizontal(|ui| {
                        ui.label("Bucket:");
                        egui::ComboBox::from_id_source("bucket_selector")
                            .selected_text(self.bucket_view.selected_bucket().unwrap_or_else(|| "Select a bucket".to_string()))
                            .show_ui(ui, |ui| {
                                let buckets = self.bucket_view.buckets().to_vec(); // Clone to avoid borrow issues
                                for bucket in buckets {
                                    let is_selected = self.bucket_view.selected_bucket().as_ref().map_or(false, |s| s == &bucket);
                                    if ui.selectable_label(is_selected, &bucket).clicked() {
                                        let bucket_clone = bucket.clone();
                                        *self.bucket_view.selected_bucket_mut() = Some(bucket.clone());
                                        self.load_objects(&bucket_clone);
                                    }
                                }
                            });
                            
                        if self.bucket_view.is_loading() {
                            ui.label("Loading...");
                        }
                        
                        if ui.button("Refresh").clicked() {
                            if let Some(bucket) = self.bucket_view.selected_bucket() {
                                self.load_objects(&bucket);
                            } else {
                                self.load_buckets();
                            }
                        }
                    });
                    
                    // Error message
                    if let Some(error) = self.bucket_view.error_message() {
                        ui.colored_label(egui::Color32::RED, error);
                    }
                    
                    // Filter
                    ui.horizontal(|ui| {
                        ui.label("Filter:");
                        ui.text_edit_singleline(self.bucket_view.filter_mut());
                        
                        if ui.button("Clear").clicked() {
                            self.bucket_view.clear_filter();
                        }
                        
                        ui.separator();
                        
                        if ui.button("Select All").clicked() {
                            self.bucket_view.select_all_visible();
                        }
                        
                        if ui.button("Deselect All").clicked() {
                            self.bucket_view.clear_selection();
                        }
                        
                        let selected_count = self.bucket_view.selected_objects().len();
                        ui.label(format!("{} selected", selected_count));
                    });
                    
                    ui.separator();
                    
                    // Object list
                    if self.bucket_view.selected_bucket().is_some() {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            // Table header
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut false, ""); // Placeholder for alignment
                                ui.label(egui::RichText::new("Name").strong());
                                ui.add_space(180.0);
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
                                        let is_selected = self.bucket_view.is_object_selected(&object.key);
                                        let mut selected = is_selected;
                                        
                                        if ui.checkbox(&mut selected, "").changed() {
                                            if selected {
                                                self.bucket_view.select_object(&object.key);
                                            } else {
                                                self.bucket_view.deselect_object(&object.key);
                                            }
                                        }
                                        
                                        let icon = if object.is_directory { "📁 " } else { "📄 " };
                                        let text = format!("{}{}", icon, object.key);
                                        let text = if is_selected {
                                            egui::RichText::new(text).strong()
                                        } else {
                                            egui::RichText::new(text)
                                        };
                                        
                                        if ui.label(text).clicked() {
                                            if self.bucket_view.is_object_selected(&object.key) {
                                                self.bucket_view.deselect_object(&object.key);
                                            } else {
                                                self.bucket_view.select_object(&object.key);
                                            }
                                        }
                                        
                                        ui.add_space(180.0 - object.key.len() as f32 * 7.0);
                                        
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
                
            // Show folder contents in the bottom panel
            egui::CentralPanel::default().show_inside(ui, |ui| {
                if let Some(index) = self.folder_list.selected_index {
                    if index < self.folder_list.folders.len() {
                        // Add transfer buttons between bucket and local views
                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() / 2.0 - 100.0);
                            
                            // Upload button (↑)
                            if ui.button("⬆️ Upload").clicked() {
                                if let Some(bucket) = self.bucket_view.selected_bucket() {
                                    self.upload_selected_files(&bucket);
                                } else {
                                    self.set_status_error("Please select a bucket first");
                                }
                            }
                            
                            ui.add_space(20.0);
                            
                            // Download button (↓)
                            if ui.button("⬇️ Download").clicked() {
                                if let Some(bucket) = self.bucket_view.selected_bucket() {
                                    self.download_selected_files(&bucket);
                                } else {
                                    self.set_status_error("Please select a bucket first");
                                }
                            }
                        });
                        
                        ui.separator();
                        self.folder_content.ui(ui);
                    } else {
                        ui.label("Select a folder to view its contents");
                    }
                } else {
                    ui.label("Select a folder to view its contents");
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
    fn load_objects(&mut self, bucket: &str) {
        let auth_clone = self.aws_auth.clone();
        let rt = self.rt.clone();
        let tx = self.status_tx.clone();
        let bucket = bucket.to_string();
        
        // Set loading state
        self.bucket_view.set_loading(true);
        self.set_status_info(&format!("Loading objects from bucket {}...", bucket));
        
        // Spawn a task to load objects
        rt.spawn(async move {
            debug!("Starting async task to load objects from bucket {}", bucket);
            let mut bucket_view = BucketView::default();
            match bucket_view.load_objects(auth_clone, &bucket).await {
                Ok(objects) => {
                    debug!("Successfully loaded {} objects from bucket {}", objects.len(), bucket);
                    let _ = tx.send(StatusMessage::Success(format!("Loaded {} objects from bucket {}", objects.len(), bucket)));
                    let _ = tx.send(StatusMessage::ObjectList(objects));
                },
                Err(e) => {
                    error!("Failed to load objects from bucket {}: {}", bucket, e);
                    let _ = tx.send(StatusMessage::Error(format!("Failed to load objects from bucket {}: {}", bucket, e)));
                }
            }
        });
    }
    
    /// Upload selected files to S3
    fn upload_selected_files(&mut self, bucket: &str) {
        // Get the selected folder path and files
        if let Some(index) = self.folder_list.selected_index {
            if index < self.folder_list.folders.len() {
                let folder_path = &self.folder_list.folders[index].path;
                let selected_files = self.folder_content.selected_files();
                
                if selected_files.is_empty() {
                    self.set_status_error("No files selected for upload");
                    return;
                }
                
                // Create a transfer manager
                let auth_clone = self.aws_auth.clone();
                // let auth_clone = {
                //     let auth_guard = match self.aws_auth.lock() {
                //         Ok(guard) => guard.clone(),
                //         Err(e) => {
                //             self.set_status_error(&format!("Failed to acquire lock on AWS auth: {}", e));
                //             return;
                //         }
                //     }
                // };
                
                let rt = self.rt.clone();
                let tx = self.status_tx.clone();
                let bucket = bucket.to_string();
                let progress_view = self.progress_view.clone();
                
                // Clone the selected files for the async task
                let files_to_upload: Vec<(PathBuf, String, u64)> = selected_files.iter()
                    .filter(|file| !file.is_directory) // Skip directories
                    .map(|file| {
                        // Calculate the S3 key by removing the folder path prefix
                        let rel_path = file.path.strip_prefix(folder_path).unwrap_or(&file.path);
                        let key = rel_path.to_string_lossy().replace('\\', "/");
                        (file.path.clone(), key, file.size)
                    })
                    .collect();
                
                let total_files = files_to_upload.len();
                let total_bytes: u64 = files_to_upload.iter().map(|(_, _, size)| *size).sum();
                
                // Show progress
                self.show_progress = true;
                self.set_status_info(&format!("Uploading {} files to bucket {}", total_files, bucket));
                
                // Start the upload task
                rt.spawn(async move {
                    let mut transfer_manager = TransferManager::new(auth_clone.clone());
                    let mut success_count = 0;
                    let mut failed_count = 0;
                    
                    // Start progress tracking
                    progress_view.start_sync(total_files, total_bytes);
                    
                    for (file_path, key, size) in files_to_upload {
                        let file_name = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        
                        // Add entry to progress tracker
                        progress_view.add_entry(crate::ui::progress::ProgressInfo {
                            file_name: file_name.clone(),
                            operation_type: crate::ui::progress::OperationType::Upload,
                            bytes_transferred: 0,
                            total_bytes: size,
                            percentage: 0.0,
                            status: crate::ui::progress::ProgressStatus::InProgress,
                            message: format!("Uploading to s3://{}/{}", bucket, key),
                            timestamp: std::time::Instant::now(),
                        });
                        
                        // Create a progress callback
                        let progress_view_clone = progress_view.clone();
                        let file_name_clone = file_name.clone();
                        let progress_callback = Box::new(move |progress: TransferProgress| {
                            progress_view_clone.update_entry(
                                &file_name_clone,
                                progress.bytes_transferred,
                                progress.percentage,
                            );
                        });
                        
                        // Upload the file
                        match transfer_manager.upload_file(&file_path, &bucket, &key, Some(progress_callback)).await {
                            Ok(_) => {
                                success_count += 1;
                                progress_view.complete_operation(&file_name, size);
                            },
                            Err(e) => {
                                failed_count += 1;
                                progress_view.fail_operation(&file_name, &format!("Upload failed: {}", e));
                            }
                        }
                    }
                    
                    // Send status message
                    if failed_count == 0 {
                        let _ = tx.send(StatusMessage::Success(
                            format!("Successfully uploaded {} files to bucket {}", success_count, bucket)
                        ));
                    } else {
                        let _ = tx.send(StatusMessage::Error(
                            format!("Upload completed with errors: {} succeeded, {} failed", success_count, failed_count)
                        ));
                    }
                    
                    // Refresh the bucket objects
                    let mut bucket_view = BucketView::default();
                    let auth = Arc::new(Mutex::new(auth_clone.clone()));
                    if let Ok(objects) = bucket_view.load_objects(auth, &bucket).await {
                        let _ = tx.send(StatusMessage::ObjectList(objects));
                    }
                });
            }
        }
    }
    
    /// Download selected files from S3
    fn download_selected_files(&mut self, bucket: &str) {
        // Get the selected folder path and S3 objects
        if let Some(index) = self.folder_list.selected_index {
            if index < self.folder_list.folders.len() {
                let folder_path = &self.folder_list.folders[index].path;
                let selected_objects = self.bucket_view.selected_objects();
                
                if selected_objects.is_empty() {
                    self.set_status_error("No objects selected for download");
                    return;
                }
                
                // Create a transfer manager
                let auth_clone = self.aws_auth.clone();
                // let auth_clone = {
                //     let auth_guard = match self.aws_auth.lock() {
                //         Ok(guard) => guard.clone(),
                //         Err(e) => {
                //             self.set_status_error(&format!("Failed to acquire lock on AWS auth: {}", e));
                //             return;
                //         }
                //     }
                // };
                
                let rt = self.rt.clone();
                let tx = self.status_tx.clone();
                let bucket = bucket.to_string();
                let progress_view = self.progress_view.clone();
                let target_folder = folder_path.clone();
                
                // Clone the selected objects for the async task
                let objects_to_download: Vec<(String, u64)> = selected_objects.iter()
                    .filter(|obj| !obj.is_directory) // Skip directories
                    .map(|obj| (obj.key.clone(), obj.size))
                    .collect();
                
                let total_files = objects_to_download.len();
                let total_bytes: u64 = objects_to_download.iter().map(|(_, size)| *size).sum();
                
                // Show progress
                self.show_progress = true;
                self.set_status_info(&format!("Downloading {} files from bucket {}", total_files, bucket));
                
                // Start the download task
                rt.spawn(async move {
                    let mut transfer_manager = TransferManager::new(auth_clone.clone());
                    let mut success_count = 0;
                    let mut failed_count = 0;
                    
                    // Start progress tracking
                    progress_view.start_sync(total_files, total_bytes);
                    
                    for (key, size) in objects_to_download {
                        // Determine the local file path
                        let local_path = target_folder.join(key.replace('/', std::path::MAIN_SEPARATOR_STR));
                        let file_name = key.split('/').last().unwrap_or(&key).to_string();
                        
                        // Add entry to progress tracker
                        progress_view.add_entry(crate::ui::progress::ProgressInfo {
                            file_name: file_name.clone(),
                            operation_type: crate::ui::progress::OperationType::Download,
                            bytes_transferred: 0,
                            total_bytes: size,
                            percentage: 0.0,
                            status: crate::ui::progress::ProgressStatus::InProgress,
                            message: format!("Downloading from s3://{}/{}", bucket, key),
                            timestamp: std::time::Instant::now(),
                        });
                        
                        // Create a progress callback
                        let progress_view_clone = progress_view.clone();
                        let file_name_clone = file_name.clone();
                        let progress_callback = Box::new(move |progress: TransferProgress| {
                            progress_view_clone.update_entry(
                                &file_name_clone,
                                progress.bytes_transferred,
                                progress.percentage,
                            );
                        });
                        
                        // Download the file
                        match transfer_manager.download_file(&bucket, &key, &local_path, Some(progress_callback)).await {
                            Ok(_) => {
                                success_count += 1;
                                progress_view.complete_operation(&file_name, size);
                            },
                            Err(e) => {
                                failed_count += 1;
                                progress_view.fail_operation(&file_name, &format!("Download failed: {}", e));
                            }
                        }
                    }
                    
                    // Send status message
                    if failed_count == 0 {
                        let _ = tx.send(StatusMessage::Success(
                            format!("Successfully downloaded {} files from bucket {}", success_count, bucket)
                        ));
                    } else {
                        let _ = tx.send(StatusMessage::Error(
                            format!("Download completed with errors: {} succeeded, {} failed", success_count, failed_count)
                        ));
                    }
                });
            }
        }
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
            drop(auth); // Release the lock before calling load_buckets
            self.load_buckets();
        }
    }
}
