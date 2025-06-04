use chrono::{/* DateTime, */ Utc, TimeZone};
use eframe::egui;
use std::sync::{Arc};
use log::{error, debug};
use std::collections::{HashMap, HashSet};
use tokio::sync::Mutex as TokioMutex;
use aws_sdk_s3::error::ProvideErrorMetadata;

use crate::aws::auth::AwsAuth;

/// Component for viewing and interacting with S3 buckets
#[derive(Default)]
pub struct BucketView {
    buckets: Vec<String>,
    objects: Vec<S3Object>,
    selected_bucket: Option<String>,
    selected_objects: HashSet<String>,
    filter: String,
    loading: bool,
    #[allow(dead_code)] // Will be used in future implementations
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
    /// Create a new bucket view
    #[allow(dead_code)] // Will be used in future implementations
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set an error message
    #[allow(dead_code)] // Will be used in future implementations
    pub fn set_error(&mut self, message: String) {
        error!("Bucket view error: {}", message);
        self.error_message = Some(message);
        self.loading = false;
    }
    
    /// Clear the error message
    #[allow(dead_code)] // Will be used in future implementations
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }
    
    /// Get the error message
    #[allow(dead_code)] // Will be used in future implementations
    pub fn error_message(&self) -> Option<&String> {
        self.error_message.as_ref()
    }
    
    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        debug!("Setting loading state: {}", loading);
        self.loading = loading;
    }
    
    /// Check if the view is in loading state
    #[allow(dead_code)] // Will be used in future implementations
    pub fn is_loading(&self) -> bool {
        self.loading
    }
    
    /// Get the selected bucket
    pub fn selected_bucket(&self) -> Option<String> {
        self.selected_bucket.clone()
    }
    
    /// Set the list of buckets
    pub fn set_buckets(&mut self, buckets: Vec<String>) {
        self.buckets = buckets;
    }
    
    /// Render the bucket view UI and return true if selection changed
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut selection_changed = false;
        
        // Filter input
        ui.horizontal(|ui| {
            ui.label("Filter:");
            if ui.text_edit_singleline(self.filter_mut()).changed() {
                // Filter changed
            }
        });
        
        // Bucket dropdown
        ui.horizontal(|ui| {
            ui.label("Select bucket:");
            
            egui::ComboBox::from_id_source("bucket_selector")
                .selected_text(self.selected_bucket.as_deref().unwrap_or("Select a bucket"))
                .show_ui(ui, |ui| {
                    for bucket in &self.buckets {
                        let is_selected = self.selected_bucket.as_ref() == Some(bucket);
                        if ui.selectable_label(is_selected, bucket).clicked() {
                            if !is_selected {
                                self.selected_bucket = Some(bucket.clone());
                                selection_changed = true;
                            }
                        }
                    }
                });
        });
        
        // Bucket list (as a fallback/alternative view)
        // egui::ScrollArea::vertical().show(ui, |ui| {
        //     for bucket in &self.buckets {
        //         let is_selected = self.selected_bucket.as_ref() == Some(bucket);
        //         if ui.selectable_label(is_selected, bucket).clicked() {
        //             if !is_selected {
        //                 self.selected_bucket = Some(bucket.clone());
        //                 selection_changed = true;
        //             }
        //         }
        //     }
        // });
        
        // Show loading indicator if loading
        if self.loading {
            ui.add(egui::Spinner::new());
        }
        
        selection_changed
    }
    
    /// Get a mutable reference to the selected bucket
    #[allow(dead_code)] // Will be used in future implementations
    pub fn selected_bucket_mut(&mut self) -> &mut Option<String> {
        &mut self.selected_bucket
    }
    
    /// Get the filter string
    pub fn filter(&self) -> &str {
        &self.filter
    }
    
    /// Set the filter string
    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
    }
    
    /// Get the filter string as an Option
    pub fn get_filter(&self) -> Option<&String> {
        if self.filter.is_empty() {
            None
        } else {
            Some(&self.filter)
        }
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
    
    /// Set the objects for the current bucket
    pub fn set_objects(&mut self, objects: Vec<S3Object>) {
        self.objects = objects;
        self.selected_objects.clear();
        // Reset loading state when objects are set
        self.loading = false;
    }
    
    /// Toggle selection of an object
    pub fn toggle_object_selection(&mut self, key: &str) {
        if self.selected_objects.contains(key) {
            self.selected_objects.remove(key);
        } else {
            self.selected_objects.insert(key.to_string());
        }
    }
    
    /// Check if an object is selected
    pub fn is_object_selected(&self, key: &str) -> bool {
        self.selected_objects.contains(key)
    }
    
    /// Get the selected objects
    pub fn selected_objects(&self) -> Vec<&S3Object> {
        self.objects.iter()
            .filter(|obj| self.selected_objects.contains(&obj.key))
            .collect()
    }
    
    /// Select all visible objects (those that match the current filter)
    pub fn select_all_visible(&mut self) {
        let filter = self.filter.to_lowercase();
        
        for obj in &self.objects {
            if filter.is_empty() || obj.key.to_lowercase().contains(&filter) {
                self.selected_objects.insert(obj.key.clone());
            }
        }
    }
    
    /// Clear all selections
    pub fn clear_selection(&mut self) {
        self.selected_objects.clear();
    }
    
    /// Get the number of objects
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }
    
    /// Load buckets from AWS
    pub async fn load_buckets(&mut self, aws_auth: Arc<TokioMutex<AwsAuth>>) -> Result<Vec<String>, String> {
        debug!("Loading buckets from AWS");
        self.loading = true;
        
        // Get the client
        let client = {
            let mut auth = aws_auth.lock().await;
            match auth.get_client().await {
                Ok(client) => client.clone(),
                Err(e) => {
                    let error = format!("Failed to get AWS client: {}", e);
                    error!("{}", error);
                    self.loading = false;
                    return Err(error);
                }
            }
        };
        
        // List buckets
        match client.list_buckets().send().await {
            Ok(resp) => {
                let bucket_names: Vec<String> = resp.buckets()
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|b| b.name().map(|s| s.to_string()))
                    .collect();
                    
                debug!("Found {} buckets", bucket_names.len());
                
                // Get the region for each bucket
                for bucket in &bucket_names {
                    if !self.bucket_regions.contains_key(bucket) {
                        match self.get_bucket_location(&client, bucket).await {
                            Ok(region) => {
                                debug!("Bucket {} is in region {}", bucket, region);
                                self.bucket_regions.insert(bucket.to_string(), region);
                            },
                            Err(e) => {
                                error!("Failed to get region for bucket {}: {}", bucket, e);
                                // Default to us-east-1
                                self.bucket_regions.insert(bucket.to_string(), "us-east-1".to_string());
                            }
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
    
    // Load objects from the selected bucket
    pub async fn load_objects(&mut self, aws_auth: Arc<TokioMutex<AwsAuth>>, bucket: &str) -> Result<Vec<S3Object>, String> {
        debug!("Loading objects from bucket: {}", bucket);
        self.loading = true;
        
        // Get the region for this bucket
        let bucket_region = match self.get_bucket_region(bucket) {
            Some(region) => region.clone(),
            None => {
                // Try to get the region
                let client = {
                    let mut auth = aws_auth.lock().await;
                    match auth.get_client().await {
                        Ok(client) => client.clone(),
                        Err(e) => {
                            let error = format!("Failed to get AWS client: {}", e);
                            error!("{}", error);
                            self.loading = false;
                            return Err(error);
                        }
                    }
                };
                
                match self.get_bucket_location(&client, bucket).await {
                    Ok(region) => {
                        debug!("Bucket {} is in region {}", bucket, region);
                        self.bucket_regions.insert(bucket.to_string(), region.clone());
                        region
                    },
                    Err(e) => {
                        error!("Failed to get region for bucket {}: {}", bucket, e);
                        // Default to us-east-1
                        "us-east-1".to_string()
                    }
                }
            }
        };
        
        // Get a client for the specific region
        let client = {
            let mut auth = aws_auth.lock().await;
            match auth.get_client_for_region(&bucket_region).await {
                Ok(client) => client,
                Err(e) => {
                    let error = format!("Failed to get AWS client for region {}: {}", bucket_region, e);
                    error!("{}", error);
                    self.loading = false;
                    return Err(error);
                }
            }
        };
        
        // List objects
        match client.list_objects_v2().bucket(bucket).send().await {
            Ok(resp) => {
                let mut s3_objects = Vec::new();
                let mut directories = HashSet::new();
                
                // Process the objects
                for obj in resp.contents().unwrap_or_default() {
                    let key = obj.key().unwrap_or_default();
                    let size = obj.size() as u64;
                    println!("{:?}",obj.last_modified());
                    // Get the last modified timestamp
                    let last_modified = obj.last_modified()
                        .map(|dt| {
                            // Format the date in a human-readable format
                            // Extract the timestamp from the debug representation
                            let dt_str = format!("{:?}", dt);
                            println!("{:?}",dt_str);
                            
                            // Parse the seconds from the debug format: DateTime{seconds:1747860201,subseconds_nanos:0}
                            if let Some(start) = dt_str.find("seconds:") {
                                let seconds_str = &dt_str[start + 8..]; // Skip "seconds:"
                                if let Some(end) = seconds_str.find(',') {
                                    // Extract just the seconds value
                                    if let Ok(seconds) = seconds_str[..end].parse::<i64>() {
                                        // Convert Unix timestamp to DateTime and format
                                        if let Some(datetime) = Utc.timestamp_opt(seconds, 0).single() {
                                            return datetime.format("%Y-%m-%d %H:%M:%S").to_string();
                                        }
                                    }
                                }
                            }
                            
                            // Fallback if parsing fails
                            dt_str
                        })
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
                
                // Add any directories that weren't already added
                for dir in directories {
                    if !s3_objects.iter().any(|obj| obj.key == dir) {
                        s3_objects.push(S3Object {
                            key: dir,
                            size: 0,
                            last_modified: "".to_string(),
                            is_directory: true,
                        });
                    }
                }
                
                // Sort objects: directories first, then by name
                s3_objects.sort_by(|a, b| {
                    match (a.is_directory, b.is_directory) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.key.cmp(&b.key),
                    }
                });
                
                debug!("Found {} objects in bucket {}", s3_objects.len(), bucket);
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
    
    /// Get the location (region) of a bucket
    async fn get_bucket_location(&self, client: &aws_sdk_s3::Client, bucket: &str) -> Result<String, String> {
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
                let sdk_error = err.into_service_error();
                let error_code = sdk_error.code().unwrap_or("Unknown");
                let error_message = sdk_error.message().unwrap_or("No error message");
                
                Err(format!("Failed to get bucket location: {} - {}", error_code, error_message))
            }
        }
    }
}
