use chrono::{DateTime, Utc};

/// Represents an object in an S3 bucket
#[derive(Debug, Clone)]
pub struct S3Object {
    /// The key (path) of the object in the bucket
    pub key: String,
    
    /// The size of the object in bytes
    pub size: u64,
    
    /// The last modified timestamp of the object
    pub last_modified: Option<DateTime<Utc>>,
    
    /// The ETag of the object (usually the MD5 hash)
    pub etag: Option<String>,
    
    /// The storage class of the object
    pub storage_class: Option<String>,
}

impl S3Object {
    /// Create a new S3 object
    pub fn new(key: String, size: u64) -> Self {
        Self {
            key,
            size,
            last_modified: None,
            etag: None,
            storage_class: None,
        }
    }
    
    /// Create a new S3 object with all fields
    pub fn with_details(
        key: String,
        size: u64,
        last_modified: Option<DateTime<Utc>>,
        etag: Option<String>,
        storage_class: Option<String>,
    ) -> Self {
        Self {
            key,
            size,
            last_modified,
            etag,
            storage_class,
        }
    }
    
    /// Get the filename part of the key
    pub fn filename(&self) -> String {
        self.key
            .split('/')
            .last()
            .unwrap_or(&self.key)
            .to_string()
    }
    
    /// Get the directory part of the key
    pub fn directory(&self) -> String {
        let parts: Vec<&str> = self.key.split('/').collect();
        if parts.len() <= 1 {
            return String::new();
        }
        
        parts[0..parts.len() - 1].join("/")
    }
    
    /// Format the size in a human-readable format
    pub fn formatted_size(&self) -> String {
        if self.size < 1024 {
            format!("{} B", self.size)
        } else if self.size < 1024 * 1024 {
            format!("{:.2} KB", self.size as f64 / 1024.0)
        } else if self.size < 1024 * 1024 * 1024 {
            format!("{:.2} MB", self.size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", self.size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }
    
    /// Format the last modified date in a human-readable format
    pub fn formatted_date(&self) -> String {
        match &self.last_modified {
            Some(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            None => "Unknown".to_string(),
        }
    }
}
