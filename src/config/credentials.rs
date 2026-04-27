use anyhow::{anyhow, Result};
use keyring::Entry;
use log::{debug, info};

const SERVICE_NAME: &str = "s3sync";

/// Credential manager for securely storing AWS credentials
#[derive(Default)]
pub struct CredentialManager;

impl CredentialManager {
    /// Save AWS credentials to the system keyring
    pub fn save_credentials(access_key: &str, secret_key: &str, region: &str) -> Result<()> {
        let access_key_entry = Entry::new(SERVICE_NAME, "aws_access_key")
            .map_err(|e| anyhow!("Failed to create keyring entry: {}", e))?;
        access_key_entry.set_password(access_key)
            .map_err(|e| anyhow!("Failed to save access key: {}", e))?;
        
        let secret_key_entry = Entry::new(SERVICE_NAME, "aws_secret_key")
            .map_err(|e| anyhow!("Failed to create keyring entry: {}", e))?;
        secret_key_entry.set_password(secret_key)
            .map_err(|e| anyhow!("Failed to save secret key: {}", e))?;
        
        let region_entry = Entry::new(SERVICE_NAME, "aws_region")
            .map_err(|e| anyhow!("Failed to create keyring entry: {}", e))?;
        region_entry.set_password(region)
            .map_err(|e| anyhow!("Failed to save region: {}", e))?;
        
        info!("AWS credentials saved to keyring");
        Ok(())
    }
    
    pub fn load_access_key() -> Result<String> {
        let entry = Entry::new(SERVICE_NAME, "aws_access_key")
            .map_err(|e| anyhow!("Failed to create keyring entry: {}", e))?;
        match entry.get_password() {
            Ok(password) => Ok(password),
            Err(e) => {
                debug!("AWS access key not found in keyring: {}", e);
                Ok(String::new())
            }
        }
    }
    
    pub fn load_secret_key() -> Result<String> {
        let entry = Entry::new(SERVICE_NAME, "aws_secret_key")
            .map_err(|e| anyhow!("Failed to create keyring entry: {}", e))?;
        match entry.get_password() {
            Ok(password) => Ok(password),
            Err(e) => {
                debug!("AWS secret key not found in keyring: {}", e);
                Ok(String::new())
            }
        }
    }
    
    pub fn load_region() -> Result<String> {
        let entry = Entry::new(SERVICE_NAME, "aws_region")
            .map_err(|e| anyhow!("Failed to create keyring entry: {}", e))?;
        match entry.get_password() {
            Ok(region) => Ok(region),
            Err(e) => {
                debug!("AWS region not found in keyring: {}", e);
                Ok("us-east-1".to_string())
            }
        }
    }
    
    #[allow(dead_code)]
    pub fn clear_credentials() -> Result<()> {
        if let Ok(entry) = Entry::new(SERVICE_NAME, "aws_access_key") {
            let _ = entry.delete_credential();
        }
        if let Ok(entry) = Entry::new(SERVICE_NAME, "aws_secret_key") {
            let _ = entry.delete_credential();
        }
        if let Ok(entry) = Entry::new(SERVICE_NAME, "aws_region") {
            let _ = entry.delete_credential();
        }
        info!("AWS credentials cleared from keyring");
        Ok(())
    }
    
    pub fn has_credentials() -> bool {
        match (Self::load_access_key(), Self::load_secret_key()) {
            (Ok(access_key), Ok(secret_key)) => !access_key.is_empty() && !secret_key.is_empty(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_credential_roundtrip() {
        // Commented out: modifies system keyring
    }
}
