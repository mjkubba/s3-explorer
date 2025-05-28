use anyhow::{anyhow, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;
use aws_types::region::Region;
use log::{error, info, debug};
use std::collections::HashMap;
use std::sync::Arc;
// use tokio::sync::Mutex as TokioMutex; // Unused
// use aws_config::meta::credentials::CredentialsProviderChain; // Unused
use aws_sdk_s3::config::Credentials;
// use aws_sdk_s3::error::ProvideErrorMetadata; // Unused

use crate::config::credentials::CredentialManager;
use crate::aws::s3::S3ErrorHelper;

/// AWS authentication manager
#[derive(Clone)]
pub struct AwsAuth {
    access_key: String,
    secret_key: String,
    region: String,
    client: Option<Arc<Client>>,
    region_clients: HashMap<String, Arc<Client>>,
}

impl Default for AwsAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl AwsAuth {
    /// Create a new AWS authentication manager
    pub fn new() -> Self {
        Self {
            access_key: String::new(),
            secret_key: String::new(),
            region: "us-east-1".to_string(),
            client: None,
            region_clients: HashMap::new(),
        }
    }
    
    /// Set the AWS credentials
    pub fn set_credentials(&mut self, access_key: String, secret_key: String, region: String) {
        debug!("Setting AWS credentials");
        self.access_key = access_key;
        self.secret_key = secret_key;
        self.region = region;
        
        // Clear the client so it will be recreated with the new credentials
        self.client = None;
        self.region_clients.clear();
    }
    
    /// Initialize the AWS client
    pub async fn initialize(&mut self) -> Result<()> {
        debug!("Initializing AWS client");
        
        if self.access_key.is_empty() || self.secret_key.is_empty() {
            return Err(anyhow!("AWS credentials not set"));
        }
        
        // Create the client
        let _ = self.get_client().await?;
        
        // Test the credentials
        self.test_credentials().await
    }
    
    /// Load credentials from the system keyring
    #[allow(dead_code)] // Will be used in future implementations
    pub fn load_credentials(&mut self) -> Result<()> {
        debug!("Loading AWS credentials from keyring");
        
        let access_key = CredentialManager::load_access_key()?;
        let secret_key = CredentialManager::load_secret_key()?;
        let region = CredentialManager::load_region()?;
        
        if access_key.is_empty() || secret_key.is_empty() {
            return Err(anyhow!("AWS credentials not found in keyring"));
        }
        
        self.set_credentials(access_key, secret_key, region);
        Ok(())
    }
    
    /// Test if the credentials are valid
    pub async fn test_credentials(&mut self) -> Result<()> {
        debug!("Testing AWS credentials");
        
        if self.access_key.is_empty() || self.secret_key.is_empty() {
            return Err(anyhow!("AWS credentials not set"));
        }
        
        // Try to list buckets
        let client = self.get_client().await?;
        
        match client.list_buckets().send().await {
            Ok(_) => {
                info!("AWS credentials are valid");
                Ok(())
            },
            Err(e) => {
                // Use our helper to extract detailed error information
                let detailed_error = S3ErrorHelper::extract_error_details(&e);
                
                error!("AWS credentials test failed: {}", detailed_error);
                Err(anyhow!("AWS credentials test failed: {}", detailed_error))
            }
        }
    }
    
    /// Get the AWS S3 client
    pub async fn get_client(&mut self) -> Result<Arc<Client>> {
        if let Some(client) = &self.client {
            return Ok(client.clone());
        }
        
        debug!("Creating new AWS S3 client for region {}", self.region);
        
        // Create a new client
        let region_provider = RegionProviderChain::first_try(Region::new(self.region.clone()));
        
        // Create credentials
        let credentials = Credentials::new(
            &self.access_key,
            &self.secret_key,
            None,
            None,
            "s3sync-app",
        );
        
        // Build the config
        let shared_config = aws_config::from_env()
            .region(region_provider)
            .credentials_provider(credentials)
            .load()
            .await;
            
        let client = Arc::new(Client::new(&shared_config));
        self.client = Some(client.clone());
        
        Ok(client)
    }
    
    /// Get an AWS S3 client for a specific region
    pub async fn get_client_for_region(&mut self, region: &str) -> Result<Arc<Client>> {
        // Check if we already have a client for this region
        if let Some(client) = self.region_clients.get(region) {
            return Ok(client.clone());
        }
        
        debug!("Creating new AWS S3 client for region {}", region);
        
        // Create a new client for the specified region
        let region_provider = RegionProviderChain::first_try(Region::new(region.to_string()));
        
        // Create credentials
        let credentials = Credentials::new(
            &self.access_key,
            &self.secret_key,
            None,
            None,
            "s3sync-app",
        );
        
        // Build the config
        let shared_config = aws_config::from_env()
            .region(region_provider)
            .credentials_provider(credentials)
            .load()
            .await;
            
        let client = Arc::new(Client::new(&shared_config));
        self.region_clients.insert(region.to_string(), client.clone());
        
        Ok(client)
    }
    
    /// Get the AWS access key
    #[allow(dead_code)] // Will be used in future implementations
    pub fn access_key(&self) -> &str {
        &self.access_key
    }
    
    /// Get the AWS secret key
    #[allow(dead_code)] // Will be used in future implementations
    pub fn secret_key(&self) -> &str {
        &self.secret_key
    }
    
    /// Get the AWS region
    #[allow(dead_code)] // Will be used in future implementations
    pub fn region(&self) -> &str {
        &self.region
    }
}
