use anyhow::{anyhow, Result};
use keyring::Entry;
use log::{debug, info};

/// Service name for keyring
const SERVICE_NAME: &str = "s3sync";

/// Credential manager for securely storing AWS credentials
pub struct CredentialManager;

impl CredentialManager {
    /// Save AWS credentials to the system keyring
    pub fn save_credentials(access_key: &str, secret_key: &str) -> Result<()> {
        // Save access key
        let access_key_entry = Entry::new(SERVICE_NAME, "aws_access_key");
        
        if let Err(e) = access_key_entry.set_password(access_key) {
            return Err(anyhow!("Failed to save access key: {}", e));
        }
        
        // Save secret key
        let secret_key_entry = Entry::new(SERVICE_NAME, "aws_secret_key");
        
        if let Err(e) = secret_key_entry.set_password(secret_key) {
            return Err(anyhow!("Failed to save secret key: {}", e));
        }
        
        info!("AWS credentials saved to keyring");
        Ok(())
    }
    
    /// Load AWS access key from the system keyring
    pub fn load_access_key() -> Result<String> {
        let entry = Entry::new(SERVICE_NAME, "aws_access_key");
        
        match entry.get_password() {
            Ok(password) => Ok(password),
            Err(e) => {
                debug!("AWS access key not found in keyring: {}", e);
                Ok(String::new())
            }
        }
    }
    
    /// Load AWS secret key from the system keyring
    pub fn load_secret_key() -> Result<String> {
        let entry = Entry::new(SERVICE_NAME, "aws_secret_key");
        
        match entry.get_password() {
            Ok(password) => Ok(password),
            Err(e) => {
                debug!("AWS secret key not found in keyring: {}", e);
                Ok(String::new())
            }
        }
    }
    
    /// Clear AWS credentials from the system keyring
    pub fn clear_credentials() -> Result<()> {
        // Clear access key
        let access_key_entry = Entry::new(SERVICE_NAME, "aws_access_key");
        
        let _ = access_key_entry.delete_password();
        
        // Clear secret key
        let secret_key_entry = Entry::new(SERVICE_NAME, "aws_secret_key");
        
        let _ = secret_key_entry.delete_password();
        
        info!("AWS credentials cleared from keyring");
        Ok(())
    }
    
    /// Test if credentials are available
    pub fn has_credentials() -> bool {
        match (Self::load_access_key(), Self::load_secret_key()) {
            (Ok(access_key), Ok(secret_key)) => !access_key.is_empty() && !secret_key.is_empty(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_credential_roundtrip() {
        // This test is commented out because it would modify the system keyring
        // Uncomment to test manually
        /*
        let test_access_key = "test_access_key";
        let test_secret_key = "test_secret_key";
        
        // Save credentials
        CredentialManager::save_credentials(test_access_key, test_secret_key).unwrap();
        
        // Load and verify
        let loaded_access_key = CredentialManager::load_access_key().unwrap();
        let loaded_secret_key = CredentialManager::load_secret_key().unwrap();
        
        assert_eq!(loaded_access_key, test_access_key);
        assert_eq!(loaded_secret_key, test_secret_key);
        
        // Clean up
        CredentialManager::clear_credentials().unwrap();
        */
    }
}
