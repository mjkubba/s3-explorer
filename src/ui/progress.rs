use eframe::egui;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

/// Maximum number of progress entries to keep in history
const MAX_PROGRESS_HISTORY: usize = 100;

/// Progress information for a file operation
#[derive(Clone, Debug)]
pub struct ProgressInfo {
    pub file_name: String,
    pub operation: OperationType,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub percentage: f32,
    pub status: ProgressStatus,
    pub timestamp: Instant,
    pub message: Option<String>,
}

/// Type of operation being performed
#[derive(Clone, Debug, PartialEq)]
pub enum OperationType {
    Upload,
    Download,
    Delete,
    Scan,
}

/// Status of a progress entry
#[derive(Clone, Debug, PartialEq)]
pub enum ProgressStatus {
    InProgress,
    Completed,
    Failed,
}

/// Progress tracker for sync operations
#[derive(Clone)]
pub struct ProgressTracker {
    entries: Arc<Mutex<VecDeque<ProgressInfo>>>,
    active_operations: Arc<Mutex<usize>>,
    total_operations: Arc<Mutex<usize>>,
    bytes_transferred: Arc<Mutex<u64>>,
    total_bytes: Arc<Mutex<u64>>,
    start_time: Arc<Mutex<Option<Instant>>>,
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_PROGRESS_HISTORY))),
            active_operations: Arc::new(Mutex::new(0)),
            total_operations: Arc::new(Mutex::new(0)),
            bytes_transferred: Arc::new(Mutex::new(0)),
            total_bytes: Arc::new(Mutex::new(0)),
            start_time: Arc::new(Mutex::new(None)),
        }
    }
}

impl ProgressTracker {
    /// Start a new sync operation
    pub fn start_sync(&self, total_operations: usize, total_bytes: u64) {
        let mut active = self.active_operations.lock().unwrap();
        let mut total_ops = self.total_operations.lock().unwrap();
        let mut total = self.total_bytes.lock().unwrap();
        let mut start = self.start_time.lock().unwrap();
        let mut entries = self.entries.lock().unwrap();
        
        *active = 0;
        *total_ops = total_operations;
        *total = total_bytes;
        *start = Some(Instant::now());
        entries.clear();
    }
    
    /// Add a new progress entry
    pub fn add_entry(&self, entry: ProgressInfo) {
        let mut entries = self.entries.lock().unwrap();
        let mut active = self.active_operations.lock().unwrap();
        let mut bytes = self.bytes_transferred.lock().unwrap();
        
        // Update active operations count
        match entry.status {
            ProgressStatus::InProgress => {
                *active += 1;
            },
            ProgressStatus::Completed => {
                if *active > 0 {
                    *active -= 1;
                }
                *bytes += entry.bytes_transferred;
            },
            ProgressStatus::Failed => {
                if *active > 0 {
                    *active -= 1;
                }
            },
        }
        
        // Add entry to the queue
        entries.push_back(entry);
        
        // Trim the queue if it gets too large
        while entries.len() > MAX_PROGRESS_HISTORY {
            entries.pop_front();
        }
    }
    
    /// Update an existing progress entry
    pub fn update_entry(&self, file_name: &str, bytes_transferred: u64, percentage: f32) {
        let mut entries = self.entries.lock().unwrap();
        
        // Find the entry for this file
        for entry in entries.iter_mut().rev() {
            if entry.file_name == file_name && entry.status == ProgressStatus::InProgress {
                entry.bytes_transferred = bytes_transferred;
                entry.percentage = percentage;
                break;
            }
        }
    }
    
    /// Mark an operation as completed
    pub fn complete_operation(&self, file_name: &str, bytes_transferred: u64) {
        let mut entries = self.entries.lock().unwrap();
        let mut active = self.active_operations.lock().unwrap();
        let mut bytes = self.bytes_transferred.lock().unwrap();
        
        // Find the entry for this file
        for entry in entries.iter_mut().rev() {
            if entry.file_name == file_name && entry.status == ProgressStatus::InProgress {
                entry.status = ProgressStatus::Completed;
                entry.bytes_transferred = bytes_transferred;
                entry.percentage = 100.0;
                
                // Update counters
                if *active > 0 {
                    *active -= 1;
                }
                *bytes += bytes_transferred;
                break;
            }
        }
    }
    
    /// Mark an operation as failed
    pub fn fail_operation(&self, file_name: &str, message: &str) {
        let mut entries = self.entries.lock().unwrap();
        let mut active = self.active_operations.lock().unwrap();
        
        // Find the entry for this file
        for entry in entries.iter_mut().rev() {
            if entry.file_name == file_name && entry.status == ProgressStatus::InProgress {
                entry.status = ProgressStatus::Failed;
                entry.message = Some(message.to_string());
                
                // Update counters
                if *active > 0 {
                    *active -= 1;
                }
                break;
            }
        }
    }
    
