use anyhow::{anyhow, Result};
use log::{debug, error, info};

use super::auth::AwsAuth;

#[allow(dead_code)]
pub struct BucketManager {
    auth: AwsAuth,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct S3ObjectInfo {
    pub key: String,
    pub size: i64,
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
    pub etag: Option<String>,
}

impl BucketManager {
    #[allow(dead_code)]
    pub fn new(auth: AwsAuth) -> Self {
        Self { auth }
    }
    
    #[allow(dead_code)]
    pub async fn list_buckets(&mut self) -> Result<Vec<String>> {
        let client = self.auth.get_client().await?;
        match client.list_buckets().send().await {
            Ok(resp) => {
                let bucket_names: Vec<String> = resp.buckets()
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
    
    #[allow(dead_code)]
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
    
    #[allow(dead_code)]
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
    
    #[allow(dead_code)]
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
