use anyhow::{anyhow, Result};
use log::info;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::ui::folder_list::SyncFolder;

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// AWS region
    pub aws_region: String,
    /// Sync interval in minutes (0 = manual only)
    pub sync_interval: u32,
    /// Whether to delete files from S3 that were deleted locally
    pub delete_enabled: bool,
    /// Bandwidth limit in KB/s (None = unlimited)
    pub bandwidth_limit: Option<u32>,
    /// File patterns to exclude from sync
    pub exclude_patterns: Vec<String>,
    /// Folders to sync
    pub folders: Vec<SyncFolderConfig>,
}

/// Configuration for a folder to sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncFolderConfig {
    /// Local folder path
    pub path: String,
    /// Whether this folder is enabled for sync
    pub enabled: bool,
    /// S3 bucket to sync to
    pub bucket: String,
    /// Prefix within the bucket (optional)
    pub prefix: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            aws_region: "us-east-1".to_string(),
            sync_interval: 0, // Manual sync by default
            delete_enabled: false,
            bandwidth_limit: None,
            exclude_patterns: vec![],
            folders: vec![],
        }
    }
}

impl AppSettings {
    /// Load settings from the config file
    #[allow(dead_code)] // Will be used in future implementations
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        
        if !config_path.exists() {
            info!("Config file not found, creating default settings");
            let default_settings = Self::default();
            default_settings.save()?;
            return Ok(default_settings);
        }
        
        let config_str = fs::read_to_string(&config_path)?;
        let settings: AppSettings = serde_json::from_str(&config_str)?;
        
        info!("Loaded settings from {}", config_path.display());
        Ok(settings)
    }
    
    /// Save settings to the config file
    #[allow(dead_code)] // Will be used in future implementations
    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let config_str = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, config_str)?;
        
        info!("Saved settings to {}", config_path.display());
        Ok(())
    }
    
    /// Get the path to the config file
    #[allow(dead_code)] // Will be used in future implementations
    fn get_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow!("Could not determine config directory"))?
            .join("s3sync");
            
        Ok(config_dir.join("settings.json"))
    }
    
    /// Convert folder configs to SyncFolder objects
    #[allow(dead_code)] // Will be used in future implementations
    pub fn to_sync_folders(&self) -> Vec<SyncFolder> {
        self.folders
            .iter()
            .map(|folder_config| SyncFolder {
                path: PathBuf::from(&folder_config.path),
                enabled: folder_config.enabled,
                status: crate::ui::folder_list::SyncStatus::Pending,
                last_synced: None,
            })
            .collect()
    }
    
    /// Update folder configs from SyncFolder objects
    #[allow(dead_code)] // Will be used in future implementations
    pub fn update_from_sync_folders(&mut self, folders: &[SyncFolder]) {
        self.folders = folders
            .iter()
            .map(|folder| SyncFolderConfig {
                path: folder.path.to_string_lossy().to_string(),
                enabled: folder.enabled,
                bucket: "".to_string(), // This would need to be stored elsewhere
                prefix: None,
            })
            .collect();
    }
}
