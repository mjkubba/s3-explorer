use std::path::PathBuf;

/// Action to take for a file
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Will be used in future implementations
pub enum FileAction {
    /// Upload file to S3
    Upload,
    /// Download file from S3
    Download,
    /// Delete file (either locally or from S3)
    Delete,
    /// No action needed
    None,
}

/// Represents a difference between local and S3 files
#[derive(Debug, Clone)]
#[allow(dead_code)] // Will be used in future implementations
pub struct FileDiff {
    /// Action to take
    pub action: FileAction,
    /// Local file path (if applicable)
    pub local_path: Option<PathBuf>,
    /// S3 object key (if applicable)
    pub s3_key: Option<String>,
}

/// Calculate file hash for comparison
#[allow(dead_code)] // Will be used in future implementations
pub fn calculate_file_hash(path: &PathBuf) -> Result<String, std::io::Error> {
    use sha2::{Sha256, Digest};
    use std::fs::File;
    use std::io::Read;
    
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024];
    
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;
    
    #[test]
    fn test_calculate_file_hash() {
        // Create a temporary directory
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        
        // Create a test file
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Hello, world!").unwrap();
        
        // Calculate hash
        let hash = calculate_file_hash(&file_path).unwrap();
        
        // Expected SHA-256 hash of "Hello, world!"
        let expected = "315f5bdb76d078c43b8ac0064e4a0164612b1fce77c869345bfc94c75894edd3";
        
        assert_eq!(hash, expected);
    }
}
