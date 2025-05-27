use log::{error, debug};
use std::sync::Arc;
use aws_sdk_s3::error::ProvideErrorMetadata;

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
                    let _ = tx.send(StatusMessage::Error(format!("Failed to get AWS client for region {}: {}", region, e)));
                    return;
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
                // Extract the location constraint as a string
                let location_str = match resp.location_constraint() {
                    Some(constraint) => {
                        // Convert the enum to a debug string and extract the value
                        let debug_str = format!("{:?}", constraint);
                        if debug_str.contains("\"\"") || debug_str == "Empty" {
                            // Empty constraint means us-east-1
                            "us-east-1".to_string()
                        } else if debug_str.starts_with("Unknown(") {
                            // Extract the value from Unknown("value")
                            let start = debug_str.find('(').map(|i| i + 2).unwrap_or(0);
                            let end = debug_str.rfind('"').unwrap_or(debug_str.len());
                            if start < end {
                                debug_str[start..end].to_string()
                            } else {
                                "us-east-1".to_string() // Default if parsing fails
                            }
                        } else {
                            // For known enum variants, extract the region name
                            let region_name = match debug_str.as_str() {
                                "EuWest1" => "eu-west-1",
                                "UsWest1" => "us-west-1",
                                "UsWest2" => "us-west-2",
                                "EuWest2" => "eu-west-2",
                                "EuWest3" => "eu-west-3",
                                "UsEast2" => "us-east-2",
                                "ApSouth1" => "ap-south-1",
                                "ApSoutheast1" => "ap-southeast-1",
                                "ApSoutheast2" => "ap-southeast-2",
                                "ApNortheast1" => "ap-northeast-1",
                                "ApNortheast2" => "ap-northeast-2",
                                "ApNortheast3" => "ap-northeast-3",
                                "SaEast1" => "sa-east-1",
                                "CnNorth1" => "cn-north-1",
                                "CnNorthwest1" => "cn-northwest-1",
                                "UsGovWest1" => "us-gov-west-1",
                                "UsGovEast1" => "us-gov-east-1",
                                "EuCentral1" => "eu-central-1",
                                "EuNorth1" => "eu-north-1",
                                "MeSouth1" => "me-south-1",
                                "AfSouth1" => "af-south-1",
                                "EuSouth1" => "eu-south-1",
                                "ApEast1" => "ap-east-1",
                                _ => "us-east-1", // Default for unknown regions
                            };
                            region_name.to_string()
                        }
                    },
                    None => "us-east-1".to_string(), // Default if no constraint is specified
                };
                
                Ok(location_str)
            },
            Err(err) => {
                // Convert to a string error without using code() and message()
                Err(format!("Failed to get bucket location: {}", err))
            }
        }
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
