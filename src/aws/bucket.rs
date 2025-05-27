use anyhow::{anyhow, Result};
use log::{debug, error, info};

use super::auth::AwsAuth;

/// S3 bucket operations
pub struct BucketManager {
    auth: AwsAuth,
}

/// S3 object information
#[derive(Debug, Clone)]
pub struct S3ObjectInfo {
    pub key: String,
    pub size: i64,
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
    pub etag: Option<String>,
}

impl BucketManager {
    /// Create a new bucket manager with the given authentication
    pub fn new(auth: AwsAuth) -> Self {
        Self { auth }
    }
    
    /// List all available buckets
    pub async fn list_buckets(&mut self) -> Result<Vec<String>> {
        let client = self.auth.get_client().await?;
        
        match client.list_buckets().send().await {
            Ok(resp) => {
                let buckets = resp.buckets().unwrap_or_default();
                let bucket_names: Vec<String> = buckets
                    .iter()
                    .filter_map(|b| b.name().map(String::from))
                    .collect();
                    
                info!("Listed {} S3 buckets", bucket_names.len());
                Ok(bucket_names)
            },
            Err(err) => {
                error!("Failed to list buckets: {}", err);
                Err(anyhow!("Failed to list buckets: {}", err))
            }
        }
    }
    
    /// Create a new bucket
    pub async fn create_bucket(&mut self, bucket_name: &str) -> Result<()> {
        let client = self.auth.get_client().await?;
        
        match client.create_bucket().bucket(bucket_name).send().await {
            Ok(_) => {
                info!("Created bucket: {}", bucket_name);
                Ok(())
            },
            Err(err) => {
                error!("Failed to create bucket {}: {}", bucket_name, err);
                Err(anyhow!("Failed to create bucket {}: {}", bucket_name, err))
            }
        }
    }
    
    /// List objects in a bucket with optional prefix
    // pub async fn list_objects(&mut self, bucket: &str, prefix: Option<&str>) -> Result<Vec<S3ObjectInfo>> {
    //     let client = self.auth.get_client().await?;
        
    //     let mut request = client.list_objects_v2().bucket(bucket);
        
    //     if let Some(prefix) = prefix {
    //         request = request.prefix(prefix);
    //     }
        
    //     match request.send().await {
    //         Ok(resp) => {
    //             let objects = resp.contents().unwrap_or_default();
    //             let object_infos: Vec<S3ObjectInfo> = objects
    //                 .iter()
    //                 .map(|obj| {
    //                     // Convert AWS DateTime to chrono DateTime
    //                     let aws_dt = obj.last_modified();
    //                     let chrono_dt = aws_dt.map(|dt| {
    //                         let secs = dt.secs();
    //                         let nanos = dt.subsec_nanos();
    //                         chrono::DateTime::<chrono::Utc>::from_timestamp(secs as i64, nanos).unwrap_or_default()
    //                     });
                        
    //                     S3ObjectInfo {
    //                         key: obj.key().unwrap_or_default().to_string(),
    //                         size: obj.size(),
    //                         last_modified: chrono_dt,
    //                         etag: obj.e_tag().map(String::from),
    //                     }
    //                 })
    //                 .collect();
                    
    //             info!("Listed {} objects in bucket {}", object_infos.len(), bucket);
    //             Ok(object_infos)
    //         },
    //         Err(err) => {
    //             error!("Failed to list objects in bucket {}: {}", bucket, err);
    //             Err(anyhow!("Failed to list objects in bucket {}: {}", bucket, err))
    //         }
    //     }
    // }
    
    /// Delete an object from a bucket
    pub async fn delete_object(&mut self, bucket: &str, key: &str) -> Result<()> {
        let client = self.auth.get_client().await?;
        
        match client.delete_object().bucket(bucket).key(key).send().await {
            Ok(_) => {
                info!("Deleted object {}/{}", bucket, key);
                Ok(())
            },
            Err(err) => {
                error!("Failed to delete object {}/{}: {}", bucket, key, err);
                Err(anyhow!("Failed to delete object {}/{}: {}", bucket, key, err))
            }
        }
    }
    
    /// Check if a bucket exists
    pub async fn bucket_exists(&mut self, bucket: &str) -> Result<bool> {
        let client = self.auth.get_client().await?;
        
        match client.head_bucket().bucket(bucket).send().await {
            Ok(_) => {
                debug!("Bucket {} exists", bucket);
                Ok(true)
            },
            Err(_) => {
                debug!("Bucket {} does not exist", bucket);
                Ok(false)
            }
        }
    }
}
