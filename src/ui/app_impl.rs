use eframe::egui;
use eframe::epi;
// use log::debug;
use std::sync::Arc;
use std::sync::mpsc;
use tokio::runtime::Handle;
use tokio::sync::Mutex as TokioMutex;

use crate::aws::auth::AwsAuth;
use crate::config::credentials::CredentialManager;
use crate::ui::app_state::{AppState, CurrentView, StatusMessage};
// use crate::ui::aws_operations::AwsOperations;
use crate::ui::bucket_view::BucketView;
use crate::ui::filter_view_renderer::FilterViewRenderer;
use crate::ui::folder_content::FolderContent;
use crate::ui::folder_list::FolderList;
use crate::ui::main_view_renderer::MainViewRenderer;
use crate::ui::menu_bar_renderer::MenuBarRenderer;
use crate::ui::progress::ProgressView;
use crate::ui::settings::SettingsView;
use crate::ui::settings_view_renderer::SettingsViewRenderer;
use crate::ui::status_bar_renderer::StatusBarRenderer;

/// Main application implementation
pub struct S3SyncApp {
    state: AppState,
}

impl Default for S3SyncApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        
        // Create the app instance
        let mut app = Self {
            state: AppState {
                folder_list: FolderList::default(),
                bucket_view: BucketView::default(),
                folder_content: FolderContent::default(),
                settings_view: SettingsView::default(),
                progress_view: ProgressView::default(),
                filter_view: None,
                current_view: CurrentView::Main,
                show_progress: false,
                aws_auth: Arc::new(TokioMutex::new(AwsAuth::default())),
                status_message: String::new(),
                status_is_error: false,
                status_tx: tx,
                status_rx: rx,
                rt: Handle::current(),
                credential_manager: CredentialManager::default(),
            }
        };
        
        // Try to load credentials from the system keyring
        if CredentialManager::has_credentials() {
            match (
                CredentialManager::load_access_key(),
                CredentialManager::load_secret_key(),
                CredentialManager::load_region()
            ) {
                (Ok(access_key), Ok(secret_key), Ok(region)) if !access_key.is_empty() && !secret_key.is_empty() => {
                    // Update the settings view with the loaded credentials
                    app.state.settings_view.set_aws_access_key(access_key.clone());
                    app.state.settings_view.set_aws_secret_key(secret_key.clone());
                    app.state.settings_view.set_aws_region(region.clone());
                    
                    // Update AWS auth with the loaded credentials
                    let auth_clone = app.state.aws_auth.clone();
                    let access_key_clone = access_key.clone();
                    let secret_key_clone = secret_key.clone();
                    let region_clone = region.clone();
                    
                    // Use a blocking task to set the credentials
                    tokio::task::block_in_place(|| {
                        app.state.rt.block_on(async {
                            let mut auth = auth_clone.lock().await;
                            auth.set_credentials(access_key_clone, secret_key_clone, region_clone);
                        });
                    });
                    
                    app.state.status_message = format!("Loaded credentials from keyring for region {}", region);
                },
                _ => {
                    // No credentials found or error loading them
                    app.state.status_message = "No saved credentials found. Please enter your AWS credentials in Settings.".to_string();
                }
            }
        }
        
        app
    }
}

impl epi::App for S3SyncApp {
    fn name(&self) -> &str {
        "S3 Sync"
    }
    
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        // Process any status messages
        self.process_status_messages();
        
        // Show progress view if needed
        if self.state.show_progress {
            self.state.progress_view.show(ctx);
        }
        
        // Render the menu bar
        MenuBarRenderer::render(&mut self.state, ctx);
        
        // Render the status bar
        StatusBarRenderer::render(&mut self.state, ctx);
        
        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.state.current_view {
                CurrentView::Main => {
                    MainViewRenderer::render(&mut self.state, ui);
                },
                CurrentView::Filter => FilterViewRenderer::render(&mut self.state, ui),
                CurrentView::Settings => SettingsViewRenderer::render(&mut self.state, ui),
            }
        });
    }
}

impl S3SyncApp {
    /// Process any status messages in the queue
    fn process_status_messages(&mut self) {
        while let Ok(msg) = self.state.status_rx.try_recv() {
            match msg {
                StatusMessage::Info(text) => {
                    self.state.status_message = text;
                    self.state.status_is_error = false;
                },
                StatusMessage::Error(text) => {
                    self.state.status_message = text;
                    self.state.status_is_error = true;
                },
                StatusMessage::ObjectList(objects) => {
                    self.state.bucket_view.set_objects(objects);
                    self.state.status_message = format!("Loaded {} objects", self.state.bucket_view.objects().len());
                },
                StatusMessage::BucketList(buckets) => {
                    self.state.bucket_view.set_buckets(buckets);
                },
                StatusMessage::Progress(progress) => {
                    self.state.progress_view.update_progress(progress);
                },
                StatusMessage::SyncComplete => {
                    self.state.show_progress = false;
                }
            }
        }
    }
}
