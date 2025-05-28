use eframe::egui;
use std::path::PathBuf;
// use std::sync::mpsc; // Unused
// use std::thread; // Unused
use log::{debug, error};
// use dirs; // Unused

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
    }
    
    /// Show a folder selection dialog
    pub fn show_folder_dialog(&mut self) {
        if self.folder_dialog_open {
            return;
        }
        
        self.folder_dialog_open = true;
        
        // Use PowerShell to open a folder selection dialog
        debug!("Opening Windows folder selection dialog");
        
        // Create a PowerShell script that will open a folder browser dialog
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join("select_folder.ps1");
        let result_path = temp_dir.join("selected_folder.txt");
        
        // Delete the result file if it exists
        if result_path.exists() {
            let _ = std::fs::remove_file(&result_path);
        }
        
        // Create the PowerShell script
        let script_content = format!(
            r#"
            Add-Type -AssemblyName System.Windows.Forms
            $folderBrowser = New-Object System.Windows.Forms.FolderBrowserDialog
            $folderBrowser.Description = "Select a folder to add to S3Sync"
            $folderBrowser.ShowNewFolderButton = $true
            
            # Show the dialog
            if ($folderBrowser.ShowDialog() -eq 'OK') {{
                $selectedPath = $folderBrowser.SelectedPath
                # Write the selected path to a file
                $selectedPath | Out-File -FilePath "{}" -Encoding utf8
            }}
            "#,
            result_path.to_string_lossy().replace("\\", "\\\\")
        );
        
        // Write the script to a file
        if let Err(e) = std::fs::write(&script_path, script_content) {
            error!("Failed to write PowerShell script: {}", e);
            self.folder_dialog_open = false;
            return;
        }
        
        // Execute the PowerShell script
        let script_path_str = script_path.to_string_lossy().to_string();
        let output = std::process::Command::new("powershell.exe")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(&script_path_str)
            .output();
        
        match output {
            Ok(_) => {
                // Check if the result file exists
                if result_path.exists() {
                    // Read the selected path from the file
                    match std::fs::read_to_string(&result_path) {
                        Ok(content) => {
                            let selected_path = content.trim().trim_start_matches('\u{feff}');
                            if !selected_path.is_empty() {
                                let path = PathBuf::from(selected_path);
                                if path.exists() && path.is_dir() {
                                    debug!("Selected folder: {}", path.display());
                                    self.add_folder(path);
                                } else {
                                    error!("Selected path is not a valid directory: {}", selected_path);
                                }
                            }
                        },
                        Err(e) => {
                            error!("Failed to read selected path: {}", e);
                        }
                    }
                    
                    // Clean up the result file
                    let _ = std::fs::remove_file(&result_path);
                }
            },
            Err(e) => {
                error!("Failed to execute PowerShell script: {}", e);
            }
        }
        
        // Clean up the script file
        let _ = std::fs::remove_file(&script_path);
        
        self.folder_dialog_open = false;
    }
    
    /// Render the folder selection dialog - no longer needed with native dialog
    pub fn render_folder_dialog(&mut self, _ui: &mut egui::Ui) -> bool {
        false
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
