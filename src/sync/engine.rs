use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

use crate::aws::auth::AwsAuth;
use crate::aws::bucket::BucketManager;
use crate::aws::transfer::{TransferManager, TransferProgress};
use crate::ui::folder_list::{SyncFolder, SyncStatus};
use crate::sync::filter::FileFilter;

use super::diff::{FileAction, FileDiff};

/// Sync engine for synchronizing local folders with S3 buckets
pub struct SyncEngine {
    auth: AwsAuth,
    bucket_manager: BucketManager,
    transfer_manager: TransferManager,
    active_syncs: Arc<Mutex<Vec<PathBuf>>>,
    file_filter: Arc<Mutex<FileFilter>>,
}

/// Sync operation result
pub struct SyncResult {
    pub files_uploaded: usize,
    pub files_downloaded: usize,
    pub files_deleted: usize,
    pub bytes_transferred: u64,
    pub errors: Vec<String>,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new(auth: AwsAuth) -> Self {
        let bucket_manager = BucketManager::new(auth.clone());
        let transfer_manager = TransferManager::new(auth.clone());
        
        Self {
            auth,
            bucket_manager,
            transfer_manager,
            active_syncs: Arc::new(Mutex::new(Vec::new())),
            file_filter: Arc::new(Mutex::new(FileFilter::default())),
        }
    }
    
    /// Get the file filter
    pub fn file_filter(&self) -> Arc<Mutex<FileFilter>> {
        self.file_filter.clone()
    }
    
    /// Sync a folder to an S3 bucket
    pub async fn sync_folder(
        &mut self,
        folder: &mut SyncFolder,
        bucket: &str,
        prefix: Option<&str>,
        delete_removed: bool,
        progress_callback: Option<&dyn Fn(TransferProgress)>,
    ) -> Result<SyncResult> {
        // Check if this folder is already being synced
        {
            let mut active_syncs = self.active_syncs.lock().unwrap();
            if active_syncs.contains(&folder.path) {
                return Err(anyhow!("Folder is already being synced"));
            }
            active_syncs.push(folder.path.clone());
        }
        
        // Update folder status
        folder.status = SyncStatus::Syncing;
        
        // Create result object
        let mut result = SyncResult {
            files_uploaded: 0,
            files_downloaded: 0,
            files_deleted: 0,
            bytes_transferred: 0,
            errors: Vec::new(),
        };
        
        // Ensure bucket exists
        if !self.bucket_manager.bucket_exists(bucket).await? {
            return Err(anyhow!("Bucket {} does not exist", bucket));
        }
        
<<<<<<< HEAD
        // Calculate differences between local and remote
        let diffs = self.calculate_diffs(&folder.path, bucket, prefix).await?;
=======
        // Get the file filter
        let filter = self.file_filter.lock().unwrap().clone();
        
        // Scan local directory
        info!("Scanning local directory: {}", folder.path.display());
        let local_files = match scan_local_directory(&folder.path, Some(&filter)) {
            Ok(files) => files,
            Err(e) => {
                error!("Failed to scan local directory: {}", e);
                folder.status = SyncStatus::Error(format!("Failed to scan directory: {}", e));
                return Err(anyhow!("Failed to scan local directory: {}", e));
            }
        };
        
        // Get S3 objects
        info!("Listing objects in bucket: {}", bucket);
        let s3_objects = match self.bucket_manager.list_objects(bucket, prefix).await {
            Ok(objects) => objects,
            Err(e) => {
                error!("Failed to list S3 objects: {}", e);
                folder.status = SyncStatus::Error(format!("Failed to list S3 objects: {}", e));
                return Err(anyhow!("Failed to list S3 objects: {}", e));
            }
        };
        
        // Convert S3 objects to the same format as local files
        let mut s3_files = HashMap::new();
        for obj in s3_objects {
            // If prefix is specified, remove it from the key to get the relative path
            let key = if let Some(p) = prefix {
                if obj.key.starts_with(p) {
                    obj.key[p.len()..].trim_start_matches('/').to_string()
                } else {
                    obj.key
                }
            } else {
                obj.key
            };
            
            // Use size and etag as a simple hash
            let hash = obj.etag.unwrap_or_else(|| "unknown".to_string());
            s3_files.insert(key, (obj.size as u64, hash));
        }
        
        // Compare files and determine actions
        let diffs = compare_files(&local_files, &s3_files, delete_removed);
        
        info!("Found {} differences to sync", diffs.len());
>>>>>>> 3e8af65 (feat(sync): Add file filtering capabilities)
        
        // Process each difference
        for diff in diffs {
            match diff.action {
                FileAction::Upload => {
                    let local_path = diff.local_path.ok_or_else(|| anyhow!("Missing local path"))?;
                    let s3_key = diff.s3_key.ok_or_else(|| anyhow!("Missing S3 key"))?;
                    
                    match self.transfer_manager.upload_file(&local_path, bucket, &s3_key, progress_callback).await {
                        Ok(_) => {
                            result.files_uploaded += 1;
                            result.bytes_transferred += local_path.metadata().map(|m| m.len()).unwrap_or(0);
                        },
                        Err(e) => {
                            result.errors.push(format!("Failed to upload {}: {}", local_path.display(), e));
                        }
                    }
                },
                FileAction::Download => {
                    let local_path = diff.local_path.ok_or_else(|| anyhow!("Missing local path"))?;
                    let s3_key = diff.s3_key.ok_or_else(|| anyhow!("Missing S3 key"))?;
                    
                    match self.transfer_manager.download_file(bucket, &s3_key, &local_path, progress_callback).await {
                        Ok(_) => {
                            result.files_downloaded += 1;
                            // Size will be updated after download
                            result.bytes_transferred += local_path.metadata().map(|m| m.len()).unwrap_or(0);
                        },
                        Err(e) => {
                            result.errors.push(format!("Failed to download {}: {}", s3_key, e));
                        }
                    }
                },
                FileAction::Delete => {
                    if delete_removed {
                        if let Some(s3_key) = diff.s3_key {
                            match self.bucket_manager.delete_object(bucket, &s3_key).await {
                                Ok(_) => {
                                    result.files_deleted += 1;
                                },
                                Err(e) => {
                                    result.errors.push(format!("Failed to delete {}: {}", s3_key, e));
                                }
                            }
                        }
                    }
                },
                FileAction::None => {
                    // No action needed
                }
            }
        }
        
        // Update folder status
        if result.errors.is_empty() {
            folder.status = SyncStatus::Synced;
            folder.last_synced = Some(chrono::Local::now());
        } else {
            folder.status = SyncStatus::Error(format!("{} errors", result.errors.len()));
        }
        
        // Remove from active syncs
        {
            let mut active_syncs = self.active_syncs.lock().unwrap();
            if let Some(pos) = active_syncs.iter().position(|p| p == &folder.path) {
                active_syncs.remove(pos);
            }
        }
        
        Ok(result)
    }
    