    /// Get overall progress percentage
    pub fn get_overall_progress(&self) -> f32 {
        let total_ops = *self.total_operations.lock().unwrap();
        let active = *self.active_operations.lock().unwrap();
        let bytes = *self.bytes_transferred.lock().unwrap();
        let total_bytes = *self.total_bytes.lock().unwrap();
        
        if total_ops == 0 {
            return 0.0;
        }
        
        let completed_ops = total_ops - active;
        
        if total_bytes > 0 {
            (bytes as f32 / total_bytes as f32) * 100.0
        } else {
            (completed_ops as f32 / total_ops as f32) * 100.0
        }
    }
    
    /// Get estimated time remaining
    pub fn get_eta(&self) -> Option<Duration> {
        let start = *self.start_time.lock().unwrap();
        let bytes = *self.bytes_transferred.lock().unwrap();
        let total_bytes = *self.total_bytes.lock().unwrap();
        
        if let Some(start_time) = start {
            if bytes > 0 && total_bytes > 0 {
                let elapsed = start_time.elapsed();
                let bytes_per_second = bytes as f64 / elapsed.as_secs_f64();
                
                if bytes_per_second > 0.0 {
                    let remaining_bytes = total_bytes - bytes;
                    let remaining_seconds = remaining_bytes as f64 / bytes_per_second;
                    return Some(Duration::from_secs_f64(remaining_seconds));
                }
            }
        }
        
        None
    }
    
    /// Get recent progress entries
    pub fn get_entries(&self) -> Vec<ProgressInfo> {
        let entries = self.entries.lock().unwrap();
        entries.iter().cloned().collect()
    }
    
    /// Check if sync is complete
    pub fn is_complete(&self) -> bool {
        let active = *self.active_operations.lock().unwrap();
        let total_ops = *self.total_operations.lock().unwrap();
        
        total_ops > 0 && active == 0
    }
}

/// UI component for displaying progress
pub struct ProgressView {
    tracker: ProgressTracker,
    show_details: bool,
}

impl Default for ProgressView {
    fn default() -> Self {
        Self {
            tracker: ProgressTracker::default(),
            show_details: false,
        }
    }
}

impl ProgressView {
    /// Create a new progress view with the given tracker
    pub fn new(tracker: ProgressTracker) -> Self {
        Self {
            tracker,
            show_details: false,
        }
    }
    
    /// Get the progress tracker
    pub fn tracker(&self) -> ProgressTracker {
        self.tracker.clone()
    }
    
    /// Render the progress UI
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let progress = self.tracker.get_overall_progress();
        
        // Overall progress bar
        ui.label("Overall Progress:");
        let progress_bar = egui::ProgressBar::new(progress / 100.0)
            .show_percentage()
            .animate(true);
        ui.add(progress_bar);
        
        // ETA
        if let Some(eta) = self.tracker.get_eta() {
            let eta_secs = eta.as_secs();
            if eta_secs < 60 {
                ui.label(format!("ETA: {} seconds", eta_secs));
            } else if eta_secs < 3600 {
                ui.label(format!("ETA: {} minutes, {} seconds", eta_secs / 60, eta_secs % 60));
            } else {
                ui.label(format!("ETA: {} hours, {} minutes", eta_secs / 3600, (eta_secs % 3600) / 60));
            }
        }
        
        // Toggle details
        if ui.button(if self.show_details { "Hide Details" } else { "Show Details" }).clicked() {
            self.show_details = !self.show_details;
        }
        
        // Show detailed progress
        if self.show_details {
            ui.separator();
            
            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                let entries = self.tracker.get_entries();
                
                for entry in entries.iter().rev() {
                    let status_color = match entry.status {
                        ProgressStatus::InProgress => egui::Color32::BLUE,
                        ProgressStatus::Completed => egui::Color32::GREEN,
                        ProgressStatus::Failed => egui::Color32::RED,
                    };
                    
                    let op_text = match entry.operation {
                        OperationType::Upload => "⬆️ Upload",
                        OperationType::Download => "⬇️ Download",
                        OperationType::Delete => "🗑️ Delete",
                        OperationType::Scan => "🔍 Scan",
                    };
                    
                    ui.horizontal(|ui| {
                        ui.colored_label(status_color, op_text);
                        ui.label(&entry.file_name);
                        
                        if entry.status == ProgressStatus::InProgress {
                            let file_progress = egui::ProgressBar::new(entry.percentage / 100.0)
                                .show_percentage()
                                .animate(true);
                            ui.add(file_progress);
                        } else if entry.status == ProgressStatus::Completed {
                            ui.label("✓ Complete");
                        } else if entry.status == ProgressStatus::Failed {
                            if let Some(msg) = &entry.message {
                                ui.label(format!("❌ Failed: {}", msg));
                            } else {
                                ui.label("❌ Failed");
                            }
                        }
                    });
                }
            });
        }
    }
}
