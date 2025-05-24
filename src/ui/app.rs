use eframe::egui;
use eframe::epi;
use log::{error};
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::Mutex as TokioMutex;
use std::sync::mpsc;

use super::bucket_view::BucketView;
use super::folder_list::FolderList;
use super::folder_content::FolderContent;
use super::settings::SettingsView;
use super::progress::ProgressView;
use super::filter_view::FilterView;

use crate::aws::auth::AwsAuth;
use crate::aws::transfer::{TransferManager, TransferProgress};
use crate::sync::filter::FileFilter;
use crate::sync::engine::SyncEngine;
use crate::config::credentials::CredentialManager;
use crate::ui::folder_content::FileEntry;
use crate::ui::bucket_view::S3Object;

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
    aws_auth: Arc<TokioMutex<AwsAuth>>,
    status_message: String,
    status_is_error: bool,
    status_tx: mpsc::Sender<StatusMessage>,
    status_rx: mpsc::Receiver<StatusMessage>,
    rt: Handle,
    credential_manager: CredentialManager,
}

/// Current view in the application
enum CurrentView {
    Main,
    Settings,
    Filter,
}

/// Status messages for communication between threads
enum StatusMessage {
    Info(String),
    Error(String),
    ObjectList(Vec<crate::ui::bucket_view::S3Object>),
    BucketList(Vec<String>),
    Progress(TransferProgress),
    SyncComplete,
}

impl Default for S3SyncApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        
        // Create the app instance
        let mut app = Self {
            folder_list: FolderList::default(),
            bucket_view: BucketView::default(),
            folder_content: FolderContent::default(),
            settings_view: SettingsView::default(),
            progress_view: ProgressView::default(),
            filter_view: None,
            current_view: CurrentView::Main,
            show_progress: false,
            aws_auth: Arc::new(TokioMutex::new(AwsAuth::default())),
            status_message: String::new(),
            status_is_error: false,
            status_tx: tx,
            status_rx: rx,
            rt: Handle::current(),
            credential_manager: CredentialManager::default(),
        };
        
        // Try to load credentials from the system keyring
        if CredentialManager::has_credentials() {
            match (
                CredentialManager::load_access_key(),
                CredentialManager::load_secret_key(),
                CredentialManager::load_region()
            ) {
                (Ok(access_key), Ok(secret_key), Ok(region)) if !access_key.is_empty() && !secret_key.is_empty() => {
                    // Update the settings view with the loaded credentials
                    app.settings_view.set_aws_access_key(access_key.clone());
                    app.settings_view.set_aws_secret_key(secret_key.clone());
                    app.settings_view.set_aws_region(region.clone());
                    
                    // Update AWS auth with the loaded credentials
                    let auth_clone = app.aws_auth.clone();
                    let access_key_clone = access_key.clone();
                    let secret_key_clone = secret_key.clone();
                    let region_clone = region.clone();
                    
                    // Use a blocking task to set the credentials
                    tokio::task::block_in_place(|| {
                        app.rt.block_on(async {
                            let mut auth = auth_clone.lock().await;
                            auth.set_credentials(access_key_clone, secret_key_clone, region_clone);
                        });
                    });
                    
                    app.status_message = format!("Loaded credentials from keyring for region {}", region);
                },
                _ => {
                    // No credentials found or error loading them
                    app.status_message = "No saved credentials found. Please enter your AWS credentials in Settings.".to_string();
                }
            }
        }
        
        app
    }
}

impl epi::App for S3SyncApp {
    fn name(&self) -> &str {
        "S3 Sync"
    }
    
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        // Process any status messages
        while let Ok(msg) = self.status_rx.try_recv() {
            match msg {
                StatusMessage::Info(text) => {
                    self.status_message = text;
                    self.status_is_error = false;
                },
                StatusMessage::Error(text) => {
                    self.status_message = text;
                    self.status_is_error = true;
                },
                StatusMessage::ObjectList(objects) => {
                    self.bucket_view.set_objects(objects);
                    self.status_message = format!("Loaded {} objects", self.bucket_view.objects().len());
                },
                StatusMessage::BucketList(buckets) => {
                    self.bucket_view.set_buckets(buckets);
                },
                StatusMessage::Progress(progress) => {
                    self.progress_view.update_progress(progress);
                },
                StatusMessage::SyncComplete => {
                    self.show_progress = false;
                }
            }
        }
        
        // Show progress view if needed
        if self.show_progress {
            self.progress_view.show(ctx);
        }
        
