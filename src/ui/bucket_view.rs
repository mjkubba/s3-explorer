use eframe::egui;
use std::sync::{Arc, Mutex};
use log::{info, error, debug};
use std::collections::HashMap;

use crate::aws::auth::AwsAuth;

/// Component for viewing and interacting with S3 buckets
#[derive(Default)]
pub struct BucketView {
    buckets: Vec<String>,
    selected_bucket: Option<String>,
    objects: Vec<S3Object>,
    filter: String,
    loading: bool,
    error_message: Option<String>,
    bucket_regions: HashMap<String, String>,
}

/// Represents an object in an S3 bucket
#[derive(Clone)]
pub struct S3Object {
    pub key: String,
    pub size: u64,
    pub last_modified: String,
    pub is_directory: bool,
}

impl BucketView {
    /// Render the bucket view UI
    pub fn ui(&mut self, _ui: &mut egui::Ui) {
        // This is now handled by the app.rs file
    }
    
    /// Set the list of buckets
    pub fn set_buckets(&mut self, buckets: Vec<String>) {
        debug!("Setting bucket list: {} buckets", buckets.len());
        self.buckets = buckets;
        self.loading = false;
    }
    
    /// Set the list of objects for the selected bucket
    pub fn set_objects(&mut self, objects: Vec<S3Object>) {
        debug!("Setting object list: {} objects", objects.len());
        self.objects = objects;
    }
    
    /// Set an error message
    pub fn set_error(&mut self, message: String) {
        error!("Bucket view error: {}", message);
        self.error_message = Some(message);
        self.loading = false;
    }
    
