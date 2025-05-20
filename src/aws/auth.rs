use anyhow::{anyhow, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;
use aws_types::region::Region;
use log::{error, info};

/// AWS authentication manager
#[derive(Clone)]
pub struct AwsAuth {
    client: Option<Client>,
    region: String,
    access_key: String,
    secret_key: String,
}

impl Default for AwsAuth {
    fn default() -> Self {
        Self {
            client: None,
            region: "us-east-1".to_string(),
            access_key: String::new(),
            secret_key: String::new(),
        }
    }
}

impl AwsAuth {
    /// Create a new AWS authentication manager with the specified credentials
    pub fn new(access_key: String, secret_key: String, region: String) -> Self {
        Self {
            client: None,
            region,
            access_key,
            secret_key,
        }
    }
    
    /// Initialize the AWS client with the current credentials
    pub async fn initialize(&mut self) -> Result<()> {
        if self.access_key.is_empty() || self.secret_key.is_empty() {
            return Err(anyhow!("AWS credentials not provided"));
        }
        
        let region_provider = RegionProviderChain::first_try(Region::new(self.region.clone()))
            .or_default_provider();
            
        let provider = aws_sdk_s3::Credentials::new(
            &self.access_key,
            &self.secret_key,
            None,
            None,
            "s3sync-app",
        );
            
        let config = aws_config::from_env()
            .region(region_provider)
            .credentials_provider(provider)
            .load()
            .await;
            
        self.client = Some(Client::new(&config));
        
        info!("AWS S3 client initialized for region {}", self.region);
        Ok(())
    }
    
    /// Get the S3 client, initializing if necessary
    pub async fn get_client(&mut self) -> Result<&Client> {
        if self.client.is_none() {
            self.initialize().await?;
        }
        
        self.client.as_ref().ok_or_else(|| anyhow!("Failed to initialize AWS client"))
    }
    
    /// Test the AWS credentials by listing buckets
    pub async fn test_credentials(&mut self) -> Result<bool> {
        let client = self.get_client().await?;
        
        match client.list_buckets().send().await {
            Ok(_) => {
                info!("AWS credentials validated successfully");
                Ok(true)
            },
            Err(err) => {
                error!("AWS credential validation failed: {}", err);
                Err(anyhow!("AWS credential validation failed: {}", err))
            }
        }
    }
    
    /// Update the AWS credentials
    pub fn update_credentials(&mut self, access_key: String, secret_key: String, region: String) {
        self.access_key = access_key;
        self.secret_key = secret_key;
        self.region = region;
        self.client = None; // Force re-initialization with new credentials
    }
}
