use eframe::egui;
use eframe::epi;
use log::{info, error, debug};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use tokio::runtime::Handle;
use std::sync::mpsc;
use std::time::Instant;

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

impl S3SyncApp {
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
                
                // Create a transfer manager with proper auth cloning
                let auth_clone = {
                    let auth_guard = match self.aws_auth.lock() {
                        Ok(guard) => guard.clone(),
                        Err(e) => {
                            self.set_status_error(&format!("Failed to acquire lock on AWS auth: {}", e));
                            return;
                        }
                    };
                    auth_guard // Return the cloned auth
                };
                
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
                            timestamp: Instant::now(),
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
                    let auth = Arc::new(Mutex::new(auth_clone));
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
                
                // Create a transfer manager with proper auth cloning
                let auth_clone = {
                    let auth_guard = match self.aws_auth.lock() {
                        Ok(guard) => guard.clone(),
                        Err(e) => {
                            self.set_status_error(&format!("Failed to acquire lock on AWS auth: {}", e));
                            return;
                        }
                    };
                    auth_guard // Return the cloned auth
                };
                
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
                            timestamp: Instant::now(),
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
