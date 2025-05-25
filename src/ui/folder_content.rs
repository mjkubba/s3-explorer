use eframe::egui;
use std::path::{PathBuf};
use std::fs;
use log::{debug, error};
use chrono::{DateTime, Utc};
use std::collections::HashSet;

/// Component for displaying the contents of a local folder
#[derive(Default)]
pub struct FolderContent {
    files: Vec<FileEntry>,
    filter: String,
    selected_files: HashSet<PathBuf>,
    pub current_folder: Option<PathBuf>,
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

impl std::fmt::Display for FileEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl FolderContent {
    /// Render the folder content UI
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // Filter input
        ui.horizontal(|ui| {
            ui.label("Filter:");
            if ui.text_edit_singleline(&mut self.filter).changed() {
                // Filter changed
            }
            
            if ui.button("Clear").clicked() {
                self.filter.clear();
            }
        });
        
        // File list
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.files.is_empty() {
                ui.label("No files to display");
                return;
            }
            
            // Table header
            ui.horizontal(|ui| {
                ui.style_mut().spacing.item_spacing.x = 10.0;
                ui.strong("Type");
                ui.strong("Name");
                ui.add_space(200.0);
                ui.strong("Size");
                ui.add_space(50.0);
                ui.strong("Modified");
            });
            
            ui.separator();
            
            // Filter files
            let filter = self.filter.to_lowercase();
            let filtered_files: Vec<_> = self.files.iter()
                .filter(|file| {
                    filter.is_empty() || file.name.to_lowercase().contains(&filter)
                })
                .collect();
            
            // Display files
            for file in filtered_files {
                let is_selected = self.selected_files.contains(&file.path);
                
                ui.horizontal(|ui| {
                    ui.style_mut().spacing.item_spacing.x = 10.0;
                    
                    // Selection checkbox
                    let mut selected = is_selected;
                    if ui.checkbox(&mut selected, "").changed() {
                        if selected {
                            self.selected_files.insert(file.path.clone());
                        } else {
                            self.selected_files.remove(&file.path);
                        }
                    }
                    
                    // Type icon
                    let icon = if file.is_directory { "📁" } else { "📄" };
                    ui.label(icon);
                    
                    // File name
                    let text = if file.is_directory {
                        egui::RichText::new(&file.name).strong()
                    } else {
                        egui::RichText::new(&file.name)
                    };
                    
                    if ui.selectable_label(is_selected, text).clicked() {
                        if is_selected {
                            self.selected_files.remove(&file.path);
                        } else {
                            self.selected_files.insert(file.path.clone());
                        }
                    }
                    
                    ui.add_space(200.0 - file.name.len() as f32 * 7.0);
                    
                    // File size
                    if file.is_directory {
                        ui.label("--");
                    } else {
                        ui.label(format_size(file.size));
                    }
                    
                    ui.add_space(50.0);
                    
                    // Last modified
                    ui.label(&file.last_modified);
                });
            }
        });
        
        // Actions
        ui.separator();
        
        ui.horizontal(|ui| {
            if ui.button("Select All").clicked() {
                self.select_all_visible();
            }
            
            if ui.button("Clear Selection").clicked() {
                self.clear_selection();
            }
            
            if ui.button("Refresh").clicked() {
                if let Some(path) = &self.current_folder {
                    self.load_files(path.clone());
                }
            }
        });
    }
    
    /// Set the current folder to display
    pub fn set_folder(&mut self, path: PathBuf) {
        debug!("Setting folder to: {}", path.display());
        self.current_folder = Some(path.clone());
        self.selected_files.clear();
        self.load_files(path);
    }
    
    /// Get the list of files
    pub fn files(&self) -> &[FileEntry] {
        debug!("Returning {} files", self.files.len());
        &self.files
    }
    
    /// Load files from the specified path
    fn load_files(&mut self, path: PathBuf) {
        debug!("Loading files from: {}", path.display());
        self.files.clear();
        
        match fs::read_dir(&path) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let file_path = entry.path();
                        let file_name = file_path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                            
                        let is_dir = file_path.is_dir();
                        let size = if is_dir {
                            0 // Directories show as 0 size
                        } else {
                            fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0)
                        };
                        
                        let last_modified = fs::metadata(&file_path)
                            .and_then(|m| m.modified())
                            .map(|time| {
                                let dt: DateTime<Utc> = time.into();
                                dt.format("%Y-%m-%d %H:%M:%S").to_string()
                            })
                            .unwrap_or_else(|_| "Unknown".to_string());
                            
                        self.files.push(FileEntry {
                            path: file_path,
                            name: file_name,
                            is_directory: is_dir,
                            size,
                            last_modified,
                        });
                    }
                }
                
                // Sort files: directories first, then by name
                self.files.sort_by(|a, b| {
                    match (a.is_directory, b.is_directory) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.cmp(&b.name),
                    }
                });
                
                debug!("Loaded {} files from {}", self.files.len(), path.display());
                
                debug!("Loaded {} files from {}", self.files.len(), path.display());
            },
            Err(e) => {
                error!("Failed to read directory {}: {}", path.display(), e);
            }
        }
    }
    
    /// Get the selected files
    pub fn selected_files(&self) -> Vec<&FileEntry> {
        self.files.iter()
            .filter(|file| self.selected_files.contains(&file.path))
            .collect()
    }
    
    /// Set the filter for the file list
    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
    }
    
    /// Get the current filter
    pub fn get_filter(&self) -> Option<&String> {
        if self.filter.is_empty() {
            None
        } else {
            Some(&self.filter)
        }
    }
    
    /// Select all visible files
    pub fn select_all_visible(&mut self) {
        let filter = self.filter.to_lowercase();
        
        for file in &self.files {
            if filter.is_empty() || file.name.to_lowercase().contains(&filter) {
                self.selected_files.insert(file.path.clone());
            }
        }
    }
    
    /// Clear all selections
    pub fn clear_selection(&mut self) {
        self.selected_files.clear();
    }
    
    /// Get the number of selected files
    pub fn selected_count(&self) -> usize {
        self.selected_files.len()
    }
    
    /// Get the total size of selected files
    pub fn selected_size(&self) -> u64 {
        self.files.iter()
            .filter(|file| self.selected_files.contains(&file.path))
            .map(|file| file.size)
            .sum()
    }
}

/// Format a size in bytes as a human-readable string
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
