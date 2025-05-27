use eframe::egui;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Instant;
use log::debug;

/// Component for displaying progress information
#[derive(Default, Clone)]
pub struct ProgressView {
    tracker: Arc<Mutex<ProgressTracker>>,
}

/// Progress information for a file operation
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    pub file_name: String,
    pub operation_type: OperationType,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub percentage: f32,
    pub status: ProgressStatus,
    pub message: String,
    pub timestamp: Instant,
}

/// Type of operation being performed
#[derive(Debug, Clone, PartialEq)]
pub enum OperationType {
    Upload,
    Download,
    Delete,
    Scan,
}

/// Status of a progress entry
#[derive(Debug, Clone, PartialEq)]
pub enum ProgressStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

/// Tracker for progress information
#[derive(Default, Clone)]
pub struct ProgressTracker {
    entries: HashMap<String, ProgressInfo>,
    total_operations: usize,
    completed_operations: usize,
    total_bytes: u64,
    transferred_bytes: u64,
    start_time: Option<Instant>,
}

impl ProgressTracker {
    /// Start a new sync operation
    pub fn start_sync(&mut self, total_operations: usize, total_bytes: u64) {
        debug!("Starting sync operation with {} operations, {} bytes", total_operations, total_bytes);
        
        self.total_operations = total_operations;
        self.completed_operations = 0;
        self.total_bytes = total_bytes;
        self.transferred_bytes = 0;
        self.start_time = Some(Instant::now());
        self.entries.clear();
    }
    
    /// Add a new progress entry
    pub fn add_entry(&mut self, entry: ProgressInfo) {
        debug!("Adding progress entry for {}: {:?}", entry.file_name, entry.operation_type);
        self.entries.insert(entry.file_name.clone(), entry);
    }
    
    /// Get all progress entries
    pub fn entries(&self) -> Vec<ProgressInfo> {
        self.entries.values().cloned().collect()
    }
    
    /// Get the total number of operations
    pub fn total_operations(&self) -> usize {
        self.total_operations
    }
    
    /// Get the number of completed operations
    pub fn completed_operations(&self) -> usize {
        self.completed_operations
    }
    