        // Main UI
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
                        self.current_view = CurrentView::Filter;
                        ui.close_menu();
                    }
                    
                    if ui.button("Refresh").clicked() {
                        self.refresh_buckets();
                        ui.close_menu();
                    }
                });
                
                ui.menu_button("Actions", |ui| {
                    if ui.button("Upload").clicked() {
                        self.upload_selected();
                        ui.close_menu();
                    }
                    
                    if ui.button("Download").clicked() {
                        self.download_selected();
                        ui.close_menu();
                    }
                    
                    if ui.button("Sync").clicked() {
                        self.sync_selected();
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
        
        // Status bar
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.status_is_error {
                    ui.colored_label(egui::Color32::RED, &self.status_message);
                } else {
                    ui.label(&self.status_message);
                }
            });
        });
        
        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_view {
                CurrentView::Main => {
                    // Main view with folder list, bucket view, and folder content
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(250.0);
                            self.folder_list.ui(ui);
                            
                            ui.separator();
                            
                            if ui.button("Connect to AWS").clicked() {
                                self.connect_to_aws();
                            }
                            
                            if ui.button("Remove Selected").clicked() {
                                self.folder_list.remove_selected();
                            }
                        });
                        
                        ui.separator();
                        
                        ui.vertical(|ui| {
                            ui.set_width(250.0);
                            if self.bucket_view.ui(ui) {
                                // Bucket selection changed, load objects
                                if let Some(bucket) = self.bucket_view.selected_bucket() {
                                    self.load_bucket_objects(&bucket);
                                }
                            }
                        });
                        
                        ui.separator();
                        
                        ui.vertical(|ui| {
                            if let Some(folder_path) = self.folder_list.selected_folder() {
                                self.folder_content.set_folder(folder_path.clone());
                                self.folder_content.ui(ui);
                            } else if let Some(bucket) = self.bucket_view.selected_bucket() {
                                // Display bucket objects in the content area
                                ui.heading(&format!("Bucket: {}", bucket));
                                
                                // Display objects from the bucket
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    for (index, obj) in self.bucket_view.objects().iter().enumerate() {
                                        // Create a unique ID for each object
                                        let id = egui::Id::new(format!("obj_{}", index));
                                        
                                        // Use a group to create a separate ID context
                                        egui::Frame::none().show(ui, |ui| {
                                            let label_text = if obj.is_directory {
                                                format!("📁 {}", obj.key)
                                            } else {
                                                format!("📄 {} ({} bytes)", obj.key, obj.size)
                                            };
                                            
                                            ui.label(label_text);
                                        });
                                    }
                                    
                                    if self.bucket_view.objects().is_empty() {
                                        ui.label("No objects in this bucket");
                                    }
                                });
                            } else {
                                ui.label("Select a folder or bucket to view its contents");
                            }
                        });
                    });
                },
                CurrentView::Filter => {
                    // Filter view
                    let mut filter_view = self.filter_view.take().unwrap_or_else(|| {
                        FilterView::new(Arc::new(std::sync::Mutex::new(FileFilter::new())))
                    });
                    
                    if filter_view.ui(ui) {
                        // Filter changed
                        let filter_string = filter_view.get_filter().lock().unwrap().to_string();
                        self.folder_content.set_filter(filter_string.clone());
                        self.bucket_view.set_filter(filter_string);
                        
                        // Return to main view
                        self.current_view = CurrentView::Main;
                    }
                    
                    self.filter_view = Some(filter_view);
                },
                CurrentView::Settings => {
                    // Settings view
                    if self.settings_view.ui(ui) {
                        // Settings applied
                        let settings = self.settings_view.get_settings();
                        
                        // Save credentials if requested
                        if settings.save_credentials {
                            CredentialManager::save_credentials(
                                &settings.aws_access_key,
                                &settings.aws_secret_key,
                                &settings.aws_region,
                            ).unwrap_or_else(|e| {
                                error!("Failed to save credentials: {}", e);
                                self.set_status_error(&format!("Failed to save credentials: {}", e));
                            });
                        }
                        
                        // Update AWS auth
                        let aws_auth = self.aws_auth.clone();
                        let access_key = settings.aws_access_key.clone();
                        let secret_key = settings.aws_secret_key.clone();
                        let region = settings.aws_region.clone();
                        
                        self.rt.spawn(async move {
                            let mut auth = aws_auth.lock().await;
                            auth.set_credentials(access_key, secret_key, region);
                        });
                        
                        // Return to main view
                        self.current_view = CurrentView::Main;
                    }
                }
            }
        });
    }
}

impl S3SyncApp {
    /// Set a status info message
    fn set_status_info(&mut self, message: &str) {
        self.status_message = message.to_string();
        self.status_is_error = false;
    }
    
