use chrono::{Utc, TimeZone};
use eframe::egui;
use std::sync::Arc;
use log::{error, debug};
use std::collections::{HashMap, HashSet};
use tokio::sync::Mutex as TokioMutex;

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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn new() -> Self { Self::default() }
    
    #[allow(dead_code)]
    pub fn set_error(&mut self, message: String) {
        error!("Bucket view error: {}", message);
        self.error_message = Some(message);
        self.loading = false;
    }
    
    #[allow(dead_code)]
    pub fn clear_error(&mut self) { self.error_message = None; }
    
    #[allow(dead_code)]
    pub fn error_message(&self) -> Option<&String> { self.error_message.as_ref() }
    
    pub fn set_loading(&mut self, loading: bool) {
        debug!("Setting loading state: {}", loading);
        self.loading = loading;
    }
    
    #[allow(dead_code)]
    pub fn is_loading(&self) -> bool { self.loading }
    
    pub fn selected_bucket(&self) -> Option<String> { self.selected_bucket.clone() }
    
    pub fn set_buckets(&mut self, buckets: Vec<String>) { self.buckets = buckets; }
    
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut selection_changed = false;
        
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(self.filter_mut());
        });
        
        ui.horizontal(|ui| {
            ui.label("Select bucket:");
            egui::ComboBox::from_id_salt("bucket_selector")
                .selected_text(self.selected_bucket.as_deref().unwrap_or("Select a bucket"))
                .show_ui(ui, |ui| {
                    for bucket in &self.buckets {
                        let is_selected = self.selected_bucket.as_ref() == Some(bucket);
                        if ui.selectable_label(is_selected, bucket).clicked() && !is_selected {
                            self.selected_bucket = Some(bucket.clone());
                            selection_changed = true;
                        }
                    }
                });
        });
        
        if self.loading {
            ui.add(egui::Spinner::new());
        }
        
        selection_changed
    }
    
    #[allow(dead_code)]
    pub fn selected_bucket_mut(&mut self) -> &mut Option<String> { &mut self.selected_bucket }
    
    pub fn filter(&self) -> &str { &self.filter }
    
    pub fn set_filter(&mut self, filter: String) { self.filter = filter; }
    
    pub fn get_filter(&self) -> Option<&String> {
        if self.filter.is_empty() { None } else { Some(&self.filter) }
    }
    
    pub fn filter_mut(&mut self) -> &mut String { &mut self.filter }
    
    pub fn clear_filter(&mut self) { self.filter.clear(); }
    
    pub fn buckets(&self) -> &[String] { &self.buckets }
    
    pub fn objects(&self) -> &[S3Object] { &self.objects }
    
    pub fn get_bucket_region(&self, bucket: &str) -> Option<&String> {
        self.bucket_regions.get(bucket)
    }
    
    pub fn set_objects(&mut self, objects: Vec<S3Object>) {
        self.objects = objects;
        self.selected_objects.clear();
        self.loading = false;
    }
    
    pub fn toggle_object_selection(&mut self, key: &str) {
        if self.selected_objects.contains(key) {
            self.selected_objects.remove(key);
        } else {
            self.selected_objects.insert(key.to_string());
        }
    }
    
    pub fn is_object_selected(&self, key: &str) -> bool { self.selected_objects.contains(key) }
    
    pub fn selected_objects(&self) -> Vec<&S3Object> {
        self.objects.iter()
            .filter(|obj| self.selected_objects.contains(&obj.key))
            .collect()
    }
    
    pub fn select_all_visible(&mut self) {
        let filter = self.filter.to_lowercase();
        for obj in &self.objects {
            if filter.is_empty() || obj.key.to_lowercase().contains(&filter) {
                self.selected_objects.insert(obj.key.clone());
            }
        }
    }
    
    pub fn clear_selection(&mut self) { self.selected_objects.clear(); }
    
    pub fn object_count(&self) -> usize { self.objects.len() }
    
    pub async fn load_buckets(&mut self, aws_auth: Arc<TokioMutex<AwsAuth>>) -> Result<Vec<String>, String> {
        debug!("Loading buckets from AWS");
        self.loading = true;
        
        let client = {
            let mut auth = aws_auth.lock().await;
            match auth.get_client().await {
                Ok(client) => client.clone(),
                Err(e) => {
                    self.loading = false;
                    return Err(format!("Failed to get AWS client: {}", e));
                }
            }
        };
        
        match client.list_buckets().send().await {
            Ok(resp) => {
                let bucket_names: Vec<String> = resp.buckets()
                    .iter()
                    .filter_map(|b| b.name().map(|s| s.to_string()))
                    .collect();
                    
                debug!("Found {} buckets", bucket_names.len());
                
                for bucket in &bucket_names {
                    if !self.bucket_regions.contains_key(bucket) {
                        match self.get_bucket_location(&client, bucket).await {
                            Ok(region) => {
                                debug!("Bucket {} is in region {}", bucket, region);
                                self.bucket_regions.insert(bucket.to_string(), region);
                            },
                            Err(e) => {
                                error!("Failed to get region for bucket {}: {}", bucket, e);
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
                let error = format!("Failed to list buckets: {}", err);
                error!("{}", error);
                self.loading = false;
                Err(error)
            }
        }
    }
    
    pub async fn load_objects(&mut self, aws_auth: Arc<TokioMutex<AwsAuth>>, bucket: &str) -> Result<Vec<S3Object>, String> {
        debug!("Loading objects from bucket: {}", bucket);
        self.loading = true;
        
        let bucket_region = match self.get_bucket_region(bucket) {
            Some(region) => region.clone(),
            None => {
                let client = {
                    let mut auth = aws_auth.lock().await;
                    match auth.get_client().await {
                        Ok(client) => client.clone(),
                        Err(e) => {
                            self.loading = false;
                            return Err(format!("Failed to get AWS client: {}", e));
                        }
                    }
                };
                match self.get_bucket_location(&client, bucket).await {
                    Ok(region) => {
                        self.bucket_regions.insert(bucket.to_string(), region.clone());
                        region
                    },
                    Err(_) => "us-east-1".to_string(),
                }
            }
        };
        
        let client = {
            let mut auth = aws_auth.lock().await;
            match auth.get_client_for_region(&bucket_region).await {
                Ok(client) => client,
                Err(e) => {
                    self.loading = false;
                    return Err(format!("Failed to get AWS client for region {}: {}", bucket_region, e));
                }
            }
        };
        
        match client.list_objects_v2().bucket(bucket).send().await {
            Ok(resp) => {
                let mut s3_objects = Vec::new();
                let mut directories = HashSet::new();
                
                for obj in resp.contents() {
                    let key = obj.key().unwrap_or_default();
                    let size = obj.size().unwrap_or(0) as u64;
                    let last_modified = obj.last_modified()
                        .map(|dt| {
                            Utc.timestamp_opt(dt.secs(), 0)
                                .single()
                                .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or_default()
                        })
                        .unwrap_or_else(|| "Unknown".to_string());
                    
                    if key.ends_with('/') {
                        directories.insert(key.to_string());
                        s3_objects.push(S3Object {
                            key: key.to_string(), size: 0, last_modified, is_directory: true,
                        });
                    } else {
                        let mut path_parts = key.split('/').collect::<Vec<_>>();
                        if path_parts.len() > 1 {
                            path_parts.pop();
                            directories.insert(path_parts.join("/") + "/");
                        }
                        s3_objects.push(S3Object {
                            key: key.to_string(), size, last_modified, is_directory: false,
                        });
                    }
                }
                
                for dir in directories {
                    if !s3_objects.iter().any(|obj| obj.key == dir) {
                        s3_objects.push(S3Object {
                            key: dir, size: 0, last_modified: String::new(), is_directory: true,
                        });
                    }
                }
                
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
                let error = format!("Failed to list objects in bucket {}: {}", bucket, err);
                error!("{}", error);
                self.loading = false;
                Err(error)
            }
        }
    }
    
    async fn get_bucket_location(&self, client: &aws_sdk_s3::Client, bucket: &str) -> Result<String, String> {
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
}
