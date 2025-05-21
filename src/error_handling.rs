use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;
use log::error;

/// Custom error types for the application
#[derive(Debug)]
pub enum AppError {
    /// AWS-related errors
    AwsError {
        message: String,
        source: Option<Box<dyn Error + Send + Sync>>,
    },
    
    /// File system errors
    FileSystemError {
        path: PathBuf,
        message: String,
        source: Option<Box<dyn Error + Send + Sync>>,
    },
    
    /// Configuration errors
    ConfigError {
        message: String,
        source: Option<Box<dyn Error + Send + Sync>>,
    },
    
    /// Sync operation errors
    SyncError {
        message: String,
        source: Option<Box<dyn Error + Send + Sync>>,
    },
    
    /// Authentication errors
    AuthError {
        message: String,
        source: Option<Box<dyn Error + Send + Sync>>,
    },
    
    /// Network errors
    NetworkError {
        message: String,
        source: Option<Box<dyn Error + Send + Sync>>,
    },
    
    /// Other errors
    OtherError {
        message: String,
        source: Option<Box<dyn Error + Send + Sync>>,
    },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::AwsError { message, .. } => write!(f, "AWS Error: {}", message),
            AppError::FileSystemError { path, message, .. } => {
                write!(f, "File System Error ({}): {}", path.display(), message)
            }
            AppError::ConfigError { message, .. } => write!(f, "Configuration Error: {}", message),
            AppError::SyncError { message, .. } => write!(f, "Sync Error: {}", message),
            AppError::AuthError { message, .. } => write!(f, "Authentication Error: {}", message),
            AppError::NetworkError { message, .. } => write!(f, "Network Error: {}", message),
            AppError::OtherError { message, .. } => write!(f, "Error: {}", message),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AppError::AwsError { source, .. } => source.as_ref().map(|s| s.as_ref() as &(dyn Error + 'static)),
            AppError::FileSystemError { source, .. } => source.as_ref().map(|s| s.as_ref() as &(dyn Error + 'static)),
            AppError::ConfigError { source, .. } => source.as_ref().map(|s| s.as_ref() as &(dyn Error + 'static)),
            AppError::SyncError { source, .. } => source.as_ref().map(|s| s.as_ref() as &(dyn Error + 'static)),
            AppError::AuthError { source, .. } => source.as_ref().map(|s| s.as_ref() as &(dyn Error + 'static)),
            AppError::NetworkError { source, .. } => source.as_ref().map(|s| s.as_ref() as &(dyn Error + 'static)),
            AppError::OtherError { source, .. } => source.as_ref().map(|s| s.as_ref() as &(dyn Error + 'static)),
        }
    }
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::FileSystemError {
            path: PathBuf::new(),
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

/// Error handler for the application
pub struct ErrorHandler;

impl ErrorHandler {
    /// Handle an error and return a user-friendly message
    pub fn handle_error<E: Error + 'static>(err: E) -> String {
        // Log the error
        error!("Error: {}", err);
        
        // Check for specific error types
        if let Some(app_err) = (&err as &dyn Error).downcast_ref::<AppError>() {
            return app_err.to_string();
        }
        
        // Check for AWS SDK errors
        let err_str = err.to_string();
        if err_str.contains("service error") {
            if err_str.contains("PermanentRedirect") {
                return "AWS Error: The bucket is in a different region than expected. The application will automatically handle this.".to_string();
            } else if err_str.contains("AccessDenied") {
                return "AWS Error: Access denied. Please check your IAM permissions.".to_string();
            } else if err_str.contains("NoSuchBucket") {
                return "AWS Error: The specified bucket does not exist.".to_string();
            } else if err_str.contains("InvalidToken") {
                return "AWS Error: Invalid credentials or token expired. Please update your credentials.".to_string();
            }
            
            return format!("AWS Error: {}", err_str);
        }
        
        // Check for file system errors
        if let Some(io_err) = (&err as &dyn Error).downcast_ref::<io::Error>() {
            match io_err.kind() {
                io::ErrorKind::NotFound => return "File not found. The file may have been moved or deleted.".to_string(),
                io::ErrorKind::PermissionDenied => return "Permission denied. Please check file permissions.".to_string(),
                io::ErrorKind::ConnectionRefused => return "Connection refused. Please check your network connection.".to_string(),
                io::ErrorKind::ConnectionReset => return "Connection reset. The server may be unavailable.".to_string(),
                io::ErrorKind::ConnectionAborted => return "Connection aborted. Please check your network connection.".to_string(),
                io::ErrorKind::NotConnected => return "Not connected. Please check your network connection.".to_string(),
                io::ErrorKind::TimedOut => return "Connection timed out. The server may be busy or unavailable.".to_string(),
                _ => return format!("File system error: {}", io_err),
            }
        }
        
        // Default error message
        format!("Error: {}", err)
    }
    
    /// Create an AWS error
    pub fn aws_error<E: Error + Send + Sync + 'static>(message: &str, source: E) -> AppError {
        AppError::AwsError {
            message: message.to_string(),
            source: Some(Box::new(source)),
        }
    }
    
    /// Create a file system error
    pub fn fs_error<E: Error + Send + Sync + 'static>(path: PathBuf, message: &str, source: E) -> AppError {
        AppError::FileSystemError {
            path,
            message: message.to_string(),
            source: Some(Box::new(source)),
        }
    }
    
    /// Create a configuration error
    pub fn config_error<E: Error + Send + Sync + 'static>(message: &str, source: E) -> AppError {
        AppError::ConfigError {
            message: message.to_string(),
            source: Some(Box::new(source)),
        }
    }
    
    /// Create a sync error
    pub fn sync_error<E: Error + Send + Sync + 'static>(message: &str, source: E) -> AppError {
        AppError::SyncError {
            message: message.to_string(),
            source: Some(Box::new(source)),
        }
    }
    
    /// Create an authentication error
    pub fn auth_error<E: Error + Send + Sync + 'static>(message: &str, source: E) -> AppError {
        AppError::AuthError {
            message: message.to_string(),
            source: Some(Box::new(source)),
        }
    }
    
    /// Create a network error
    pub fn network_error<E: Error + Send + Sync + 'static>(message: &str, source: E) -> AppError {
        AppError::NetworkError {
            message: message.to_string(),
            source: Some(Box::new(source)),
        }
    }
    
    /// Create a generic error
    pub fn other_error<E: Error + Send + Sync + 'static>(message: &str, source: E) -> AppError {
        AppError::OtherError {
            message: message.to_string(),
            source: Some(Box::new(source)),
        }
    }
    
    /// Create a simple error with no source
    pub fn simple_error(message: &str) -> AppError {
        AppError::OtherError {
            message: message.to_string(),
            source: None,
        }
    }
    
    /// Try an operation with retry logic
    pub async fn retry<F, T, E>(operation: F, retries: usize, delay_ms: u64) -> Result<T, E>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
        E: Error + Send + Sync + 'static,
    {
        let mut attempts = 0;
        let mut last_error = None;
        
        while attempts < retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    attempts += 1;
                    last_error = Some(err);
                    
                    if attempts < retries {
                        // Wait before retrying
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }
}
