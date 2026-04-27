use log::{error, debug};
// use std::sync::Arc; // Unused
use std::path::{Path, PathBuf};

use crate::aws::transfer::TransferManager;
use crate::ui::app_state::{AppState, StatusMessage};
use crate::ui::bucket_view::S3Object;

/// AWS-related operations for the application
pub struct AwsOperations;

impl AwsOperations {
    /// Connect to AWS
    pub fn connect_to_aws(app_state: &mut AppState) {
        let auth_clone = app_state.aws_auth.clone();
        let tx = app_state.status_tx.clone();
        let bucket_view_tx = app_state.status_tx.clone();
        
        // Grab credentials from settings view to pass into the async task
        let access_key = app_state.settings_view.aws_access_key();
        let secret_key = app_state.settings_view.aws_secret_key();
        let region = app_state.settings_view.aws_region();
        
        if access_key.is_empty() || secret_key.is_empty() {
            app_state.set_status_error("AWS credentials not set. Please enter them in File → Settings first.");
            return;
        }
        
        app_state.set_status_info("Connecting to AWS...");
        
        app_state.rt.spawn(async move {
            let mut auth = auth_clone.lock().await;
            
            // Ensure credentials are set on the auth object
            auth.set_credentials(access_key, secret_key, region);
            
            // Initialize the AWS SDK
            match auth.initialize().await {
                Ok(_) => {
                    // Successfully connected
                    let _ = tx.send(StatusMessage::Info("Connected to AWS".to_string()));
                    
                    // Create a transfer manager
                    let client = match auth.get_client().await {
                        Ok(client) => client,
                        Err(e) => {
                            error!("Failed to get AWS client: {}", e);
                            let _ = tx.send(StatusMessage::Error(format!("Failed to get AWS client: {}", e)));
                            return;
                        }
                    };
                    
                    let transfer_manager = TransferManager::new(client);
                    
                    // List buckets
                    match transfer_manager.list_buckets().await {
                        Ok(buckets) => {
                            let _ = bucket_view_tx.send(StatusMessage::BucketList(buckets));
                        },
                        Err(e) => {
                            error!("Failed to list buckets: {}", e);
                            let _ = tx.send(StatusMessage::Error(format!("Failed to list buckets: {}", e)));
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to connect to AWS: {}", e);
                    let _ = tx.send(StatusMessage::Error(format!("Failed to connect to AWS: {}", e)));
                }
            }
        });
    }
    
    /// Load objects from a bucket
    pub fn load_bucket_objects(app_state: &mut AppState, bucket: &str) {
        let auth_clone = app_state.aws_auth.clone();
        let tx = app_state.status_tx.clone();
        let bucket_name = bucket.to_string();
        
        app_state.set_status_info(&format!("Loading objects from bucket {}...", bucket));
        app_state.bucket_view.set_loading(true);
        
        // Get the bucket region from the bucket view
        let bucket_region = app_state.bucket_view.get_bucket_region(bucket).cloned();
        
        app_state.rt.spawn(async move {
            // Get the AWS client for the specific region if available
            let mut auth = auth_clone.lock().await;
            
            // First, try to get the region if we don't have it yet
            let region = if let Some(region) = bucket_region {
                debug!("Using cached region {} for bucket {}", region, bucket_name);
                region
            } else {
                // Try to get the default client to query the bucket location
                let default_client = match auth.get_client().await {
                    Ok(client) => client,
                    Err(e) => {
                        error!("Failed to get default AWS client: {}", e);
                        let _ = tx.send(StatusMessage::Error(format!("Failed to get AWS client: {}", e)));
                        return;
                    }
                };
                
                // Try to get the bucket location
                match Self::get_bucket_location(&default_client, &bucket_name).await {
                    Ok(region) => {
                        debug!("Detected region {} for bucket {}", region, bucket_name);
                        region
                    },
                    Err(e) => {
                        error!("Failed to get region for bucket {}: {}", bucket_name, e);
                        // Default to us-east-1 if we can't determine the region
                        "us-east-1".to_string()
                    }
                }
            };
            
            // Now get a client for the specific region
            let client = match auth.get_client_for_region(&region).await {
                Ok(client) => {
                    debug!("Using region-specific client for bucket {} in region {}", bucket_name, region);
                    client
                },
                Err(e) => {
                    error!("Failed to get AWS client for region {}: {}", region, e);
                    
                    // Try with us-east-2 as a fallback
                    match auth.get_client_for_region("us-east-2").await {
                        Ok(client) => {
                            debug!("Using fallback us-east-2 client for bucket {}", bucket_name);
                            client
                        },
                        Err(fallback_err) => {
                            error!("Failed to get fallback AWS client: {}", fallback_err);
                            let _ = tx.send(StatusMessage::Error(format!("Failed to get AWS client: {}", e)));
                            return;
                        }
                    }
                }
            };
            
            // Create a transfer manager
            let transfer_manager = TransferManager::new(client);
            
            // List objects with improved error handling
            match transfer_manager.list_objects(&bucket_name).await {
                Ok(objects) => {
                    let _ = tx.send(StatusMessage::ObjectList(objects));
                },
                Err(e) => {
                    // If we get an error and we're not using us-east-2, try that region as a fallback
                    if region != "us-east-2" {
                        error!("Failed with region {}, trying us-east-2 as fallback", region);
                        
                        // Try with us-east-2 client
                        match auth.get_client_for_region("us-east-2").await {
                            Ok(client) => {
                                let transfer_manager = TransferManager::new(client);
                                match transfer_manager.list_objects(&bucket_name).await {
                                    Ok(objects) => {
                                        debug!("Successfully listed objects using us-east-2 region");
                                        let _ = tx.send(StatusMessage::ObjectList(objects));
                                        return;
                                    },
                                    Err(fallback_err) => {
                                        error!("Fallback to us-east-2 also failed: {}", fallback_err);
                                    }
                                }
                            },
                            Err(client_err) => {
                                error!("Failed to get us-east-2 client: {}", client_err);
                            }
                        }
                    }
                    
                    // Log the detailed error
                    error!("Failed to list objects: {}", e);
                    
                    // Send a more user-friendly error message to the UI
                    let error_message = if e.to_string().contains("S3 service error") {
                        // This is our enhanced error message from the transfer.rs improvement
                        format!("{}", e)
                    } else {
                        // Generic error handling for other types of errors
                        format!("Failed to list objects: {}", e)
                    };
                    
                    let _ = tx.send(StatusMessage::Error(error_message));
                }
            }
        });
    }
    
    /// Helper function to get the location (region) of a bucket
    async fn get_bucket_location(client: &aws_sdk_s3::Client, bucket: &str) -> Result<String, String> {
        match client.get_bucket_location().bucket(bucket).send().await {
            Ok(resp) => {
                let location = resp.location_constraint()
                    .map(|c| c.as_str().to_string())
                    .unwrap_or_default();
                if location.is_empty() {
                    Ok("us-east-1".to_string())
                } else {
                    Ok(location)
                }
            },
            Err(err) => Err(format!("Failed to get bucket location: {}", err)),
        }
    }
    
    /// Upload selected files to S3
    pub fn upload_selected(app_state: &mut AppState) {
        // Check if we have a selected bucket and folder
        let bucket = match app_state.bucket_view.selected_bucket() {
            Some(bucket) => bucket.clone(),
            None => {
                app_state.set_status_error("No S3 bucket selected for upload");
                return;
            }
        };
        
        // Get the selected folder path
        let folder_path = match app_state.folder_list.selected_folder() {
            Some(path) => path.clone(),
            None => {
                app_state.set_status_error("No local folder selected for upload");
                return;
            }
        };
        
        // Get the selected files
        let selected_files = app_state.folder_content.selected_files();
        if selected_files.is_empty() {
            app_state.set_status_error("No files selected to upload");
            return;
        }
        
        // Clone necessary data for the async task
        let auth_clone = app_state.aws_auth.clone();
        let tx = app_state.status_tx.clone();
        let bucket_name = bucket.clone();
        let folder_path_clone = folder_path.clone();
        let files_to_upload: Vec<PathBuf> = selected_files.iter()
            .filter(|file| !file.is_directory) // Skip directories
            .map(|file| file.path.clone())
            .collect();
        
        // Get the bucket region from the bucket view
        let bucket_region = app_state.bucket_view.get_bucket_region(&bucket).cloned();
        
        app_state.set_status_info(&format!("Uploading {} files to bucket {}...", files_to_upload.len(), bucket));
        
        // Spawn an async task to handle the upload
        app_state.rt.spawn(async move {
            // Get the AWS client
            let mut auth = auth_clone.lock().await;
            
            // First, try to get the region if we don't have it yet
            let region = if let Some(region) = bucket_region {
                debug!("Using cached region {} for bucket {}", region, bucket_name);
                region
            } else {
                // Try to get the default client to query the bucket location
                let default_client = match auth.get_client().await {
                    Ok(client) => client,
                    Err(e) => {
                        error!("Failed to get default AWS client: {}", e);
                        let _ = tx.send(StatusMessage::Error(format!("Failed to get AWS client: {}", e)));
                        return;
                    }
                };
                
                // Try to get the bucket location
                match Self::get_bucket_location(&default_client, &bucket_name).await {
                    Ok(region) => {
                        debug!("Detected region {} for bucket {}", region, bucket_name);
                        region
                    },
                    Err(e) => {
                        error!("Failed to get region for bucket {}: {}", bucket_name, e);
                        // Try us-east-1 as default
                        "us-east-1".to_string()
                    }
                }
            };
            
            // Now get a client for the specific region
            let client = match auth.get_client_for_region(&region).await {
                Ok(client) => {
                    debug!("Using region-specific client for bucket {} in region {}", bucket_name, region);
                    client
                },
                Err(e) => {
                    error!("Failed to get AWS client for region {}: {}", region, e);
                    
                    // Try with us-east-2 as a fallback
                    match auth.get_client_for_region("us-east-2").await {
                        Ok(client) => {
                            debug!("Using fallback us-east-2 client for bucket {}", bucket_name);
                            client
                        },
                        Err(fallback_err) => {
                            error!("Failed to get fallback AWS client: {}", fallback_err);
                            let _ = tx.send(StatusMessage::Error(format!("Failed to get AWS client: {}", e)));
                            return;
                        }
                    }
                }
            };
            
            // Create a transfer manager
            let transfer_manager = TransferManager::new(client);
            
            // Track upload statistics
            let mut success_count = 0;
            let mut error_count = 0;
            
            // Process each file
            for file_path in files_to_upload {
                // Calculate the S3 key by removing the folder path prefix
                let rel_path = match file_path.strip_prefix(&folder_path_clone) {
                    Ok(rel) => rel,
                    Err(_) => {
                        // If we can't determine the relative path, use the file name
                        match file_path.file_name() {
                            Some(name) => Path::new(name),
                            None => {
                                error!("Could not determine file name for {}", file_path.display());
                                error_count += 1;
                                continue;
                            }
                        }
                    }
                };
                
                let s3_key = rel_path.to_string_lossy().replace('\\', "/");
                
                // Upload the file
                match transfer_manager.upload_file(&file_path, &bucket_name, &s3_key, None).await {
                    Ok(_) => {
                        success_count += 1;
                        debug!("Successfully uploaded {} to s3://{}/{}", file_path.display(), bucket_name, s3_key);
                    },
                    Err(e) => {
                        error_count += 1;
                        error!("Failed to upload {}: {}", file_path.display(), e);
                        error!("Error details: {:#?}", e.to_string());
                    }
                }
            }
            
            // Send status message
            if error_count == 0 {
                let _ = tx.send(StatusMessage::Info(
                    format!("Successfully uploaded {} files to bucket {}", success_count, bucket_name)
                ));
            } else {
                let _ = tx.send(StatusMessage::Error(
                    format!("Upload completed with errors: {} succeeded, {} failed", success_count, error_count)
                ));
            }
            
            // Refresh the bucket objects
            let _ = tx.send(StatusMessage::Info(format!("Refreshing bucket contents...")));
            match transfer_manager.list_objects(&bucket_name).await {
                Ok(objects) => {
                    let _ = tx.send(StatusMessage::ObjectList(objects));
                },
                Err(e) => {
                    error!("Failed to refresh bucket objects: {}", e);
                }
            }
        });
    }
    
    /// Download selected objects from S3
    pub fn download_selected(app_state: &mut AppState) {
        // Check if we have a selected bucket
        let bucket = match app_state.bucket_view.selected_bucket() {
            Some(bucket) => bucket.clone(),
            None => {
                app_state.set_status_error("No S3 bucket selected for download");
                return;
            }
        };
        
        // Get the selected folder path for download destination
        let folder_path = match app_state.folder_list.selected_folder() {
            Some(path) => path.clone(),
            None => {
                app_state.set_status_error("No local folder selected as download destination");
                return;
            }
        };
        
        // Get the selected objects
        let selected_objects = app_state.bucket_view.selected_objects();
        if selected_objects.is_empty() {
            app_state.set_status_error("No objects selected to download");
            return;
        }
        
        // Clone necessary data for the async task
        let auth_clone = app_state.aws_auth.clone();
        let tx = app_state.status_tx.clone();
        let bucket_name = bucket.clone();
        let folder_path_clone = folder_path.clone();
        
        // Create a vector of objects to download
        let objects_to_download: Vec<S3Object> = selected_objects.iter()
            .filter(|obj| !obj.is_directory) // Skip directories
            .map(|&obj| obj.clone())
            .collect();
        
        // Get the bucket region from the bucket view
        let bucket_region = app_state.bucket_view.get_bucket_region(&bucket).cloned();
        
        app_state.set_status_info(&format!("Downloading {} files from bucket {}...", objects_to_download.len(), bucket));
        
        // Spawn an async task to handle the download
        app_state.rt.spawn(async move {
            // Get the AWS client
            let mut auth = auth_clone.lock().await;
            
            // First, try to get the region if we don't have it yet
            let region = if let Some(region) = bucket_region {
                debug!("Using cached region {} for bucket {}", region, bucket_name);
                region
            } else {
                // Try to get the default client to query the bucket location
                let default_client = match auth.get_client().await {
                    Ok(client) => client,
                    Err(e) => {
                        error!("Failed to get default AWS client: {}", e);
                        let _ = tx.send(StatusMessage::Error(format!("Failed to get AWS client: {}", e)));
                        return;
                    }
                };
                
                // Try to get the bucket location
                match Self::get_bucket_location(&default_client, &bucket_name).await {
                    Ok(region) => {
                        debug!("Detected region {} for bucket {}", region, bucket_name);
                        region
                    },
                    Err(e) => {
                        error!("Failed to get region for bucket {}: {}", bucket_name, e);
                        // Try us-east-1 as default
                        "us-east-1".to_string()
                    }
                }
            };
            
            // Now get a client for the specific region
            let client = match auth.get_client_for_region(&region).await {
                Ok(client) => {
                    debug!("Using region-specific client for bucket {} in region {}", bucket_name, region);
                    client
                },
                Err(e) => {
                    error!("Failed to get AWS client for region {}: {}", region, e);
                    
                    // Try with us-east-2 as a fallback
                    match auth.get_client_for_region("us-east-2").await {
                        Ok(client) => {
                            debug!("Using fallback us-east-2 client for bucket {}", bucket_name);
                            client
                        },
                        Err(fallback_err) => {
                            error!("Failed to get fallback AWS client: {}", fallback_err);
                            let _ = tx.send(StatusMessage::Error(format!("Failed to get AWS client: {}", e)));
                            return;
                        }
                    }
                }
            };
            
            // Create a transfer manager
            let transfer_manager = TransferManager::new(client);
            
            // Track download statistics
            let mut success_count = 0;
            let mut error_count = 0;
            
            // Process each object
            for object in objects_to_download {
                // Calculate the local file path
                let local_path = folder_path_clone.join(object.key.replace('/', std::path::MAIN_SEPARATOR_STR));
                
                // Create parent directories if they don't exist
                if let Some(parent) = local_path.parent() {
                    if !parent.exists() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            error!("Failed to create directory {}: {}", parent.display(), e);
                            error_count += 1;
                            continue;
                        }
                    }
                }
                
                // Download the file
                match transfer_manager.download_file(&bucket_name, &object.key, &local_path, None).await {
                    Ok(_) => {
                        success_count += 1;
                        debug!("Successfully downloaded s3://{}/{} to {}", bucket_name, object.key, local_path.display());
                    },
                    Err(e) => {
                        error_count += 1;
                        error!("Failed to download {}: {}", object.key, e);
                        error!("Error details: {:#?}", e.to_string());
                    }
                }
            }
            
            // Send status message
            if error_count == 0 {
                let _ = tx.send(StatusMessage::Info(
                    format!("Successfully downloaded {} files from bucket {}", success_count, bucket_name)
                ));
            } else {
                let _ = tx.send(StatusMessage::Error(
                    format!("Download completed with errors: {} succeeded, {} failed", success_count, error_count)
                ));
            }
            
            // Refresh the local folder contents
            let _ = tx.send(StatusMessage::Info(format!("Refreshing local folder contents...")));
        });
    }
    
    /// Sync selected folders with S3
    pub fn sync_selected(app_state: &mut AppState) {
        // Implementation will go here
        app_state.set_status_info("Sync functionality not yet implemented");
    }
    
    /// Refresh the list of buckets
    pub fn refresh_buckets(app_state: &mut AppState) {
        Self::connect_to_aws(app_state);
    }
}
