use eframe::egui;

/// Settings data structure
#[derive(Clone, Debug)]
pub struct Settings {
    pub aws_access_key: String,
    pub aws_secret_key: String,
    pub aws_region: String,
    pub sync_interval: u32,
    pub delete_enabled: bool,
    pub bandwidth_limit: Option<u32>,
    pub exclude_patterns: String,
    pub save_credentials: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            aws_access_key: String::new(),
            aws_secret_key: String::new(),
            aws_region: "us-east-1".to_string(),
            sync_interval: 0,
            delete_enabled: false,
            bandwidth_limit: None,
            exclude_patterns: String::new(),
            save_credentials: false,
        }
    }
}

/// Component for application settings
#[derive(Default)]
pub struct SettingsView {
    aws_access_key: String,
    aws_secret_key: String,
    aws_region: String,
    sync_interval: u32,
    delete_enabled: bool,
    bandwidth_limit: Option<u32>,
    exclude_patterns: String,
    save_credentials: bool,
    settings_applied: bool,
}

impl SettingsView {
    /// Render the settings UI and return true if settings were applied
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        ui.heading("Settings");
        
        // Reset the settings_applied flag
        self.settings_applied = false;
        
        egui::Grid::new("settings_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                // AWS Settings
                ui.heading("AWS Configuration");
                ui.end_row();
                
                ui.label("Access Key ID:");
                ui.text_edit_singleline(&mut self.aws_access_key);
                ui.end_row();
                
                ui.label("Secret Access Key:");
                ui.add(egui::TextEdit::singleline(&mut self.aws_secret_key).password(true));
                ui.end_row();
                
                ui.label("Region:");
                egui::ComboBox::from_label("")
                    .selected_text(&self.aws_region)
                    .show_ui(ui, |ui| {
                        for region in &["us-east-1", "us-west-1", "us-west-2", "eu-west-1", "ap-northeast-1"] {
                            ui.selectable_value(&mut self.aws_region, region.to_string(), *region);
                        }
                    });
                ui.end_row();
                
                ui.label("Save credentials:");
                ui.checkbox(&mut self.save_credentials, "Save AWS credentials securely");
                ui.end_row();
                
                ui.add_space(10.0);
                ui.end_row();
                
                // Sync Settings
                ui.heading("Sync Settings");
                ui.end_row();
                
                ui.label("Sync Interval (minutes):");
                ui.add(egui::Slider::new(&mut self.sync_interval, 0..=1440)
                    .text("min")
                    .clamp_to_range(true));
                ui.end_row();
                
                // Display manual/auto based on sync_interval
                ui.label("");
                if self.sync_interval == 0 {
                    ui.label("Manual sync only");
                } else {
                    ui.label(format!("Auto sync every {} minutes", self.sync_interval));
                }
                ui.end_row();
                
                ui.label("Delete files:");
                ui.checkbox(&mut self.delete_enabled, "Delete files in S3 that were deleted locally");
                ui.end_row();
                
                ui.label("Bandwidth Limit (KB/s):");
                ui.horizontal(|ui| {
                    let mut limit_enabled = self.bandwidth_limit.is_some();
                    ui.checkbox(&mut limit_enabled, "");
                    
                    let mut value = self.bandwidth_limit.unwrap_or(1024);
                    ui.add_enabled(
                        limit_enabled,
                        egui::Slider::new(&mut value, 64..=10240).text("KB/s")
                    );
                    
                    self.bandwidth_limit = if limit_enabled { Some(value) } else { None };
                });
                ui.end_row();
                
                ui.label("Exclude Patterns:");
                ui.text_edit_multiline(&mut self.exclude_patterns);
                ui.end_row();
            });
            
        ui.separator();
        
        let mut result = self.settings_applied;
        
        ui.horizontal(|ui| {
            if ui.button("Apply Settings").clicked() {
                self.settings_applied = true;
                result = true;
            }
            
            if ui.button("Cancel").clicked() {
                // Return to previous screen without applying settings
                result = true;
            }
        });
        
        result
    }
    
    /// Get the current settings
    pub fn get_settings(&self) -> Settings {
        Settings {
            aws_access_key: self.aws_access_key.clone(),
            aws_secret_key: self.aws_secret_key.clone(),
            aws_region: self.aws_region.clone(),
            sync_interval: self.sync_interval,
            delete_enabled: self.delete_enabled,
            bandwidth_limit: self.bandwidth_limit,
            exclude_patterns: self.exclude_patterns.clone(),
            save_credentials: self.save_credentials,
        }
    }
    
    /// Get the AWS access key
    pub fn aws_access_key(&self) -> String {
        self.aws_access_key.clone()
    }
    
    /// Get the AWS secret key
    pub fn aws_secret_key(&self) -> String {
        self.aws_secret_key.clone()
    }
    
    /// Get the AWS region
    pub fn aws_region(&self) -> String {
        self.aws_region.clone()
    }
    
    /// Set the AWS access key
    pub fn set_aws_access_key(&mut self, access_key: String) {
        self.aws_access_key = access_key;
    }
    
    /// Set the AWS secret key
    pub fn set_aws_secret_key(&mut self, secret_key: String) {
        self.aws_secret_key = secret_key;
    }
    
    /// Set the AWS region
    pub fn set_aws_region(&mut self, region: String) {
        self.aws_region = region;
    }
    
    /// Load settings from configuration
    pub fn load_settings(&mut self) {
        // TODO: Implement loading settings from config file
    }
    
    /// Save settings to configuration
    pub fn save_settings(&self) {
        // TODO: Implement saving settings to config file
    }
}