    /// Get the total number of bytes
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }
    
    /// Get the number of transferred bytes
    pub fn transferred_bytes(&self) -> u64 {
        self.transferred_bytes
    }
    
    /// Get the overall percentage complete
    pub fn overall_percentage(&self) -> f32 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        
        (self.transferred_bytes as f32 / self.total_bytes as f32) * 100.0
    }
    
    /// Update a progress entry
    pub fn update_entry(&mut self, file_name: &str, bytes_transferred: u64, percentage: f32) {
        if let Some(entry) = self.entries.get_mut(file_name) {
            // Calculate the delta in bytes transferred
            let delta = bytes_transferred - entry.bytes_transferred;
            
            // Update the entry
            entry.bytes_transferred = bytes_transferred;
            entry.percentage = percentage;
            
            // Update the overall transferred bytes
            self.transferred_bytes += delta;
        }
    }
    
    /// Mark an operation as complete
    pub fn complete_operation(&mut self, file_name: &str, bytes_transferred: u64) {
        debug!("Completing operation for {}", file_name);
        
        if let Some(entry) = self.entries.get_mut(file_name) {
            // Calculate the delta in bytes transferred
            let delta = bytes_transferred - entry.bytes_transferred;
            
            // Update the entry
            entry.bytes_transferred = bytes_transferred;
            entry.percentage = 100.0;
            entry.status = ProgressStatus::Completed;
            
            // Update the overall transferred bytes
            self.transferred_bytes += delta;
            
            // Update the completed operations count
            self.completed_operations += 1;
        }
    }
    
    /// Mark an operation as failed
    pub fn fail_operation(&mut self, file_name: &str, message: &str) {
        debug!("Operation failed for {}: {}", file_name, message);
        
        if let Some(entry) = self.entries.get_mut(file_name) {
            // Update the entry
            entry.status = ProgressStatus::Failed(message.to_string());
            
            // Update the completed operations count
            self.completed_operations += 1;
        }
    }
    
    /// Get the elapsed time since the sync started
    pub fn elapsed_time(&self) -> Option<std::time::Duration> {
        self.start_time.map(|t| t.elapsed())
    }
    
    /// Format a duration as a string
    fn format_duration(duration: std::time::Duration) -> String {
        let total_seconds = duration.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        
        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
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
            format!("{:.2} KB", size as f32 / KB as f32)
        } else if size < GB {
            format!("{:.2} MB", size as f32 / MB as f32)
        } else {
            format!("{:.2} GB", size as f32 / GB as f32)
        }
    }
    
    /// Calculate the transfer rate in bytes per second
    fn transfer_rate(&self) -> Option<u64> {
        let elapsed = self.elapsed_time()?;
        
        if elapsed.as_secs() == 0 {
            return None;
        }
        
        Some(self.transferred_bytes / elapsed.as_secs())
    }
    
    /// Format the transfer rate as a human-readable string
    fn format_transfer_rate(&self) -> String {
        if let Some(rate) = self.transfer_rate() {
            format!("{}/s", Self::format_size(rate))
        } else {
            "N/A".to_string()
        }
    }
    
    /// Estimate the time remaining
    fn estimate_time_remaining(&self) -> Option<std::time::Duration> {
        let rate = self.transfer_rate()?;
        
        if rate == 0 || self.transferred_bytes >= self.total_bytes {
            return None;
        }
        
        let remaining_bytes = self.total_bytes - self.transferred_bytes;
        let remaining_seconds = remaining_bytes / rate;
        
        Some(std::time::Duration::from_secs(remaining_seconds))
    }
    
    /// Format the estimated time remaining as a string
    fn format_time_remaining(&self) -> String {
        if let Some(remaining) = self.estimate_time_remaining() {
            Self::format_duration(remaining)
        } else {
            "N/A".to_string()
        }
    }
    
    /// Check if all operations are complete
    pub fn is_complete(&self) -> bool {
        self.completed_operations >= self.total_operations && self.total_operations > 0
    }
}

impl ProgressView {
    /// Create a new progress view with the given tracker
    pub fn new() -> Self {
        Self {
            tracker: Arc::new(Mutex::new(ProgressTracker::default())),
        }
    }
    
    /// Render the progress view UI
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Transfer Progress");
        
        // Get a lock on the tracker
        let tracker = self.tracker.lock().unwrap();
        
        // Overall progress
        let total_ops = tracker.total_operations();
        let completed_ops = tracker.completed_operations();
        let percentage = tracker.overall_percentage();
        
        ui.horizontal(|ui| {
            ui.label(format!("Operations: {}/{}", completed_ops, total_ops));
            ui.separator();
            ui.label(format!("Progress: {:.1}%", percentage));
            ui.separator();
            
            if let Some(elapsed) = tracker.elapsed_time() {
                ui.label(format!("Elapsed: {}", ProgressTracker::format_duration(elapsed)));
                ui.separator();
            }
            
            ui.label(format!("Rate: {}", tracker.format_transfer_rate()));
            ui.separator();
            ui.label(format!("Remaining: {}", tracker.format_time_remaining()));
        });
        
        // Progress bar
        let progress = percentage / 100.0;
        ui.add(egui::ProgressBar::new(progress).show_percentage());
        
        ui.separator();
        