    /// Set a status error message
    fn set_status_error(&mut self, message: &str) {
        self.status_message = message.to_string();
        self.status_is_error = true;
    }
    
    /// Connect to AWS
    fn connect_to_aws(&mut self) {
        let auth_clone = self.aws_auth.clone();
        let tx = self.status_tx.clone();
        let bucket_view_tx = self.status_tx.clone();
        
        self.set_status_info("Connecting to AWS...");
        
        self.rt.spawn(async move {
            // Get a mutable reference to the auth
            let mut auth = auth_clone.lock().await;
            
            let result = auth.test_credentials().await;
            
            match result {
                Ok(_) => {
                    let _ = tx.send(StatusMessage::Info("Connected to AWS".to_string()));
                    
                    // Now that we're connected, try to list buckets
                    drop(auth); // Release the lock before the next await
                    
                    let mut bucket_view = BucketView::default();
                    match bucket_view.load_buckets(auth_clone).await {
                        Ok(buckets) => {
                            // Send the bucket list to the main thread
                            let bucket_count = buckets.len();
                            let _ = bucket_view_tx.send(StatusMessage::BucketList(buckets));
                            let _ = tx.send(StatusMessage::Info(format!("Found {} buckets", bucket_count)));
                        },
                        Err(e) => {
                            let _ = tx.send(StatusMessage::Error(format!("Failed to list buckets: {}", e)));
                        }
                    }
                },
                Err(e) => {
                    let _ = tx.send(StatusMessage::Error(format!("Failed to connect to AWS: {}", e)));
                }
            }
        });
    }
    
