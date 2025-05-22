use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use log::{debug, error};

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
    folder_receiver: Option<mpsc::Receiver<Option<PathBuf>>>,
}

impl FolderList {
    /// Render the folder list UI
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Folders");
        
        // Check if we have a result from the folder dialog
        if let Some(receiver) = &self.folder_receiver {
            match receiver.try_recv() {
                Ok(Some(path)) => {
                    debug!("Selected folder: {}", path.display());
                    self.add_folder(path);
                    self.folder_dialog_open = false;
                    self.folder_receiver = None;
                },
                Ok(None) => {
                    debug!("Folder selection canceled");
                    self.folder_dialog_open = false;
                    self.folder_receiver = None;
                },
                Err(mpsc::TryRecvError::Empty) => {
                    // Still waiting for selection
                },
                Err(mpsc::TryRecvError::Disconnected) => {
                    error!("Folder selection dialog channel disconnected");
                    self.folder_dialog_open = false;
                    self.folder_receiver = None;
                }
            }
        }
        
        ui.horizontal(|ui| {
            if ui.button("Add Folder").clicked() && !self.folder_dialog_open {
                self.open_folder_dialog();
            }
            
            if let Some(index) = self.selected_index {
                if ui.button("Remove Folder").clicked() {
                    self.remove_folder(index);
                }
            }
        });
        
        ui.separator();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (index, folder) in self.folders.iter_mut().enumerate() {
                let is_selected = Some(index) == self.selected_index;
                
                let response = ui.selectable_label(
                    is_selected,
                    format!("{}", folder.path.display()),
                );
                
                if response.clicked() {
                    self.selected_index = Some(index);
                }
                
                // Show folder status
                ui.horizontal(|ui| {
                    ui.checkbox(&mut folder.enabled, "");
                    
                    let status_text = match &folder.status {
                        SyncStatus::Synced => "✓ Synced",
                        SyncStatus::Pending => "⏱ Pending",
                        SyncStatus::Syncing => "⟳ Syncing",
                        SyncStatus::Error(_) => "❌ Error",
                    };
                    
                    ui.label(status_text);
                    
                    if let Some(time) = folder.last_synced {
                        ui.label(format!("Last: {}", time.format("%H:%M:%S")));
                    }
                });
                
                ui.separator();
            }
        });
        
        // Show a message if folder dialog is open
        if self.folder_dialog_open {
            ui.label("Folder selection dialog is open...");
        }
    }
    
    /// Open a folder selection dialog
    fn open_folder_dialog(&mut self) {
        debug!("Opening folder selection dialog");
        self.folder_dialog_open = true;
        
        // Create a channel to receive the selected folder
        let (sender, receiver) = mpsc::channel();
        self.folder_receiver = Some(receiver);
        
        // Spawn a thread to show the folder dialog
        thread::spawn(move || {
            match native_dialog::FileDialog::new()
                .set_location(&dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")))
                .show_open_single_dir() {
                    Ok(Some(path)) => {
                        let _ = sender.send(Some(path));
                    },
                    Ok(None) => {
                        let _ = sender.send(None);
                    },
                    Err(e) => {
                        error!("Error showing folder dialog: {}", e);
                        let _ = sender.send(None);
                    }
                }
        });
    }
    
    /// Add a new folder to the list
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
}
