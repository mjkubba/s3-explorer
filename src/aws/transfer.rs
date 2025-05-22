use anyhow::{anyhow, Result};
use aws_sdk_s3::Client;
use aws_sdk_s3::types::ByteStream;
use log::{debug, error, info};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Write;
use tokio::io::AsyncReadExt;

use crate::aws::auth::AwsAuth;

/// Manager for transferring files to and from S3
pub struct TransferManager {
    auth: AwsAuth,
}

/// Progress information for a transfer
#[derive(Clone)]
pub struct TransferProgress {
    pub file_name: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub percentage: f32,
}

impl TransferManager {
    /// Create a new transfer manager with the given authentication
    pub fn new(auth: AwsAuth) -> Self {
        Self { auth }
    }
    
    /// Upload a file to S3
    pub async fn upload_file(
        &mut self,
        local_path: &Path,
        bucket: &str,
        key: &str,
        progress_callback: Option<Box<dyn Fn(TransferProgress) + Send + Sync>>,
    ) -> Result<()> {
        debug!("Uploading file {} to s3://{}/{}", local_path.display(), bucket, key);
        
        // Get the region for this bucket
        let region = match self.auth.get_bucket_location(bucket).await {
            Ok(region) => region,
            Err(e) => {
                error!("Failed to get region for bucket {}: {}", bucket, e);
                // Default to the current region if we can't get the bucket location
                self.auth.region().to_string()
            }
        };
        
        // Get a client for the specific region
        let client = self.auth.get_client_for_region(&region).await?;
        
        // Read the file
        let file = File::open(local_path)?;
        let metadata = file.metadata()?;
        let file_size = metadata.len();
        
        // Create a byte stream from the file
        let body = ByteStream::from_path(local_path).await?;
        
        // Upload the file
        let result = client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(body)
            .send()
            .await;
            
        match result {
            Ok(_) => {
                info!("Successfully uploaded {} to s3://{}/{}", local_path.display(), bucket, key);
                
                // Call the progress callback with 100% completion
                if let Some(callback) = progress_callback {
                    callback(TransferProgress {
                        file_name: local_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                        bytes_transferred: file_size,
                        total_bytes: file_size,
                        percentage: 100.0,
                    });
                }
                
                Ok(())
            },
            Err(e) => {
                error!("Failed to upload {} to s3://{}/{}: {}", local_path.display(), bucket, key, e);
                Err(anyhow!("Failed to upload file: {}", e))
            }
        }
    }
    
    /// Download a file from S3
    pub async fn download_file(
        &mut self,
        bucket: &str,
        key: &str,
        local_path: &Path,
        progress_callback: Option<Box<dyn Fn(TransferProgress) + Send + Sync>>,
    ) -> Result<()> {
        debug!("Downloading s3://{}/{} to {}", bucket, key, local_path.display());
        
        // Get the region for this bucket
        let region = match self.auth.get_bucket_location(bucket).await {
            Ok(region) => region,
            Err(e) => {
                error!("Failed to get region for bucket {}: {}", bucket, e);
                // Default to the current region if we can't get the bucket location
                self.auth.region().to_string()
            }
        };
        
        // Get a client for the specific region
        let client = self.auth.get_client_for_region(&region).await?;
        
        // Get the object
        let result = client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await;
            
        match result {
            Ok(resp) => {
                // Create the parent directory if it doesn't exist
                if let Some(parent) = local_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                
                // Process the response and write to file
                self.process_download_response(resp, local_path, progress_callback).await?;
                
                info!("Successfully downloaded s3://{}/{} to {}", bucket, key, local_path.display());
                Ok(())
            },
            Err(e) => {
                error!("Failed to download s3://{}/{} to {}: {}", bucket, key, local_path.display(), e);
                Err(anyhow!("Failed to download file: {}", e))
            }
        }
    }
    
    async fn process_download_response(
        &self,
        resp: aws_sdk_s3::output::GetObjectOutput,
        local_path: &Path,
        progress_callback: Option<Box<dyn Fn(TransferProgress) + Send + Sync>>,
    ) -> Result<()> {
        let total_size = resp.content_length() as u64;
        let file_name = local_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        
        // Create the file
        let mut file = File::create(local_path)?;
        
        // Get the body as a stream
        let mut body = resp.body.into_async_read();
        
        // Read the stream in chunks and write to file
        let mut buffer = vec![0; 8192]; // 8KB buffer
        let mut bytes_read = 0;
        
        loop {
            let n = body.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            
            file.write_all(&buffer[..n])?;
            bytes_read += n as u64;
            
            // Update progress
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
        
        Ok(())
    }
    
    /// Check if an object exists in S3
    pub async fn object_exists(&mut self, bucket: &str, key: &str) -> Result<bool> {
        debug!("Checking if object s3://{}/{} exists", bucket, key);
        
        // Get the region for this bucket
        let region = match self.auth.get_bucket_location(bucket).await {
            Ok(region) => region,
            Err(e) => {
                error!("Failed to get region for bucket {}: {}", bucket, e);
                // Default to the current region if we can't get the bucket location
                self.auth.region().to_string()
            }
        };
        
        // Get a client for the specific region
        let client = self.auth.get_client_for_region(&region).await?;
        
        // Check if the object exists
        let result = client
            .head_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await;
            
        Ok(result.is_ok())
    }
    
    /// Get metadata for an object in S3
    pub async fn get_object_metadata(
        &mut self,
        bucket: &str,
        key: &str,
    ) -> Result<aws_sdk_s3::output::HeadObjectOutput> {
        debug!("Getting metadata for object s3://{}/{}", bucket, key);
        
        // Get the region for this bucket
        let region = match self.auth.get_bucket_location(bucket).await {
            Ok(region) => region,
            Err(e) => {
                error!("Failed to get region for bucket {}: {}", bucket, e);
                // Default to the current region if we can't get the bucket location
                self.auth.region().to_string()
            }
        };
        
        // Get a client for the specific region
        let client = self.auth.get_client_for_region(&region).await?;
        
        // Get the object metadata
        let result = client
            .head_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await?;
            
        Ok(result)
    }
}
