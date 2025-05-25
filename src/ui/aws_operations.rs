use log::error;
use std::sync::Arc;

use crate::aws::transfer::TransferManager;
use crate::ui::app_state::{AppState, StatusMessage};

/// AWS-related operations for the application
pub struct AwsOperations;

impl AwsOperations {
    /// Connect to AWS
    pub fn connect_to_aws(app_state: &mut AppState) {
        let auth_clone = app_state.aws_auth.clone();
        let tx = app_state.status_tx.clone();
        let bucket_view_tx = app_state.status_tx.clone();
        
        app_state.set_status_info("Connecting to AWS...");
        
        app_state.rt.spawn(async move {
            // Get a mutable reference to the auth
            let mut auth = auth_clone.lock().await;
            
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
        
        app_state.rt.spawn(async move {
            // Get the AWS client
            let mut auth = auth_clone.lock().await;
            let client = match auth.get_client().await {
                Ok(client) => client,
                Err(e) => {
                    error!("Failed to get AWS client: {}", e);
                    let _ = tx.send(StatusMessage::Error(format!("Failed to get AWS client: {}", e)));
                    return;
                }
            };
            
            // Create a transfer manager
            let transfer_manager = TransferManager::new(client);
            
            // List objects
            match transfer_manager.list_objects(&bucket_name).await {
                Ok(objects) => {
                    let _ = tx.send(StatusMessage::ObjectList(objects));
                },
                Err(e) => {
                    error!("Failed to list objects: {}", e);
                    let _ = tx.send(StatusMessage::Error(format!("Failed to list objects: {}", e)));
                }
            }
        });
    }
    
    /// Upload selected files to S3
    pub fn upload_selected(app_state: &mut AppState) {
        // Implementation will go here
        app_state.set_status_info("Upload functionality not yet implemented");
    }
    
    /// Download selected objects from S3
    pub fn download_selected(app_state: &mut AppState) {
        // Implementation will go here
        app_state.set_status_info("Download functionality not yet implemented");
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
