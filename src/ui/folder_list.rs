use eframe::egui;
use std::path::PathBuf;

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
}

impl FolderList {
    /// Render the folder list UI
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Folders");
        
        if ui.button("Add Folder").clicked() {
            // TODO: Implement folder selection dialog
            // For now, add a dummy folder for testing
            self.add_test_folder();
        }
        
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
                        SyncStatus::Error(msg) => "❌ Error",
                    };
                    
                    ui.label(status_text);
                    
                    if let Some(time) = folder.last_synced {
                        ui.label(format!("Last: {}", time.format("%H:%M:%S")));
                    }
                });
                
                ui.separator();
            }
        });
    }
    
    /// Add a test folder for development purposes
    fn add_test_folder(&mut self) {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let documents_dir = home_dir.join("Documents");
        
        self.folders.push(SyncFolder {
            path: documents_dir,
            enabled: true,
            status: SyncStatus::Pending,
            last_synced: Some(chrono::Local::now()),
        });
    }
    
    /// Add a new folder to the list
    pub fn add_folder(&mut self, path: PathBuf) {
        self.folders.push(SyncFolder {
            path,
            enabled: true,
            status: SyncStatus::Pending,
            last_synced: None,
        });
    }
    
    /// Remove a folder from the list
    pub fn remove_folder(&mut self, index: usize) {
        if index < self.folders.len() {
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
