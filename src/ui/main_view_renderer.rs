use eframe::egui;
use log::debug;

use crate::ui::app_state::AppState;
use crate::ui::aws_operations::AwsOperations;
use crate::ui::utils::format_size;

/// Renderer for the main view of the application
pub struct MainViewRenderer;

impl MainViewRenderer {
    /// Render the main view
    pub fn render(app_state: &mut AppState, ui: &mut egui::Ui) {
        // Main view with folder list, bucket view, and folder content
        ui.horizontal(|ui| {
            Self::render_left_panel(app_state, ui);
            ui.separator();
            Self::render_right_panel(app_state, ui);
        });
    }

    /// Render the left panel with folder list and bucket view
    fn render_left_panel(app_state: &mut AppState, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.set_width(250.0);
            
            // Local folders section
            ui.group(|ui| {
                ui.heading("Local Folders");
                app_state.folder_list.ui(ui);
                
                ui.separator();
                
                ui.horizontal(|ui| {
                    if ui.button("Add Folder").clicked() {
                        app_state.folder_list.show_folder_dialog();
                    }
                    
                    if ui.button("Remove").clicked() {
                        app_state.folder_list.remove_selected();
                    }
                });
            });
            
            ui.add_space(10.0);
            
            // S3 buckets section
            ui.group(|ui| {
                ui.heading("S3 Buckets");
                if app_state.bucket_view.ui(ui) {
                    // Bucket selection changed, load objects
                    if let Some(bucket) = app_state.bucket_view.selected_bucket() {
                        AwsOperations::load_bucket_objects(app_state, &bucket);
                    }
                }
                
                ui.separator();
                
                if ui.button("Connect to AWS").clicked() {
                    AwsOperations::connect_to_aws(app_state);
                }
            });
        });
    }

    /// Render the right panel with content views
    fn render_right_panel(app_state: &mut AppState, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.set_min_width(600.0);
            
            // Create a top section for controls
            ui.horizontal(|ui| {
                if let Some(bucket) = app_state.bucket_view.selected_bucket() {
                    ui.heading(&format!("Bucket: {}", bucket));
                    
                    ui.with_layout(egui::Layout::right_to_left(), |ui| {
                        if ui.button("Upload").clicked() {
                            AwsOperations::upload_selected(app_state);
                        }
                        
                        if ui.button("Download").clicked() {
                            AwsOperations::download_selected(app_state);
                        }
                        
                        if ui.button("Refresh").clicked() {
                            if let Some(bucket) = app_state.bucket_view.selected_bucket() {
                                AwsOperations::load_bucket_objects(app_state, &bucket);
                            }
                        }
                    });
                } else {
                    ui.heading("No S3 bucket selected");
                }
            });
            
            ui.separator();
            
            // Split the view into two panels
            egui::TopBottomPanel::top("bucket_content_panel")
                .resizable(true)
                .default_height(300.0)
                .show_inside(ui, |ui| {
                    Self::render_bucket_content(app_state, ui);
                });
            
            // Local folder content in the bottom panel
            ui.separator();
            Self::render_folder_content(app_state, ui);
        });
    }

    /// Render the bucket content section
    fn render_bucket_content(app_state: &mut AppState, ui: &mut egui::Ui) {
        // S3 bucket contents (if a bucket is selected)
        if let Some(_) = app_state.bucket_view.selected_bucket() {
            // Create a table header for bucket contents
            ui.horizontal(|ui| {
                ui.style_mut().spacing.item_spacing.x = 10.0;
                ui.strong("Type");
                ui.strong("Name");
                ui.add_space(200.0);
                ui.strong("Size");
                ui.add_space(50.0);
                ui.strong("Last Modified");
            });
            
            ui.separator();
            
            // Display bucket objects in a scrollable area
            egui::ScrollArea::vertical()
                .id_source("bucket_contents_scroll")
                .show(ui, |ui| {
                    let objects = app_state.bucket_view.objects();
                    
                    if objects.is_empty() {
                        ui.label("No objects in this bucket");
                    } else {
                        // Add each object as a row in the table
                        for object in objects {
                            // Use a container for each row
                            egui::containers::Frame::none()
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.style_mut().spacing.item_spacing.x = 10.0;
                                        
                                        // Type icon
                                        let icon = if object.is_directory { "📁" } else { "📄" };
                                        ui.label(icon);
                                        
                                        // Name
                                        let name_len = object.key.len();
                                        ui.label(&object.key);
                                        ui.add_space(200.0 - name_len as f32 * 7.0); // Approximate spacing
                                        
                                        // Size
                                        let size_text = if object.is_directory {
                                            "-".to_string()
                                        } else {
                                            format_size(object.size)
                                        };
                                        ui.label(&size_text);
                                        ui.add_space(50.0);
                                        
                                        // Last Modified
                                        ui.label(&object.last_modified);
                                    });
                                });
                            
                            // Add some spacing between rows
                            ui.add_space(2.0);
                        }
                    }
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Please select a bucket from the list on the left");
            });
        }
    }

    /// Render the folder content section
    fn render_folder_content(app_state: &mut AppState, ui: &mut egui::Ui) {
        // Local folder contents (if a folder is selected)
        ui.horizontal(|ui| {
            if let Some(folder_path) = app_state.folder_list.selected_folder() {
                ui.heading(&format!("Local Folder: {}", folder_path.display()));
                
                ui.with_layout(egui::Layout::right_to_left(), |ui| {
                    if ui.button("Refresh Folder").clicked() {
                        if let Some(path) = &app_state.folder_content.current_folder {
                            let path_clone = path.clone();
                            app_state.folder_content.load_files(path_clone);
                        }
                    }
                });
            } else {
                ui.heading("No local folder selected");
            }
        });
        
        ui.separator();
        
        if let Some(folder_path) = app_state.folder_list.selected_folder() {
            // Set the folder in the folder_content component and ensure files are loaded
            if app_state.folder_content.current_folder.as_ref() != Some(folder_path) {
                debug!("Loading folder contents for: {}", folder_path.display());
                app_state.folder_content.set_folder(folder_path.clone());
            }
            
            // Display files directly here for debugging
            let files = app_state.folder_content.files().to_vec(); // Clone the files to avoid borrow issues
            debug!("Found {} files in folder", files.len());
            
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
            
            // Display files in a scrollable area
            egui::ScrollArea::vertical().id_source("local_folder_scroll").show(ui, |ui| {
                if files.is_empty() {
                    ui.label("No files in this folder or unable to access folder contents");
                    
                    // Add a refresh button
                    if ui.button("Refresh").clicked() {
                        if let Some(path) = &app_state.folder_content.current_folder {
                            let path_clone = path.clone();
                            app_state.folder_content.load_files(path_clone);
                        }
                    }
                } else {
                    for file in files {
                        ui.horizontal(|ui| {
                            ui.style_mut().spacing.item_spacing.x = 10.0;
                            
                            // Type icon
                            let icon = if file.is_directory { "📁" } else { "📄" };
                            ui.label(icon);
                            
                            // Name
                            ui.label(&file.name);
                            ui.add_space(200.0 - file.name.len() as f32 * 7.0);
                            
                            // Size
                            let size_text = if file.is_directory {
                                "--".to_string()
                            } else {
                                format_size(file.size)
                            };
                            ui.label(&size_text);
                            ui.add_space(50.0);
                            
                            // Last Modified
                            ui.label(&file.last_modified);
                        });
                    }
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Please select a folder from the list on the left");
            });
        }
    }
}
