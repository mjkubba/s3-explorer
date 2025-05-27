use eframe::egui;
use log::error;
use std::sync::Arc;

use crate::config::credentials::CredentialManager;
use crate::ui::app_state::{AppState, CurrentView};

/// Renderer for the settings view of the application
pub struct SettingsViewRenderer;

impl SettingsViewRenderer {
    /// Render the settings view
    pub fn render(app_state: &mut AppState, ui: &mut egui::Ui) {
        // Settings view
        if app_state.settings_view.ui(ui) {
            // Settings applied
            let settings = app_state.settings_view.get_settings();
            
            // Save credentials if requested
            if settings.save_credentials {
                CredentialManager::save_credentials(
                    &settings.aws_access_key,
                    &settings.aws_secret_key,
                    &settings.aws_region,
                ).unwrap_or_else(|e| {
                    error!("Failed to save credentials: {}", e);
                    app_state.set_status_error(&format!("Failed to save credentials: {}", e));
                });
            }
            
            // Update AWS auth
            let aws_auth = app_state.aws_auth.clone();
            let access_key = settings.aws_access_key.clone();
            let secret_key = settings.aws_secret_key.clone();
            let region = settings.aws_region.clone();
            
            app_state.rt.spawn(async move {
                let mut auth = aws_auth.lock().await;
                auth.set_credentials(access_key, secret_key, region);
            });
            
            // Return to main view
            app_state.current_view = CurrentView::Main;
        }
    }
}
