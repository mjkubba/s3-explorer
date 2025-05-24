use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::sync::filter::FileFilter;

/// UI component for configuring file filters
pub struct FilterView {
    filter: Arc<Mutex<FileFilter>>,
    include_patterns: String,
    exclude_patterns: String,
    include_extensions: String,
    exclude_extensions: String,
    min_size: String,
    max_size: String,
    error_message: Option<String>,
    changes_applied: bool,
}

impl FilterView {
    /// Create a new filter view with the given filter
    pub fn new(filter: Arc<Mutex<FileFilter>>) -> Self {
        Self {
            filter,
            include_patterns: String::new(),
            exclude_patterns: String::new(),
            include_extensions: String::new(),
            exclude_extensions: String::new(),
            min_size: String::new(),
            max_size: String::new(),
            error_message: None,
            changes_applied: false,
        }
    }
    
    /// Render the filter UI and return true if changes were applied
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        ui.heading("File Filters");
        
        // Reset the changes_applied flag
        self.changes_applied = false;
        
        // Show error message if any
        if let Some(error) = &self.error_message {
            ui.colored_label(egui::Color32::RED, error);
            if ui.button("Clear Error").clicked() {
                self.error_message = None;
            }
            ui.separator();
        }
        
        // Include patterns
        ui.collapsing("Include Patterns", |ui| {
            ui.label("Files matching these patterns will be included (one pattern per line):");
            ui.text_edit_multiline(&mut self.include_patterns);
            ui.label("Example: docs/**/*.md");
        });
        
        // Exclude patterns
        ui.collapsing("Exclude Patterns", |ui| {
            ui.label("Files matching these patterns will be excluded (one pattern per line):");
            ui.text_edit_multiline(&mut self.exclude_patterns);
            ui.label("Example: temp/**/*");
        });
        
        // File extensions
        ui.collapsing("File Extensions", |ui| {
            ui.label("Include extensions (comma-separated, without dots):");
            ui.text_edit_singleline(&mut self.include_extensions);
            ui.label("Example: txt,md,json");
            
            ui.label("Exclude extensions (comma-separated, without dots):");
            ui.text_edit_singleline(&mut self.exclude_extensions);
            ui.label("Example: tmp,bak,log");
        });
        
        // File size
        ui.collapsing("File Size", |ui| {
            ui.label("Minimum file size (bytes, KB, MB, GB):");
            ui.text_edit_singleline(&mut self.min_size);
            ui.label("Example: 10KB or 1MB");
            
            ui.label("Maximum file size (bytes, KB, MB, GB):");
            ui.text_edit_singleline(&mut self.max_size);
            ui.label("Example: 100MB or 1GB");
        });
        
        ui.separator();
        
        ui.horizontal(|ui| {
            if ui.button("Apply Filters").clicked() {
                self.apply_filters();
            }
            
            if ui.button("Clear All Filters").clicked() {
                self.clear_filters();
            }
        });
        
        // Return whether changes were applied
        self.changes_applied
    }
    
    /// Get the current filter
    pub fn get_filter(&self) -> Arc<Mutex<FileFilter>> {
        self.filter.clone()
    }
    
    /// Apply the configured filters
    fn apply_filters(&mut self) {
        // Create a new filter
        let mut new_filter = FileFilter::new();
        
        // Parse include patterns
        if !self.include_patterns.is_empty() {
            match new_filter.parse_patterns(&self.include_patterns) {
                Ok(_) => {},
                Err(e) => {
                    self.error_message = Some(format!("Invalid include pattern: {}", e));
                    return;
                }
            }
        }
        
        // Parse exclude patterns
        if !self.exclude_patterns.is_empty() {
            // Add ! prefix to each line if not already present
            let exclude_patterns = self.exclude_patterns
                .lines()
                .map(|line| {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        line.to_string()
                    } else if !line.starts_with('!') {
                        format!("!{}", line)
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            
            match new_filter.parse_patterns(&exclude_patterns) {
                Ok(_) => {},
                Err(e) => {
                    self.error_message = Some(format!("Invalid exclude pattern: {}", e));
                    return;
                }
            }
        }
        
        // Parse include extensions
        if !self.include_extensions.is_empty() {
            new_filter.parse_extensions(&self.include_extensions).unwrap();
        }
        
        // Parse exclude extensions
        if !self.exclude_extensions.is_empty() {
            // Add ! prefix to each extension if not already present
            let exclude_extensions = self.exclude_extensions
                .split(',')
                .map(|ext| {
                    let ext = ext.trim();
                    if ext.is_empty() {
                        ext.to_string()
                    } else if !ext.starts_with('!') {
                        format!("!{}", ext)
                    } else {
                        ext.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(",");
            
            new_filter.parse_extensions(&exclude_extensions).unwrap();
        }
        
        // Parse min size
        if !self.min_size.is_empty() {
            match parse_size(&self.min_size) {
                Ok(size) => new_filter.set_min_size(size),
                Err(e) => {
                    self.error_message = Some(format!("Invalid minimum size: {}", e));
                    return;
                }
            }
        }
        
        // Parse max size
        if !self.max_size.is_empty() {
            match parse_size(&self.max_size) {
                Ok(size) => new_filter.set_max_size(size),
                Err(e) => {
                    self.error_message = Some(format!("Invalid maximum size: {}", e));
                    return;
                }
            }
        }
        
        // Update the filter
        if let Ok(mut filter) = self.filter.lock() {
            *filter = new_filter;
            self.changes_applied = true;
        } else {
            self.error_message = Some("Failed to update filter".to_string());
        }
    }
    
    /// Clear all filters
    fn clear_filters(&mut self) {
        self.include_patterns.clear();
        self.exclude_patterns.clear();
        self.include_extensions.clear();
        self.exclude_extensions.clear();
        self.min_size.clear();
        self.max_size.clear();
        
        if let Ok(mut filter) = self.filter.lock() {
            filter.clear();
            self.changes_applied = true;
        } else {
            self.error_message = Some("Failed to clear filter".to_string());
        }
    }
    
    /// Check if changes were applied
    pub fn changes_applied(&self) -> bool {
        self.changes_applied
    }
}

/// Parse a size string like "10KB" or "1.5MB" into bytes
fn parse_size(size_str: &str) -> Result<u64, String> {
    let size_str = size_str.trim().to_uppercase();
    
    if size_str.is_empty() {
        return Err("Empty size string".to_string());
    }
    
    // Check for units
    if size_str.ends_with("KB") {
        let num_str = size_str.trim_end_matches("KB").trim();
        let num = num_str.parse::<f64>().map_err(|e| e.to_string())?;
        return Ok((num * 1024.0) as u64);
    } else if size_str.ends_with("MB") {
        let num_str = size_str.trim_end_matches("MB").trim();
        let num = num_str.parse::<f64>().map_err(|e| e.to_string())?;
        return Ok((num * 1024.0 * 1024.0) as u64);
    } else if size_str.ends_with("GB") {
        let num_str = size_str.trim_end_matches("GB").trim();
        let num = num_str.parse::<f64>().map_err(|e| e.to_string())?;
        return Ok((num * 1024.0 * 1024.0 * 1024.0) as u64);
    } else if size_str.ends_with("TB") {
        let num_str = size_str.trim_end_matches("TB").trim();
        let num = num_str.parse::<f64>().map_err(|e| e.to_string())?;
        return Ok((num * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64);
    } else {
        // Assume bytes
        let num = size_str.parse::<u64>().map_err(|e| e.to_string())?;
        return Ok(num);
    }
}
