use eframe::egui;
use std::sync::Arc;
use std::sync::Mutex;

use crate::sync::filter::FileFilter;
use crate::ui::app_state::{AppState, CurrentView};
use crate::ui::filter_view::FilterView;

/// Renderer for the filter view of the application
pub struct FilterViewRenderer;

impl FilterViewRenderer {
    /// Render the filter view
    pub fn render(app_state: &mut AppState, ui: &mut egui::Ui) {
        // Filter view
        let mut filter_view = app_state.filter_view.take().unwrap_or_else(|| {
            FilterView::new(Arc::new(Mutex::new(FileFilter::new())))
        });
        
        if filter_view.ui(ui) {
            // Filter changed
            let filter_string = filter_view.get_filter().lock().unwrap().to_string();
            app_state.folder_content.set_filter(filter_string.clone());
            app_state.bucket_view.set_filter(filter_string);
            
            // Return to main view
            app_state.current_view = CurrentView::Main;
        }
        
        app_state.filter_view = Some(filter_view);
    }
}
