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
    
    /// Get a client for a specific region
    pub async fn get_client_for_region(&self, region: &str) -> Result<Client> {
        debug!("Creating client for specific region: {}", region);
        
        if self.access_key.is_empty() || self.secret_key.is_empty() {
            return Err(anyhow!("AWS credentials not provided"));
        }
        
        // Set environment variables for AWS SDK
        env::set_var("AWS_ACCESS_KEY_ID", &self.access_key);
        env::set_var("AWS_SECRET_ACCESS_KEY", &self.secret_key);
        env::set_var("AWS_REGION", region);
        
        let region_provider = RegionProviderChain::first_try(Region::new(region.to_string()))
            .or_default_provider();
            
        debug!("Creating AWS SDK configuration for region {}", region);
        let config = aws_config::from_env()
            .region(region_provider)
            .load()
            .await;
            
        debug!("Creating S3 client for region {}", region);
        let client = Client::new(&config);
        
        info!("AWS S3 client created for region {}", region);
        Ok(client)
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
    
    /// Get the location (region) of a bucket
    pub async fn get_bucket_location(&mut self, bucket_name: &str) -> Result<String> {
        debug!("Getting location for bucket: {}", bucket_name);
        
        let client = self.get_client().await?;
        
        match client.get_bucket_location().bucket(bucket_name).send().await {
            Ok(resp) => {
                // Extract the location constraint as a string
                let location_str = match resp.location_constraint() {
                    Some(constraint) => {
                        // Convert the enum to a debug string and extract the value
                        let debug_str = format!("{:?}", constraint);
                        if debug_str.contains("\"\"") || debug_str == "Empty" {
                            // Empty constraint means us-east-1
                            "us-east-1".to_string()
                        } else if debug_str.starts_with("Unknown(") {
                            // Extract the value from Unknown("value")
                            let start = debug_str.find('(').map(|i| i + 2).unwrap_or(0);
                            let end = debug_str.rfind('"').unwrap_or(debug_str.len());
                            if start < end {
                                debug_str[start..end].to_string()
                            } else {
                                "us-east-1".to_string() // Default if parsing fails
                            }
                        } else {
                            // For known enum variants, extract the region name
                            let region_name = match debug_str.as_str() {
                                "EuWest1" => "eu-west-1",
                                "UsWest1" => "us-west-1",
                                "UsWest2" => "us-west-2",
                                "EuWest2" => "eu-west-2",
                                "EuWest3" => "eu-west-3",
                                "UsEast2" => "us-east-2",
                                "ApSouth1" => "ap-south-1",
                                "ApSoutheast1" => "ap-southeast-1",
                                "ApSoutheast2" => "ap-southeast-2",
                                "ApNortheast1" => "ap-northeast-1",
                                "ApNortheast2" => "ap-northeast-2",
                                "ApNortheast3" => "ap-northeast-3",
                                "SaEast1" => "sa-east-1",
                                "CnNorth1" => "cn-north-1",
                                "CnNorthwest1" => "cn-northwest-1",
                                "UsGovWest1" => "us-gov-west-1",
                                "UsGovEast1" => "us-gov-east-1",
                                "EuCentral1" => "eu-central-1",
                                "EuNorth1" => "eu-north-1",
                                "MeSouth1" => "me-south-1",
                                "AfSouth1" => "af-south-1",
                                "EuSouth1" => "eu-south-1",
                                "ApEast1" => "ap-east-1",
                                _ => "us-east-1", // Default for unknown regions
                            };
                            region_name.to_string()
                        }
                    },
                    None => "us-east-1".to_string(), // Default if no constraint is specified
                };
                
                info!("Bucket {} is in region {}", bucket_name, location_str);
                Ok(location_str)
            },
            Err(err) => {
                let sdk_error = err.into_service_error();
                let error_code = sdk_error.code().unwrap_or("Unknown");
                let error_message = sdk_error.message().unwrap_or("No error message");
                
                error!("Failed to get bucket location: Code={}, Message={}", error_code, error_message);
                Err(anyhow!("Failed to get bucket location: {} - {}", error_code, error_message))
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