    /// Refresh the list of buckets
    fn refresh_buckets(&mut self) {
        let auth_clone = self.aws_auth.clone();
        let tx = self.status_tx.clone();
        
        self.set_status_info("Refreshing buckets...");
        
        self.rt.spawn(async move {
            // Get a mutable reference to the auth
            let mut auth = auth_clone.lock().await;
            
            // Load buckets
            let client = match auth.get_client().await {
                Ok(client) => client.clone(),
                Err(e) => {
                    let _ = tx.send(StatusMessage::Error(format!("Failed to get AWS client: {}", e)));
                    return;
                }
            };
            
            // Release the lock before the next await
            drop(auth);
            
            match client.list_buckets().send().await {
                Ok(resp) => {
                    let buckets: Vec<String> = resp.buckets()
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|b| b.name().map(|s| s.to_string()))
                        .collect();
                        
                    let _ = tx.send(StatusMessage::Info(format!("Found {} buckets", buckets.len())));
                    
                    // Update bucket list in the UI
                    // This needs to be done in the main thread
                },
                Err(e) => {
                    let _ = tx.send(StatusMessage::Error(format!("Failed to list buckets: {}", e)));
                }
            }
        });
    }
    
    /// Refresh the contents of the selected bucket
    fn refresh_selected_bucket(&mut self) {
        if let Some(bucket) = self.bucket_view.selected_bucket() {
            self.load_bucket_objects(&bucket);
        }
    }
    
    /// Load objects from a bucket
    fn load_bucket_objects(&mut self, bucket: &str) {
        let auth_clone = self.aws_auth.clone();
        let bucket_name = bucket.to_string();
        let tx = self.status_tx.clone();
        
        self.set_status_info(&format!("Loading objects from {}...", bucket));
        
        self.rt.spawn(async move {
            let mut bucket_view = BucketView::default();
            match bucket_view.load_objects(auth_clone, &bucket_name).await {
                Ok(objects) => {
                    let _ = tx.send(StatusMessage::ObjectList(objects));
                },
                Err(e) => {
                    let _ = tx.send(StatusMessage::Error(format!("Failed to load objects: {}", e)));
                }
            }
        });
    }
    
    /// Upload selected files to S3
    fn upload_selected(&mut self) {
        // Get the selected bucket
        let bucket = match self.bucket_view.selected_bucket() {
            Some(bucket) => bucket,
            None => {
                self.set_status_error("No bucket selected");
                return;
            }
        };
        
        // Get the selected files and make a copy
        let selected_files = self.folder_content.selected_files();
        if selected_files.is_empty() {
            self.set_status_error("No files selected");
            return;
        }
        
        // Create a vector of owned FileEntry objects
        let selected_files_owned: Vec<FileEntry> = selected_files.into_iter()
            .map(|file| FileEntry {
                path: file.path.clone(),
                name: file.name.clone(),
                is_directory: file.is_directory,
                size: file.size,
                last_modified: file.last_modified.clone(),
            })
            .collect();
            
        // Clone data for the async block
        let bucket_name = bucket.clone();
        
        // Prepare for upload
        let auth_clone = self.aws_auth.clone();
        let tx = self.status_tx.clone();
        let progress_view = self.progress_view.clone();
        
        // Show the progress view
        self.show_progress = true;
        
        self.rt.spawn(async move {
            // Get the AWS client
            let mut auth_guard = auth_clone.lock().await;
            let client = match auth_guard.get_client().await {
                Ok(client) => client.clone(),
                Err(e) => {
                    let _ = tx.send(StatusMessage::Error(format!("Failed to get AWS client: {}", e)));
                    let _ = tx.send(StatusMessage::SyncComplete);
                    return;
                }
            };
            
            // Release the lock
            drop(auth_guard);
            
            // Create a transfer manager
            let transfer_manager = TransferManager::new(client);
            
            // Prepare the list of files to upload
            let mut files_to_upload = Vec::new();
            let mut total_bytes = 0;
            
            for file in selected_files_owned {
                if !file.is_directory {
                    let file_path = file.path.clone();
                    let key_name = file.name.clone();
                    let size = file.size;
                    
                    files_to_upload.push((file_path, key_name, size));
                    total_bytes += size;
                }
            }
            
            let total_files = files_to_upload.len();
            
            if total_files == 0 {
                let _ = tx.send(StatusMessage::Info("No files to upload".to_string()));
                let _ = tx.send(StatusMessage::SyncComplete);
                return;
            }
            
            let _ = tx.send(StatusMessage::Info(format!("Uploading {} files ({} bytes)", total_files, total_bytes)));
            
            let mut success_count = 0;
            let mut failed_count = 0;
            
            // Start progress tracking
            progress_view.start_sync(total_files, total_bytes);
            
            for (file_path, key_name, size) in files_to_upload {
                let file_name = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                
                // Add entry to progress tracker
                progress_view.add_file(&file_name, size);
                
                // Create a progress callback
                let progress_tx = tx.clone();
                let progress_callback = Box::new(move |progress: TransferProgress| {
                    let _ = progress_tx.send(StatusMessage::Progress(progress));
                });
                
                // Upload the file
                match transfer_manager.upload_file(&file_path, &bucket_name, &key_name, Some(progress_callback)).await {
                    Ok(_) => {
                        success_count += 1;
                        progress_view.complete_file(&file_name);
                    },
                    Err(e) => {
                        failed_count += 1;
                        progress_view.fail_file(&file_name);
                        let _ = tx.send(StatusMessage::Error(format!("Failed to upload {}: {}", file_name, e)));
                    }
                }
            }
            
            // Complete the progress tracking
            progress_view.complete_sync();
            
            // Send completion message
            let _ = tx.send(StatusMessage::Info(format!("Upload complete: {} succeeded, {} failed", success_count, failed_count)));
            let _ = tx.send(StatusMessage::SyncComplete);
        });
    }
    
    /// Download selected objects from S3
    fn download_selected(&mut self) {
        // Get the selected bucket
        let bucket = match self.bucket_view.selected_bucket() {
            Some(bucket) => bucket,
            None => {
                self.set_status_error("No bucket selected");
                return;
            }
        };
        
        // Get the selected objects and make a copy
        let selected_objects = self.bucket_view.selected_objects();
        if selected_objects.is_empty() {
            self.set_status_error("No objects selected");
            return;
        }
        
        // Create a vector of owned S3Object objects
        let selected_objects_owned: Vec<S3Object> = selected_objects.into_iter()
            .map(|obj| S3Object {
                key: obj.key.clone(),
                size: obj.size,
                last_modified: obj.last_modified.clone(),
                is_directory: obj.is_directory,
            })
            .collect();
            
        // Get the destination folder
        let folder_path = match self.folder_list.selected_folder() {
            Some(path) => path.clone(),
            None => {
                self.set_status_error("No destination folder selected");
                return;
            }
        };
        
        // Clone data for the async block
        let folder_path_clone = folder_path.clone();
        let bucket_name = bucket.clone();
        
        // Prepare for download
        let auth_clone = self.aws_auth.clone();
        let tx = self.status_tx.clone();
        let progress_view = self.progress_view.clone();
        
        // Show the progress view
        self.show_progress = true;
        
        self.rt.spawn(async move {
            // Get the AWS client
            let mut auth_guard = auth_clone.lock().await;
            let client = match auth_guard.get_client().await {
                Ok(client) => client.clone(),
                Err(e) => {
                    let _ = tx.send(StatusMessage::Error(format!("Failed to get AWS client: {}", e)));
                    let _ = tx.send(StatusMessage::SyncComplete);
                    return;
                }
            };
            
            // Release the lock
            drop(auth_guard);
            
            // Create a transfer manager
            let transfer_manager = TransferManager::new(client);
            
            // Prepare the list of files to download
            let mut files_to_download = Vec::new();
            let mut total_bytes = 0;
            
            for obj in selected_objects_owned {
                if !obj.is_directory {
                    let key_string = obj.key.clone();
                    let local_path = folder_path_clone.join(obj.key.clone());
                    let size = obj.size;
                    
                    files_to_download.push((key_string, local_path, size));
                    total_bytes += size;
                }
            }
            
            let total_files = files_to_download.len();
            
            if total_files == 0 {
                let _ = tx.send(StatusMessage::Info("No files to download".to_string()));
                let _ = tx.send(StatusMessage::SyncComplete);
                return;
            }
            
            let _ = tx.send(StatusMessage::Info(format!("Downloading {} files ({} bytes)", total_files, total_bytes)));
            
            let mut success_count = 0;
            let mut failed_count = 0;
            
            // Start progress tracking
            progress_view.start_sync(total_files, total_bytes);
            
            for (key_string, local_path, size) in files_to_download {
                // Add entry to progress tracker
                progress_view.add_file(&key_string, size);
                
                // Create a progress callback
                let progress_tx = tx.clone();
                let progress_callback = Box::new(move |progress: TransferProgress| {
                    let _ = progress_tx.send(StatusMessage::Progress(progress));
                });
                
                // Download the file
                match transfer_manager.download_file(&bucket_name, &key_string, &local_path, Some(progress_callback)).await {
                    Ok(_) => {
                        success_count += 1;
                        progress_view.complete_file(&key_string);
                    },
                    Err(e) => {
                        failed_count += 1;
                        progress_view.fail_file(&key_string);
                        let _ = tx.send(StatusMessage::Error(format!("Failed to download {}: {}", key_string, e)));
                    }
                }
            }
            
            // Complete the progress tracking
            progress_view.complete_sync();
            
            // Send completion message
            let _ = tx.send(StatusMessage::Info(format!("Download complete: {} succeeded, {} failed", success_count, failed_count)));
            let _ = tx.send(StatusMessage::SyncComplete);
        });
    }
    
    /// Sync the selected folder with the selected bucket
    fn sync_selected(&mut self) {
        // Get the selected bucket
        let bucket = match self.bucket_view.selected_bucket() {
            Some(bucket) => bucket,
            None => {
                self.set_status_error("No bucket selected");
                return;
            }
        };
        
        // Get the selected folder
        let folder_path = match self.folder_list.selected_folder() {
            Some(path) => path.clone(),
            None => {
                self.set_status_error("No folder selected");
                return;
            }
        };
        
        // Clone data for the async block
        let folder_path_clone = folder_path.clone();
        let bucket_name = bucket.clone();
        
        // Prepare for sync
        let auth_clone = self.aws_auth.clone();
        let tx = self.status_tx.clone();
        let progress_view = self.progress_view.clone();
        
        // Show the progress view
        self.show_progress = true;
        
        self.rt.spawn(async move {
            // Get the AWS client
            let mut auth_guard = auth_clone.lock().await;
            let client = match auth_guard.get_client().await {
                Ok(client) => client.clone(),
                Err(e) => {
                    let _ = tx.send(StatusMessage::Error(format!("Failed to get AWS client: {}", e)));
                    let _ = tx.send(StatusMessage::SyncComplete);
                    return;
                }
            };
            
            // Release the lock
            drop(auth_guard);
            
            // Create a transfer manager
            let transfer_manager = TransferManager::new(client);
            
            // Create a sync engine
            let mut sync_engine = SyncEngine::new(transfer_manager);
            
            // Sync the folder
            match sync_engine.sync_folder(&folder_path_clone, &bucket_name, false, None).await {
                Ok(result) => {
                    let _ = tx.send(StatusMessage::Info(format!(
                        "Sync complete: {} files uploaded, {} files downloaded, {} files deleted, {} errors",
                        result.files_uploaded,
                        result.files_downloaded,
                        result.files_deleted,
                        result.errors.len()
                    )));
                    
                    // Log any errors
                    for error in result.errors {
                        let _ = tx.send(StatusMessage::Error(error));
                    }
                },
                Err(e) => {
                    let _ = tx.send(StatusMessage::Error(format!("Sync failed: {}", e)));
                }
            }
            
            // Complete the progress tracking
            progress_view.complete_sync();
            
            // Send completion message
            let _ = tx.send(StatusMessage::SyncComplete);
        });
    }
}
