use std::sync::Arc;
use std::sync::mpsc;
use tokio::runtime::Handle;
use tokio::sync::Mutex as TokioMutex;

use crate::aws::auth::AwsAuth;
use crate::aws::transfer::TransferProgress;
use crate::config::credentials::CredentialManager;
use crate::ui::bucket_view::{BucketView, S3Object};
use crate::ui::folder_list::FolderList;
use crate::ui::folder_content::FolderContent;
use crate::ui::settings::SettingsView;
use crate::ui::progress::ProgressView;
use crate::ui::filter_view::FilterView;

/// Current view in the application
pub enum CurrentView {
    Main,
    Settings,
    Filter,
}

/// Status messages for communication between threads
pub enum StatusMessage {
    Info(String),
    Error(String),
    ObjectList(Vec<S3Object>),
    BucketList(Vec<String>),
    #[allow(dead_code)] // Will be used in future implementations
    Progress(TransferProgress),
    #[allow(dead_code)] // Will be used in future implementations
    SyncComplete,
}

/// Main application state
pub struct AppState {
    pub folder_list: FolderList,
    pub bucket_view: BucketView,
    pub folder_content: FolderContent,
    pub settings_view: SettingsView,
    pub progress_view: ProgressView,
    pub filter_view: Option<FilterView>,
    pub current_view: CurrentView,
    pub show_progress: bool,
    pub aws_auth: Arc<TokioMutex<AwsAuth>>,
    pub status_message: String,
    pub status_is_error: bool,
    pub status_tx: mpsc::Sender<StatusMessage>,
    pub status_rx: mpsc::Receiver<StatusMessage>,
    pub rt: Handle,
    #[allow(dead_code)] // Will be used in future implementations
    pub credential_manager: CredentialManager,
}

impl AppState {
    /// Set a status info message
    pub fn set_status_info(&mut self, message: &str) {
        self.status_message = message.to_string();
        self.status_is_error = false;
    }
    
    /// Set a status error message
    pub fn set_status_error(&mut self, message: &str) {
        self.status_message = message.to_string();
        self.status_is_error = true;
    }
}
