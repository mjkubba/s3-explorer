use eframe::egui;

/// Component for viewing and interacting with S3 buckets
#[derive(Default)]
pub struct BucketView {
    buckets: Vec<String>,
    selected_bucket: Option<String>,
    objects: Vec<S3Object>,
    filter: String,
}

/// Represents an object in an S3 bucket
struct S3Object {
    key: String,
    size: u64,
    last_modified: String,
    is_directory: bool,
}

impl BucketView {
    /// Render the bucket view UI
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("S3 Buckets");
        
        // Bucket selection
        egui::ComboBox::from_label("Select Bucket")
            .selected_text(self.selected_bucket.as_deref().unwrap_or("No bucket selected"))
            .show_ui(ui, |ui| {
                for bucket in &self.buckets {
                    ui.selectable_value(&mut self.selected_bucket, Some(bucket.clone()), bucket);
                }
            });
            
        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked() {
                // TODO: Implement bucket refresh
            }
            
            if ui.button("Create Bucket").clicked() {
                // TODO: Implement bucket creation
            }
        });
        
        ui.separator();
        
        // Filter
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter);
            
            if ui.button("Clear").clicked() {
                self.filter.clear();
            }
        });
        
        // Object list
        ui.separator();
        
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
            for object in &self.objects {
                if !self.filter.is_empty() && !object.key.contains(&self.filter) {
                    continue;
                }
                
                ui.horizontal(|ui| {
                    let icon = if object.is_directory { "📁 " } else { "📄 " };
                    ui.label(format!("{}{}", icon, object.key));
                    ui.add_space(200.0 - object.key.len() as f32 * 7.0);
                    
                    let size_str = if object.is_directory {
                        "-".to_string()
                    } else {
                        format_size(object.size)
                    };
                    
                    ui.label(size_str);
                    ui.add_space(100.0);
                    ui.label(&object.last_modified);
                });
                
                ui.separator();
            }
        });
    }
    
    /// Load buckets from AWS
    pub fn load_buckets(&mut self) {
        // TODO: Implement actual AWS S3 bucket loading
        // For now, add some dummy data
        self.buckets = vec![
            "my-backup-bucket".to_string(),
            "my-photos".to_string(),
            "my-documents".to_string(),
        ];
    }
    
    /// Load objects from the selected bucket
    pub fn load_objects(&mut self, bucket: &str) {
        // TODO: Implement actual AWS S3 object loading
        // For now, add some dummy data
        self.objects = vec![
            S3Object {
                key: "Documents/".to_string(),
                size: 0,
                last_modified: "2023-10-15 14:30:22".to_string(),
                is_directory: true,
            },
            S3Object {
                key: "Photos/".to_string(),
                size: 0,
                last_modified: "2023-10-14 09:15:10".to_string(),
                is_directory: true,
            },
            S3Object {
                key: "backup.zip".to_string(),
                size: 1_500_000,
                last_modified: "2023-10-10 18:45:33".to_string(),
                is_directory: false,
            },
            S3Object {
                key: "notes.txt".to_string(),
                size: 2_500,
                last_modified: "2023-10-16 11:22:05".to_string(),
                is_directory: false,
            },
        ];
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