        // File list
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Table header
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("File").strong());
                ui.add_space(200.0);
                ui.label(egui::RichText::new("Operation").strong());
                ui.add_space(80.0);
                ui.label(egui::RichText::new("Progress").strong());
                ui.add_space(80.0);
                ui.label(egui::RichText::new("Status").strong());
            });
            
            ui.separator();
            
            // Table rows
            let entries = tracker.entries();
            
            if entries.is_empty() {
                ui.label("No operations in progress");
            } else {
                for entry in entries {
                    ui.horizontal(|ui| {
                        // File name
                        ui.label(&entry.file_name);
                        ui.add_space(200.0 - entry.file_name.len() as f32 * 7.0);
                        
                        // Operation type
                        let op_str = match entry.operation_type {
                            OperationType::Upload => "Upload",
                            OperationType::Download => "Download",
                            OperationType::Delete => "Delete",
                            OperationType::Scan => "Scan",
                        };
                        ui.label(op_str);
                        ui.add_space(80.0);
                        
                        // Progress
                        ui.label(format!("{:.1}%", entry.percentage));
                        ui.add_space(80.0);
                        
                        // Status
                        match entry.status {
                            ProgressStatus::Pending => ui.label("Pending"),
                            ProgressStatus::InProgress => ui.label("In Progress"),
                            ProgressStatus::Completed => ui.label(egui::RichText::new("Completed").color(egui::Color32::GREEN)),
                            ProgressStatus::Failed(ref msg) => ui.label(egui::RichText::new(format!("Failed: {}", msg)).color(egui::Color32::RED)),
                        };
                    });
                    
                    ui.separator();
                }
            }
        });
    }
    
    /// Start a new sync operation
    pub fn start_sync(&self, total_operations: usize, total_bytes: u64) {
        let mut tracker = self.tracker.lock().unwrap();
        tracker.start_sync(total_operations, total_bytes);
    }
    
    /// Add a new progress entry
    pub fn add_entry(&self, entry: ProgressInfo) {
        let mut tracker = self.tracker.lock().unwrap();
        tracker.add_entry(entry);
    }
    
    /// Update a progress entry
    pub fn update_entry(&self, file_name: &str, bytes_transferred: u64, percentage: f32) {
        let mut tracker = self.tracker.lock().unwrap();
        tracker.update_entry(file_name, bytes_transferred, percentage);
    }
    
    /// Mark an operation as complete
    pub fn complete_operation(&self, file_name: &str, bytes_transferred: u64) {
        let mut tracker = self.tracker.lock().unwrap();
        tracker.complete_operation(file_name, bytes_transferred);
    }
    
    /// Mark an operation as failed
    pub fn fail_operation(&self, file_name: &str, message: &str) {
        let mut tracker = self.tracker.lock().unwrap();
        tracker.fail_operation(file_name, message);
    }
    
    /// Check if all operations are complete
    pub fn is_complete(&self) -> bool {
        let tracker = self.tracker.lock().unwrap();
        tracker.is_complete()
    }
    
    /// Get a clone of this progress view
    pub fn clone(&self) -> Self {
        Self {
            tracker: self.tracker.clone(),
        }
    }
    
    /// Add a file to track
    pub fn add_file(&self, file_name: &str, size: u64) {
        let entry = ProgressInfo {
            file_name: file_name.to_string(),
            operation_type: OperationType::Upload, // Default to upload
            bytes_transferred: 0,
            total_bytes: size,
            percentage: 0.0,
            status: ProgressStatus::Pending,
            message: String::new(),
            timestamp: Instant::now(),
        };
        
        self.add_entry(entry);
    }
    
    /// Mark a file as complete
    pub fn complete_file(&self, file_name: &str) {
        let mut tracker = self.tracker.lock().unwrap();
        if let Some(entry) = tracker.entries.get_mut(file_name) {
            // Update the entry directly
            entry.bytes_transferred = entry.total_bytes;
            entry.percentage = 100.0;
            entry.status = ProgressStatus::Completed;
            
            // Update the completed operations count
            tracker.completed_operations += 1;
        }
    }
    
    /// Mark a file as failed
    pub fn fail_file(&self, file_name: &str) {
        self.fail_operation(file_name, "Transfer failed");
    }
    
    /// Mark the sync operation as complete
    pub fn complete_sync(&self) {
        // Nothing to do here, just a placeholder for the API
    }
    
    /// Update progress for a file
    pub fn update_progress(&self, progress: crate::aws::transfer::TransferProgress) {
        self.update_entry(
            &progress.file_name,
            progress.bytes_transferred,
            progress.percentage
        );
    }
    
    /// Show the progress view as a modal overlay
    pub fn show(&self, ctx: &egui::Context) {
        let mut open = true;
        
        egui::Window::new("Transfer Progress")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([600.0, 400.0])
            .show(ctx, |ui| {
                let mut view = self.clone();
                view.ui(ui);
            });
    }
}
