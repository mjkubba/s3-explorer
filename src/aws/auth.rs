use anyhow::{anyhow, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;
use aws_types::region::Region;
use log::{error, info, debug};
use std::env;

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
    
    /// Check if credentials are empty
    pub fn is_empty(&self) -> bool {
        self.access_key.is_empty() || self.secret_key.is_empty()
    }
    
    /// Get the current region
    pub fn region(&self) -> &str {
        &self.region
    }
    
    /// Initialize the AWS client with the current credentials
    pub async fn initialize(&mut self) -> Result<()> {
        if self.access_key.is_empty() || self.secret_key.is_empty() {
            return Err(anyhow!("AWS credentials not provided"));
        }
        
        debug!("Initializing AWS client with region: {}", self.region);
        debug!("Using access key ID: {}", self.access_key.chars().take(5).collect::<String>() + "...");
        
        // Set environment variables for AWS SDK
        env::set_var("AWS_ACCESS_KEY_ID", &self.access_key);
        env::set_var("AWS_SECRET_ACCESS_KEY", &self.secret_key);
        env::set_var("AWS_REGION", &self.region);
        
        let region_provider = RegionProviderChain::first_try(Region::new(self.region.clone()))
            .or_default_provider();
            
        debug!("Creating AWS SDK configuration");
        let config = aws_config::from_env()
            .region(region_provider)
            .load()
            .await;
            
        debug!("Creating S3 client");
        self.client = Some(Client::new(&config));
        
        info!("AWS S3 client initialized for region {}", self.region);
        Ok(())
    }
    
    /// Get the S3 client, initializing if necessary
    pub async fn get_client(&mut self) -> Result<&Client> {
        if self.client.is_none() {
            debug!("Client not initialized, initializing now");
            self.initialize().await?;
        }
        
        self.client.as_ref().ok_or_else(|| anyhow!("Failed to initialize AWS client"))
    }
    
    /// Test the AWS credentials by listing buckets
    pub async fn test_credentials(&mut self) -> Result<bool> {
        debug!("Testing AWS credentials");
        
        if self.is_empty() {
            return Err(anyhow!("AWS credentials not provided"));
        }
        
        let client = match self.get_client().await {
            Ok(client) => client,
            Err(e) => {
                error!("Failed to get AWS client: {}", e);
                return Err(anyhow!("Failed to initialize AWS client: {}", e));
            }
        };
        
        debug!("Sending list_buckets request to AWS");
        match client.list_buckets().send().await {
            Ok(resp) => {
                let bucket_count = resp.buckets().unwrap_or_default().len();
                info!("AWS credentials validated successfully. Found {} buckets", bucket_count);
                Ok(true)
            },
            Err(err) => {
                let sdk_error = err.into_service_error();
                let error_code = sdk_error.code().unwrap_or("Unknown");
                let error_message = sdk_error.message().unwrap_or("No error message");
                
                error!("AWS credential validation failed: Code={}, Message={}", error_code, error_message);
                Err(anyhow!("AWS credential validation failed: {} - {}", error_code, error_message))
            }
        }
    }
    
    /// Update the AWS credentials
    pub fn update_credentials(&mut self, access_key: String, secret_key: String, region: String) {
        debug!("Updating AWS credentials");
        self.access_key = access_key;
        self.secret_key = secret_key;
        self.region = region;
        self.client = None; // Force re-initialization with new credentials
    }
}
