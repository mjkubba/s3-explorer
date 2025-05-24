use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use log::{debug, error};
use dirs;

/// Represents a folder to be synced
#[derive(Clone, Debug)]
pub struct SyncFolder {
    pub path: PathBuf,
    pub enabled: bool,
    pub status: SyncStatus,
    pub last_synced: Option<chrono::DateTime<chrono::Local>>,
}

/// Status of a sync folder
#[derive(Clone, Debug, PartialEq)]
pub enum SyncStatus {
    Synced,
    Pending,
    Syncing,
    Error(String),
}

/// Component for managing the list of folders to sync
#[derive(Default)]
pub struct FolderList {
    pub folders: Vec<SyncFolder>,
    pub selected_index: Option<usize>,
    folder_dialog_open: bool,
}

impl FolderList {
    /// Render the folder list UI
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Local Folders");
        
        // Folder list
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.folders.is_empty() {
                ui.label("No folders added yet");
            } else {
                // Create a copy of the folders for iteration
                let folders = self.folders.clone();
                
                for (i, folder) in folders.iter().enumerate() {
                    let is_selected = self.selected_index == Some(i);
                    
                    ui.horizontal(|ui| {
                        // Selection
                        if ui.selectable_label(is_selected, "").clicked() {
                            self.selected_index = Some(i);
                        }
                        
                        // Enabled checkbox
                        let mut enabled = folder.enabled;
                        if ui.checkbox(&mut enabled, "").changed() {
                            // Update the folder's enabled state
                            if i < self.folders.len() {
                                self.folders[i].enabled = enabled;
                            }
                        }
                        
                        // Folder path
                        let text = folder.path.to_string_lossy().to_string();
                        if ui.selectable_label(is_selected, &text).clicked() {
                            self.selected_index = Some(i);
                        }
                        
                        // Status indicator
                        match &folder.status {
                            SyncStatus::Synced => {
                                ui.label(egui::RichText::new("✓").color(egui::Color32::GREEN));
                            },
                            SyncStatus::Pending => {
                                ui.label("⏱");
                            },
                            SyncStatus::Syncing => {
                                ui.add(egui::Spinner::new());
                            },
                            SyncStatus::Error(msg) => {
                                ui.label(egui::RichText::new("✗").color(egui::Color32::RED))
                                    .on_hover_text(msg);
                            },
                        }
                        
                        // Last synced time
                        if let Some(time) = folder.last_synced {
                            ui.label(time.format("%Y-%m-%d %H:%M").to_string());
                        } else {
                            ui.label("Never");
                        }
                    });
                }
            }
        });
        
        // Actions
        ui.separator();
        
        ui.horizontal(|ui| {
            if ui.button("Add Folder").clicked() {
                self.show_folder_dialog();
            }
            
            if ui.button("Remove").clicked() {
                if let Some(index) = self.selected_index {
                    self.remove_folder(index);
                }
            }
        });
    }
    
    /// Show a folder selection dialog
    fn show_folder_dialog(&mut self) {
        if self.folder_dialog_open {
            return;
        }
        
        self.folder_dialog_open = true;
        
        // Create a channel to receive the selected path
        let (tx, rx) = mpsc::channel();
        
        // Spawn a thread to show the dialog
        thread::spawn(move || {
            let result = native_dialog::FileDialog::new()
                .set_location(&dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")))
                .show_open_single_dir();
                
            match result {
                Ok(Some(path)) => {
                    let _ = tx.send(Some(path));
                },
                Ok(None) => {
                    let _ = tx.send(None);
                },
                Err(e) => {
                    error!("Failed to show folder dialog: {}", e);
                    let _ = tx.send(None);
                }
            }
        });
        
        // Check for the result in the next frame
        let result = rx.try_recv();
        
        match result {
            Ok(Some(path)) => {
                self.add_folder(path);
                self.folder_dialog_open = false;
            },
            Ok(None) => {
                self.folder_dialog_open = false;
            },
            Err(mpsc::TryRecvError::Empty) => {
                // Still waiting for the dialog
            },
            Err(mpsc::TryRecvError::Disconnected) => {
                error!("Folder dialog thread disconnected");
                self.folder_dialog_open = false;
            }
        }
    }
    
    /// Add a folder to the list
    pub fn add_folder(&mut self, path: PathBuf) {
        debug!("Adding folder: {}", path.display());
        
        self.folders.push(SyncFolder {
            path,
            enabled: true,
            status: SyncStatus::Pending,
            last_synced: None,
        });
        
        // Select the newly added folder
        self.selected_index = Some(self.folders.len() - 1);
    }
    
    /// Remove a folder from the list
    pub fn remove_folder(&mut self, index: usize) {
        if index < self.folders.len() {
            debug!("Removing folder: {}", self.folders[index].path.display());
            self.folders.remove(index);
            if let Some(selected) = self.selected_index {
                if selected >= index && selected > 0 {
                    self.selected_index = Some(selected - 1);
                } else if selected == index {
                    self.selected_index = None;
                }
            }
        }
    }
    
    /// Get the currently selected folder
    pub fn selected_folder(&self) -> Option<&PathBuf> {
        self.selected_index.and_then(|index| {
            if index < self.folders.len() {
                Some(&self.folders[index].path)
            } else {
                None
            }
        })
    }
    
    /// Remove the currently selected folder
    pub fn remove_selected(&mut self) {
        if let Some(index) = self.selected_index {
            self.remove_folder(index);
        }
    }
    
    /// Update the status of a folder
    pub fn update_status(&mut self, path: &PathBuf, status: SyncStatus) {
        for folder in &mut self.folders {
            if folder.path == *path {
                folder.status = status.clone();
                if let SyncStatus::Synced = &status {
                    folder.last_synced = Some(chrono::Local::now());
                }
                break;
            }
        }
    }
    
    /// Get all enabled folders
    pub fn enabled_folders(&self) -> Vec<&PathBuf> {
        self.folders.iter()
            .filter(|f| f.enabled)
            .map(|f| &f.path)
            .collect()
    }
}
