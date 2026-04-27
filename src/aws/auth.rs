use anyhow::{anyhow, Result};
use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use log::{error, info, debug};
use std::collections::HashMap;
use std::sync::Arc;

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
    pub fn new() -> Self {
        Self {
            access_key: String::new(),
            secret_key: String::new(),
            region: "us-east-1".to_string(),
            client: None,
            region_clients: HashMap::new(),
        }
    }
    
    pub fn set_credentials(&mut self, access_key: String, secret_key: String, region: String) {
        debug!("Setting AWS credentials");
        self.access_key = access_key;
        self.secret_key = secret_key;
        self.region = region;
        self.client = None;
        self.region_clients.clear();
    }
    
    pub async fn initialize(&mut self) -> Result<()> {
        debug!("Initializing AWS client");
        if self.access_key.is_empty() || self.secret_key.is_empty() {
            return Err(anyhow!("AWS credentials not set"));
        }
        let _ = self.get_client().await?;
        self.test_credentials().await
    }
    
    #[allow(dead_code)]
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
    
    pub async fn test_credentials(&mut self) -> Result<()> {
        debug!("Testing AWS credentials");
        if self.access_key.is_empty() || self.secret_key.is_empty() {
            return Err(anyhow!("AWS credentials not set"));
        }
        let client = self.get_client().await?;
        match client.list_buckets().send().await {
            Ok(_) => {
                info!("AWS credentials are valid");
                Ok(())
            },
            Err(e) => {
                let detailed_error = S3ErrorHelper::extract_error_details(&e);
                error!("AWS credentials test failed: {}", detailed_error);
                Err(anyhow!("AWS credentials test failed: {}", detailed_error))
            }
        }
    }
    
    pub async fn get_client(&mut self) -> Result<Arc<Client>> {
        if let Some(client) = &self.client {
            return Ok(client.clone());
        }
        debug!("Creating new AWS S3 client for region {}", self.region);
        let client = Arc::new(self.build_client(&self.region.clone()).await);
        self.client = Some(client.clone());
        Ok(client)
    }
    
    pub async fn get_client_for_region(&mut self, region: &str) -> Result<Arc<Client>> {
        if let Some(client) = self.region_clients.get(region) {
            return Ok(client.clone());
        }
        debug!("Creating new AWS S3 client for region {}", region);
        let client = Arc::new(self.build_client(region).await);
        self.region_clients.insert(region.to_string(), client.clone());
        Ok(client)
    }
    
    async fn build_client(&self, region: &str) -> Client {
        let credentials = Credentials::new(
            &self.access_key,
            &self.secret_key,
            None,
            None,
            "s3sync-app",
        );
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(Region::new(region.to_string()))
            .credentials_provider(credentials)
            .load()
            .await;
        Client::new(&config)
    }
    
    #[allow(dead_code)]
    pub fn access_key(&self) -> &str { &self.access_key }
    #[allow(dead_code)]
    pub fn secret_key(&self) -> &str { &self.secret_key }
    #[allow(dead_code)]
    pub fn region(&self) -> &str { &self.region }
}
