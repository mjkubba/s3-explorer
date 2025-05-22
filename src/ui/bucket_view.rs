use eframe::egui;
use std::sync::{Arc, Mutex};
use log::{info, error, debug};

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
}

/// Represents an object in an S3 bucket
#[derive(Clone)]
struct S3Object {
    key: String,
    size: u64,
    last_modified: String,
    is_directory: bool,
}

impl BucketView {
    /// Render the bucket view UI
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("S3 Buckets");
        
        // Show loading indicator or error message
        if self.loading {
            ui.horizontal(|ui| {
                ui.label("⏳ Loading buckets...");
            });
        } else if let Some(error) = &self.error_message {
            ui.colored_label(egui::Color32::RED, error);
            if ui.button("Clear Error").clicked() {
                self.error_message = None;
            }
        }
        
        // Bucket selection
        egui::ComboBox::from_label("Select Bucket")
            .selected_text(self.selected_bucket.as_deref().unwrap_or("No bucket selected"))
            .show_ui(ui, |ui| {
                for bucket in &self.buckets {
                    ui.selectable_value(&mut self.selected_bucket, Some(bucket.clone()), bucket);
                }
            });
        
        ui.separator();
        
        // Filter
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter);
            
            if ui.button("Clear").clicked() {
                self.filter.clear();
            }
        });
        
        // Object list
        ui.separator();
        
        if self.selected_bucket.is_some() {
            if ui.button("Load Objects").clicked() {
                // This will be handled by the parent component
                info!("Load objects requested for bucket: {}", self.selected_bucket.as_ref().unwrap());
            }
            
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
                for object in &self.objects {
                    if !self.filter.is_empty() && !object.key.contains(&self.filter) {
                        continue;
                    }
                    
                    ui.horizontal(|ui| {
                        let icon = if object.is_directory { "📁 " } else { "📄 " };
                        ui.label(format!("{}{}", icon, object.key));
                        ui.add_space(200.0 - object.key.len() as f32 * 7.0);
                        
                        let size_str = if object.is_directory {
                            "-".to_string()
                        } else {
                            format_size(object.size)
                        };
                        
                        ui.label(size_str);
                        ui.add_space(100.0);
                        ui.label(&object.last_modified);
                    });
                    
                    ui.separator();
                }
            });
        } else {
            ui.label("Select a bucket to view objects");
        }
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
    pub async fn load_objects(&mut self, aws_auth: Arc<Mutex<AwsAuth>>, bucket: &str) -> Result<(), String> {
        debug!("Loading objects from bucket: {}", bucket);
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
            
            // Clone the auth object
            auth_guard.clone()
        };
        
        // Now use the cloned auth object
        debug!("Getting AWS client for object listing");
        let client = match auth_clone.get_client().await {
            Ok(client) => client,
            Err(e) => {
                let error = format!("Failed to get AWS client: {}", e);
                error!("{}", error);
                self.loading = false;
                return Err(error);
            }
        };
        
        debug!("Sending list_objects_v2 request for bucket: {}", bucket);
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
                
                info!("Listed {} objects in bucket {}", s3_objects.len(), bucket);
                self.objects = s3_objects;
                self.loading = false;
                Ok(())
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
fn format_size(size: u64) -> String {
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