    /// Clear the error message
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }
    
    /// Get the error message
    pub fn error_message(&self) -> Option<&String> {
        self.error_message.as_ref()
    }
    
    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        debug!("Setting loading state: {}", loading);
        self.loading = loading;
    }
    
    /// Check if the view is in loading state
    pub fn is_loading(&self) -> bool {
        self.loading
    }
    
    /// Get the selected bucket
    pub fn selected_bucket(&self) -> Option<String> {
        self.selected_bucket.clone()
    }
    
    /// Get a mutable reference to the selected bucket
    pub fn selected_bucket_mut(&mut self) -> &mut Option<String> {
        &mut self.selected_bucket
    }
    
    /// Get the filter string
    pub fn filter(&self) -> &str {
        &self.filter
    }
    
    /// Get a mutable reference to the filter string
    pub fn filter_mut(&mut self) -> &mut String {
        &mut self.filter
    }
    
    /// Clear the filter
    pub fn clear_filter(&mut self) {
        self.filter.clear();
    }
    
    /// Get the list of buckets
    pub fn buckets(&self) -> &[String] {
        &self.buckets
    }
    
    /// Get the list of objects
    pub fn objects(&self) -> &[S3Object] {
        &self.objects
    }
    
    /// Get the region for a bucket
    pub fn get_bucket_region(&self, bucket: &str) -> Option<&String> {
        self.bucket_regions.get(bucket)
    }
    
    /// Set the region for a bucket
    pub fn set_bucket_region(&mut self, bucket: String, region: String) {
        debug!("Setting region for bucket {}: {}", bucket, region);
        self.bucket_regions.insert(bucket, region);
    }
    
    /// Load buckets from AWS
    pub async fn load_buckets(&mut self, aws_auth: Arc<Mutex<AwsAuth>>) -> Result<Vec<String>, String> {
        debug!("Loading buckets from AWS");
        self.loading = true;
        
        // Clone the auth to avoid holding the lock across await points
        let mut auth_clone = {
            let auth_guard = match aws_auth.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    let error = format!("Failed to acquire lock on AWS auth: {}", e);
                    error!("{}", error);
                    return Err(error);
                }
            };
            
            // Check if credentials are empty
            if auth_guard.is_empty() {
                self.loading = false;
                return Err("AWS credentials not provided".to_string());
            }
            
            // Clone the auth object
            auth_guard.clone()
        };
        
        // Now use the cloned auth object
        debug!("Getting AWS client");
        let client = match auth_clone.get_client().await {
            Ok(client) => client,
            Err(e) => {
                let error = format!("Failed to get AWS client: {}", e);
                error!("{}", error);
                self.loading = false;
                return Err(error);
            }
        };
        
        debug!("Sending list_buckets request");
        match client.list_buckets().send().await {
            Ok(resp) => {
                let buckets = resp.buckets().unwrap_or_default();
                let bucket_names: Vec<String> = buckets
                    .iter()
                    .filter_map(|b| b.name().map(String::from))
                    .collect();
                    
                info!("Listed {} S3 buckets", bucket_names.len());
                
                // Get the region for each bucket
                for bucket_name in &bucket_names {
                    match auth_clone.get_bucket_location(bucket_name).await {
                        Ok(region) => {
                            self.set_bucket_region(bucket_name.clone(), region);
                        },
                        Err(e) => {
                            error!("Failed to get region for bucket {}: {}", bucket_name, e);
                            // Default to the current region if we can't get the bucket location
                            self.set_bucket_region(bucket_name.clone(), auth_clone.region().to_string());
                        }
                    }
                }
                
                self.buckets = bucket_names.clone();
                self.loading = false;
                Ok(bucket_names)
            },
            Err(err) => {
                let sdk_error = err.into_service_error();
                let error_code = sdk_error.code().unwrap_or("Unknown");
                let error_message = sdk_error.message().unwrap_or("No error message");
                
                let error = format!("Failed to list buckets: {} - {}", error_code, error_message);
                error!("{}", error);
                self.loading = false;
                Err(error)
            }
        }
    }
    
    /// Load objects from the selected bucket
    pub async fn load_objects(&mut self, aws_auth: Arc<Mutex<AwsAuth>>, bucket: &str) -> Result<Vec<S3Object>, String> {
        debug!("Loading objects from bucket: {}", bucket);
        self.loading = true;
        
        // Get the region for this bucket
        let bucket_region = match self.get_bucket_region(bucket) {
            Some(region) => region.clone(),
            None => {
                // If we don't have the region cached, try to get it
                let mut auth_clone = {
                    let auth_guard = match aws_auth.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            let error = format!("Failed to acquire lock on AWS auth: {}", e);
                            error!("{}", error);
                            return Err(error);
                        }
                    };
                    
                    // Clone the auth object
                    auth_guard.clone()
                };
                
                match auth_clone.get_bucket_location(bucket).await {
                    Ok(region) => {
                        self.set_bucket_region(bucket.to_string(), region.clone());
                        region
                    },
                    Err(e) => {
                        error!("Failed to get region for bucket {}: {}", bucket, e);
                        // Default to the current region if we can't get the bucket location
                        auth_clone.region().to_string()
                    }
                }
            }
        };
        
        debug!("Using region {} for bucket {}", bucket_region, bucket);
        
        // Clone the auth to avoid holding the lock across await points
        let auth_clone = {
            let auth_guard = match aws_auth.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    let error = format!("Failed to acquire lock on AWS auth: {}", e);
                    error!("{}", error);
                    return Err(error);
                }
            };
            
            // Clone the auth object
            auth_guard.clone()
        };
        
        // Now use the cloned auth object to get a client for the specific region
        debug!("Getting AWS client for region {}", bucket_region);
        let client = match auth_clone.get_client_for_region(&bucket_region).await {
            Ok(client) => client,
            Err(e) => {
                let error = format!("Failed to get AWS client for region {}: {}", bucket_region, e);
                error!("{}", error);
                self.loading = false;
                return Err(error);
            }
        };
        
        debug!("Sending list_objects_v2 request for bucket: {} in region {}", bucket, bucket_region);
        match client.list_objects_v2().bucket(bucket).send().await {
            Ok(resp) => {
                let objects = resp.contents().unwrap_or_default();
                let mut s3_objects = Vec::new();
                
                // Process objects and identify directories
                let mut directories = std::collections::HashSet::new();
                
                for obj in objects {
                    let key = obj.key().unwrap_or_default();
                    let size = obj.size() as u64;
                    let last_modified = obj.last_modified()
                        .map(|dt| format!("{:?}", dt))
                        .unwrap_or_else(|| "Unknown".to_string());
                    
                    // Check if this is a "directory" (prefix)
                    if key.ends_with('/') {
                        directories.insert(key.to_string());
                        s3_objects.push(S3Object {
                            key: key.to_string(),
                            size: 0,
                            last_modified,
                            is_directory: true,
                        });
                    } else {
                        // Check if this object is in a directory
                        let mut path_parts = key.split('/').collect::<Vec<_>>();
                        if path_parts.len() > 1 {
                            path_parts.pop(); // Remove the filename
                            let dir_path = path_parts.join("/") + "/";
                            directories.insert(dir_path.clone());
                        }
                        
                        s3_objects.push(S3Object {
                            key: key.to_string(),
                            size,
                            last_modified,
                            is_directory: false,
                        });
                    }
                }
                
                // Add any directories that weren't explicitly listed
                for dir in directories {
                    if !s3_objects.iter().any(|obj| obj.key == dir && obj.is_directory) {
                        s3_objects.push(S3Object {
                            key: dir,
                            size: 0,
                            last_modified: "".to_string(),
                            is_directory: true,
                        });
                    }
                }
                
                // Sort: directories first, then by name
                s3_objects.sort_by(|a, b| {
                    match (a.is_directory, b.is_directory) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.key.cmp(&b.key),
                    }
                });
                
                info!("Listed {} objects in bucket {} (region {})", s3_objects.len(), bucket, bucket_region);
                self.objects = s3_objects.clone();
                self.loading = false;
                Ok(s3_objects)
            },
            Err(err) => {
                let sdk_error = err.into_service_error();
                let error_code = sdk_error.code().unwrap_or("Unknown");
                let error_message = sdk_error.message().unwrap_or("No error message");
                
                let error = format!("Failed to list objects in bucket {}: {} - {}", bucket, error_code, error_message);
                error!("{}", error);
                self.loading = false;
                Err(error)
            }
        }
    }
}

/// Format file size in human-readable format
pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if size < KB {
        format!("{} B", size)
    } else if size < MB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else if size < GB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else {
        format!("{:.2} GB", size as f64 / GB as f64)
    }
}