    /// Calculate differences between local folder and S3 bucket
    async fn calculate_diffs(
        &mut self,
        local_path: &Path,
        bucket: &str,
        prefix: Option<&str>,
    ) -> Result<Vec<FileDiff>> {
        let mut diffs = Vec::new();
        
        // Get list of S3 objects
        let s3_objects = self.bucket_manager.list_objects(bucket, prefix).await?;
        
        // Create a map of S3 objects by key
        let mut s3_objects_map = std::collections::HashMap::new();
        for obj in s3_objects {
            s3_objects_map.insert(obj.key.clone(), obj);
        }
        
        // Walk the local directory
        for entry in WalkDir::new(local_path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let relative_path = entry.path().strip_prefix(local_path)?;
                let s3_key = if let Some(prefix) = prefix {
                    format!("{}/{}", prefix, relative_path.to_string_lossy())
                } else {
                    relative_path.to_string_lossy().to_string()
                };
                
                // Check if file exists in S3
                if let Some(s3_obj) = s3_objects_map.remove(&s3_key) {
                    // File exists in both places, check if it needs to be updated
                    let local_metadata = entry.metadata()?;
                    let local_size = local_metadata.len();
                    let s3_size = s3_obj.size as u64;
                    
                    // TODO: Implement more sophisticated comparison (e.g., checksums)
                    if local_size != s3_size {
                        diffs.push(FileDiff {
                            action: FileAction::Upload,
                            local_path: Some(entry.path().to_path_buf()),
                            s3_key: Some(s3_key),
                        });
                    } else {
                        diffs.push(FileDiff {
                            action: FileAction::None,
                            local_path: Some(entry.path().to_path_buf()),
                            s3_key: Some(s3_key),
                        });
                    }
                } else {
                    // File exists locally but not in S3, upload it
                    diffs.push(FileDiff {
                        action: FileAction::Upload,
                        local_path: Some(entry.path().to_path_buf()),
                        s3_key: Some(s3_key),
                    });
                }
            }
        }
        
        // Any remaining S3 objects don't exist locally
        for (s3_key, _) in s3_objects_map {
            // Determine the local path
            let relative_path = if let Some(prefix) = prefix {
                if s3_key.starts_with(prefix) {
                    s3_key[prefix.len()..].trim_start_matches('/')
                } else {
                    &s3_key
                }
            } else {
                &s3_key
            };
            
            let local_file_path = local_path.join(relative_path);
            
            diffs.push(FileDiff {
                action: FileAction::Delete,
                local_path: Some(local_file_path),
                s3_key: Some(s3_key),
            });
        }
        
        Ok(diffs)
    }
}
