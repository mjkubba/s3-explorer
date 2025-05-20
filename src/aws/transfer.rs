use anyhow::{anyhow, Result};
use aws_sdk_s3::output::GetObjectOutput;
use log::{debug, error, info};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use super::auth::AwsAuth;

/// S3 file transfer manager
pub struct TransferManager {
    auth: AwsAuth,
}

/// Transfer progress information
pub struct TransferProgress {
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
        progress_callback: Option<&dyn Fn(TransferProgress)>,
    ) -> Result<()> {
        let client = self.auth.get_client().await?;
        
        // Read file metadata
        let metadata = tokio::fs::metadata(local_path).await?;
        let file_size = metadata.len();
        
        // Read file content
        let body = tokio::fs::read(local_path).await?;
        
        // Upload the file
        match client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(body.into())
            .send()
            .await
        {
            Ok(_) => {
                info!("Uploaded {} to {}/{}", local_path.display(), bucket, key);
                
                // Call progress callback with 100% completion
                if let Some(callback) = progress_callback {
                    callback(TransferProgress {
                        bytes_transferred: file_size,
                        total_bytes: file_size,
                        percentage: 100.0,
                    });
                }
                
                Ok(())
            },
            Err(err) => {
                error!("Failed to upload {}: {}", local_path.display(), err);
                Err(anyhow!("Failed to upload {}: {}", local_path.display(), err))
            }
        }
    }
    
    /// Download a file from S3
    pub async fn download_file(
        &mut self,
        bucket: &str,
        key: &str,
        local_path: &Path,
        progress_callback: Option<&dyn Fn(TransferProgress)>,
    ) -> Result<()> {
        let client = self.auth.get_client().await?;
        
        // Create parent directories if they don't exist
        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // Get the object
        let resp = client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await?;
            
        let total_size = resp.content_length() as u64;
        
        // Process the response
        self.process_download_response(resp, local_path, total_size, progress_callback).await?;
        
        info!("Downloaded {}/{} to {}", bucket, key, local_path.display());
        Ok(())
    }
    
    /// Process the download response and write to file
    async fn process_download_response(
        &self,
        resp: GetObjectOutput,
        local_path: &Path,
        total_size: u64,
        progress_callback: Option<&dyn Fn(TransferProgress)>,
    ) -> Result<()> {
        // Create the file
        let mut file = File::create(local_path).await?;
        
        // Get the body as bytes
        let body = resp.body.collect().await?;
        let bytes = body.into_bytes();
        let bytes_len = bytes.len() as u64;
        
        // Write the bytes to the file
        file.write_all(&bytes).await?;
        
        // Update progress
        if let Some(callback) = progress_callback {
            callback(TransferProgress {
                bytes_transferred: bytes_len,
                total_bytes: total_size,
                percentage: (bytes_len as f32 / total_size as f32) * 100.0,
            });
        }
        
        Ok(())
    }
    
    /// Check if a file exists in S3
    pub async fn object_exists(&mut self, bucket: &str, key: &str) -> Result<bool> {
        let client = self.auth.get_client().await?;
        
        match client.head_object().bucket(bucket).key(key).send().await {
            Ok(_) => {
                debug!("Object {}/{} exists", bucket, key);
                Ok(true)
            },
            Err(_) => {
                debug!("Object {}/{} does not exist", bucket, key);
                Ok(false)
            }
        }
    }
    
    /// Get object metadata
    pub async fn get_object_metadata(
        &mut self,
        bucket: &str,
        key: &str,
    ) -> Result<Option<aws_sdk_s3::output::HeadObjectOutput>> {
        let client = self.auth.get_client().await?;
        
        match client.head_object().bucket(bucket).key(key).send().await {
            Ok(metadata) => {
                debug!("Retrieved metadata for {}/{}", bucket, key);
                Ok(Some(metadata))
            },
            Err(_) => {
                debug!("Could not retrieve metadata for {}/{}", bucket, key);
                Ok(None)
            }
        }
    }
}
