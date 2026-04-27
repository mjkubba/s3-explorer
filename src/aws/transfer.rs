use anyhow::{anyhow, Result};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use log::debug;
use log::error;
use std::path::Path;
use std::sync::Arc;
use std::fs;
use tokio::io::AsyncWriteExt;
use chrono::{Utc, TimeZone};

use crate::aws::s3::S3ErrorHelper;

/// Progress information for a file transfer
#[derive(Debug, Clone)]
pub struct TransferProgress {
    pub file_name: String,
    pub bytes_transferred: u64,
    #[allow(dead_code)]
    pub total_bytes: u64,
    pub percentage: f32,
}

/// Manager for S3 file transfers
#[derive(Clone)]
pub struct TransferManager {
    client: Arc<Client>,
}

impl TransferManager {
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }
    
    pub async fn list_buckets(&self) -> Result<Vec<String>> {
        debug!("Listing S3 buckets");
        let resp = self.client.list_buckets().send().await?;
        let buckets = resp.buckets()
            .iter()
            .filter_map(|b| b.name().map(|s| s.to_string()))
            .collect();
        Ok(buckets)
    }
    
    pub async fn list_objects(&self, bucket: &str) -> Result<Vec<crate::ui::bucket_view::S3Object>> {
        debug!("Listing objects in bucket {}", bucket);
        
        let mut objects = Vec::new();
        let mut continuation_token = None;
        
        loop {
            let mut req = self.client.list_objects_v2()
                .bucket(bucket)
                .delimiter("/");
                
            if let Some(token) = &continuation_token {
                req = req.continuation_token(token);
            }
            
            let resp = match req.send().await {
                Ok(response) => response,
                Err(e) => {
                    let detailed_error = S3ErrorHelper::extract_error_details(&e);
                    error!("Failed to list objects in bucket {}: {}", bucket, detailed_error);
                    return Err(anyhow!("S3 service error: {}", detailed_error));
                }
            };
            
            // Process common prefixes (directories)
            for prefix in resp.common_prefixes() {
                if let Some(prefix_str) = prefix.prefix() {
                    let key = prefix_str.trim_end_matches('/').to_string();
                    objects.push(crate::ui::bucket_view::S3Object {
                        key,
                        size: 0,
                        last_modified: String::new(),
                        is_directory: true,
                    });
                }
            }
            
            // Process objects (files)
            for object in resp.contents() {
                let key = object.key().unwrap_or_default().to_string();
                let size = object.size().unwrap_or(0) as u64;
                let last_modified = object.last_modified()
                    .map(|dt| {
                        Utc.timestamp_opt(dt.secs(), 0)
                            .single()
                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                    
                objects.push(crate::ui::bucket_view::S3Object {
                    key,
                    size,
                    last_modified,
                    is_directory: false,
                });
            }
            
            // Check if there are more objects
            if resp.is_truncated() == Some(true) && resp.next_continuation_token().is_some() {
                continuation_token = resp.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }
        
        Ok(objects)
    }
    
    pub async fn upload_file(
        &self,
        local_path: &Path,
        bucket: &str,
        s3_key: &str,
        progress_callback: Option<Box<dyn Fn(TransferProgress) + Send + Sync>>,
    ) -> Result<()> {
        debug!("Uploading {} to s3://{}/{}", local_path.display(), bucket, s3_key);
        
        let metadata = fs::metadata(local_path)?;
        let total_size = metadata.len();
        
        let file_name = local_path.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| s3_key.to_string());
            
        let body = ByteStream::from_path(local_path).await?;
        
        let resp = self.client.put_object()
            .bucket(bucket)
            .key(s3_key)
            .body(body)
            .send()
            .await?;
            
        debug!("Upload complete: {:?}", resp);
        
        if let Some(callback) = progress_callback {
            callback(TransferProgress {
                file_name,
                bytes_transferred: total_size,
                total_bytes: total_size,
                percentage: 100.0,
            });
        }
        
        Ok(())
    }
    
    pub async fn download_file(
        &self,
        bucket: &str,
        s3_key: &str,
        local_path: &Path,
        progress_callback: Option<Box<dyn Fn(TransferProgress) + Send + Sync>>,
    ) -> Result<()> {
        debug!("Downloading s3://{}/{} to {}", bucket, s3_key, local_path.display());
        
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let resp = self.client.get_object()
            .bucket(bucket)
            .key(s3_key)
            .send()
            .await?;
            
        let total_size = resp.content_length().unwrap_or(0) as u64;
        
        let file_name = local_path.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| s3_key.to_string());
            
        let mut file = tokio::fs::File::create(local_path).await?;
        
        let mut stream = resp.body.into_async_read();
        let mut bytes_read = 0u64;
        let mut buffer = vec![0u8; 8192];
        
        loop {
            let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buffer).await?;
            if n == 0 {
                break;
            }
            
            file.write_all(&buffer[..n]).await?;
            bytes_read += n as u64;
            
            if let Some(callback) = &progress_callback {
                let percentage = if total_size > 0 {
                    (bytes_read as f32 / total_size as f32) * 100.0
                } else {
                    0.0
                };
                callback(TransferProgress {
                    file_name: file_name.clone(),
                    bytes_transferred: bytes_read,
                    total_bytes: total_size,
                    percentage,
                });
            }
        }
        
        file.flush().await?;
        debug!("Download complete");
        Ok(())
    }
    
    #[allow(dead_code)]
    pub async fn delete_object(&self, bucket: &str, s3_key: &str) -> Result<()> {
        debug!("Deleting object: s3://{}/{}", bucket, s3_key);
        self.client.delete_object()
            .bucket(bucket)
            .key(s3_key)
            .send()
            .await?;
        debug!("Object deleted");
        Ok(())
    }
    
    #[allow(dead_code)]
    pub async fn object_exists(&self, bucket: &str, s3_key: &str) -> Result<bool> {
        debug!("Checking if object exists: s3://{}/{}", bucket, s3_key);
        match self.client.head_object()
            .bucket(bucket)
            .key(s3_key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("404") {
                    Ok(false)
                } else {
                    Err(anyhow!("Failed to check if object exists: {}", e))
                }
            }
        }
    }
    
    #[allow(dead_code)]
    pub async fn get_object_size(&self, bucket: &str, s3_key: &str) -> Result<u64> {
        debug!("Getting size of object: s3://{}/{}", bucket, s3_key);
        let resp = self.client.head_object()
            .bucket(bucket)
            .key(s3_key)
            .send()
            .await?;
        Ok(resp.content_length().unwrap_or(0) as u64)
    }
    
    #[allow(dead_code)]
    pub async fn get_object_etag(&self, bucket: &str, s3_key: &str) -> Result<String> {
        debug!("Getting ETag of object: s3://{}/{}", bucket, s3_key);
        let resp = self.client.head_object()
            .bucket(bucket)
            .key(s3_key)
            .send()
            .await?;
        resp.e_tag()
            .map(|s| s.trim_matches('"').to_string())
            .ok_or_else(|| anyhow!("ETag not found for object"))
    }
}
