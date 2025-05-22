use eframe::egui;
use std::path::{Path, PathBuf};
use std::fs;
use log::{debug, error};
use chrono::{DateTime, Local};

/// Component for displaying the contents of a local folder
#[derive(Default)]
pub struct FolderContent {
    files: Vec<FileEntry>,
    filter: String,
}

/// Represents a file or directory in the folder
#[derive(Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_directory: bool,
    pub size: u64,
    pub last_modified: String,
}

impl FolderContent {
    /// Render the folder content UI
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Local Files");
        
        // Filter
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter);
            
            if ui.button("Clear").clicked() {
                self.filter.clear();
            }
        });
        
        ui.separator();
        
        // File list
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Table header
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Name").strong());
                ui.add_space(200.0);
                ui.label(egui::RichText::new("Size").strong());
                ui.add_space(100.0);
                ui.label(egui::RichText::new("Last Modified").strong());
            });
            
            ui.separator();
            
            // Table rows
            if self.files.is_empty() {
                ui.label("No files in this folder");
            } else {
                let filter = self.filter.to_lowercase();
                
                for file in &self.files {
                    if !filter.is_empty() && !file.name.to_lowercase().contains(&filter) {
                        continue;
                    }
                    
                    ui.horizontal(|ui| {
                        let icon = if file.is_directory { "📁 " } else { "📄 " };
                        ui.label(format!("{}{}", icon, file.name));
                        ui.add_space(200.0 - file.name.len() as f32 * 7.0);
                        
                        let size_str = if file.is_directory {
                            "-".to_string()
                        } else {
                            format_size(file.size)
                        };
                        
                        ui.label(size_str);
                        ui.add_space(100.0);
                        ui.label(&file.last_modified);
                    });
                    
                    ui.separator();
                }
            }
        });
    }
    
    /// Load the contents of a folder
    pub fn load_folder(&mut self, path: &Path) {
        debug!("Loading folder contents: {}", path.display());
        self.files.clear();
        
        match fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let file_path = entry.path();
                        let file_name = entry.file_name().to_string_lossy().to_string();
                        let is_directory = file_path.is_dir();
                        
                        // Get file metadata
                        let (size, last_modified) = match entry.metadata() {
                            Ok(metadata) => {
                                let size = metadata.len();
                                let last_modified = match metadata.modified() {
                                    Ok(time) => {
                                        // Convert system time to chrono DateTime
                                        let datetime: DateTime<Local> = time.into();
                                        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                                    },
                                    Err(_) => "Unknown".to_string(),
                                };
                                (size, last_modified)
                            },
                            Err(_) => (0, "Unknown".to_string()),
                        };
                        
                        self.files.push(FileEntry {
                            path: file_path,
                            name: file_name,
                            is_directory,
                            size,
                            last_modified,
                        });
                    }
                }
                
                // Sort: directories first, then by name
                self.files.sort_by(|a, b| {
                    match (a.is_directory, b.is_directory) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.cmp(&b.name),
                    }
                });
                
                debug!("Loaded {} files/directories", self.files.len());
            },
            Err(e) => {
                error!("Failed to read directory {}: {}", path.display(), e);
            }
        }
    }
}

/// Format file size in human-readable format
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if size < KB {
        format!("{} B", size)
    } else if size < MB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else if size < GB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else {
        format!("{:.2} GB", size as f64 / GB as f64)
    }
}
