use anyhow::Result;
use log::{error, info};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;

use crate::ui::folder_list::SyncFolder;

/// Scheduler for periodic sync operations
pub struct SyncScheduler {
    interval_minutes: u32,
    running: Arc<Mutex<bool>>,
    folders: Arc<Mutex<Vec<SyncFolder>>>,
}

/// Sync task message
pub enum SyncTask {
    /// Sync a specific folder
    SyncFolder(usize),
    /// Sync all folders
    SyncAll,
    /// Stop all sync operations
    Stop,
}

impl SyncScheduler {
    /// Create a new sync scheduler
    pub fn new(interval_minutes: u32) -> Self {
        Self {
            interval_minutes,
            running: Arc::new(Mutex::new(false)),
            folders: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Set the sync interval in minutes (0 = manual only)
    pub fn set_interval(&mut self, minutes: u32) {
        self.interval_minutes = minutes;
    }
    
    /// Update the folders to sync
    pub fn update_folders(&mut self, folders: Vec<SyncFolder>) {
        let mut folders_lock = self.folders.lock().unwrap();
        *folders_lock = folders;
    }
    
    /// Start the scheduler
    pub fn start(&self, tx: mpsc::Sender<SyncTask>) -> Result<()> {
        if self.interval_minutes == 0 {
            info!("Scheduler not started (manual sync only)");
            return Ok(());
        }
        
        {
            let mut running = self.running.lock().unwrap();
            if *running {
                return Ok(());
            }
            *running = true;
        }
        
        let interval_minutes = self.interval_minutes;
        let running = self.running.clone();
        let folders = self.folders.clone();
        
        tokio::spawn(async move {
            info!("Starting sync scheduler with interval of {} minutes", interval_minutes);
            
            loop {
                // Wait for the specified interval
                time::sleep(Duration::from_secs(interval_minutes as u64 * 60)).await;
                
                // Check if we should still be running
                {
                    let running_lock = running.lock().unwrap();
                    if !*running_lock {
                        break;
                    }
                }
                
                // Get the number of folders
                let folder_count = {
                    let folders_lock = folders.lock().unwrap();
                    folders_lock.len()
                };
                
                // Send sync task for each folder
                for i in 0..folder_count {
                    if tx.send(SyncTask::SyncFolder(i)).await.is_err() {
                        error!("Failed to send sync task for folder {}", i);
                        break;
                    }
                }
            }
            
            info!("Sync scheduler stopped");
        });
        
        Ok(())
    }
    
    /// Stop the scheduler
    pub fn stop(&self) {
        let mut running = self.running.lock().unwrap();
        *running = false;
        info!("Stopping sync scheduler");
    }
}
