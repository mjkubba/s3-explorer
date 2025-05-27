use anyhow::{anyhow, Result};
use glob::Pattern;
use log::{debug, error};
use std::fmt;
use std::path::Path;

/// Filter for files during sync operations
#[derive(Clone, Default)]
pub struct FileFilter {
    include_patterns: Vec<Pattern>,
    exclude_patterns: Vec<Pattern>,
    min_size: Option<u64>,
    max_size: Option<u64>,
}

impl fmt::Display for FileFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FileFilter with {} include patterns, {} exclude patterns", 
            self.include_patterns.len(), 
            self.exclude_patterns.len()
        )
    }
}

impl FileFilter {
    /// Create a new file filter
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Parse patterns from a string
    pub fn parse_patterns(&mut self, patterns: &str) -> Result<()> {
        for line in patterns.lines() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Check if this is an exclude pattern
            if line.starts_with('!') {
                let pattern = &line[1..];
                match Pattern::new(pattern) {
                    Ok(p) => self.exclude_patterns.push(p),
                    Err(e) => return Err(anyhow!("Invalid exclude pattern '{}': {}", pattern, e)),
                }
            } else {
                // Include pattern
                match Pattern::new(line) {
                    Ok(p) => self.include_patterns.push(p),
                    Err(e) => return Err(anyhow!("Invalid include pattern '{}': {}", line, e)),
                }
            }
        }
        
        Ok(())
    }
    
    /// Parse extensions from a comma-separated string
    pub fn parse_extensions(&mut self, extensions: &str) -> Result<()> {
        for ext in extensions.split(',') {
            let ext = ext.trim();
            
            // Skip empty extensions
            if ext.is_empty() {
                continue;
            }
            
            // Check if this is an exclude extension
            if ext.starts_with('!') {
                let pattern = format!("*.{}", &ext[1..]);
                match Pattern::new(&pattern) {
                    Ok(p) => self.exclude_patterns.push(p),
                    Err(e) => return Err(anyhow!("Invalid exclude extension '{}': {}", ext, e)),
                }
            } else {
                // Include extension
                let pattern = format!("*.{}", ext);
                match Pattern::new(&pattern) {
                    Ok(p) => self.include_patterns.push(p),
                    Err(e) => return Err(anyhow!("Invalid include extension '{}': {}", ext, e)),
                }
            }
        }
        
        Ok(())
    }
    
    /// Set the minimum file size
    pub fn set_min_size(&mut self, size: u64) {
        self.min_size = Some(size);
    }
    
    /// Set the maximum file size
    pub fn set_max_size(&mut self, size: u64) {
        self.max_size = Some(size);
    }
    
    /// Clear all filters
    pub fn clear(&mut self) {
        self.include_patterns.clear();
        self.exclude_patterns.clear();
        self.min_size = None;
        self.max_size = None;
    }
    
    /// Check if a file should be included
    pub fn should_include(&self, path: &Path, size: u64) -> bool {
        // Check size constraints
        if let Some(min_size) = self.min_size {
            if size < min_size {
                return false;
            }
        }
        
        if let Some(max_size) = self.max_size {
            if size > max_size {
                return false;
            }
        }
        
        // Convert path to string for pattern matching
        let path_str = path.to_string_lossy();
        
        // Check exclude patterns first
        for pattern in &self.exclude_patterns {
            if pattern.matches(&path_str) {
                debug!("Path {} excluded by pattern {}", path_str, pattern);
                return false;
            }
        }
        
        // If there are no include patterns, include everything not excluded
        if self.include_patterns.is_empty() {
            return true;
        }
        
        // Check include patterns
        for pattern in &self.include_patterns {
            if pattern.matches(&path_str) {
                debug!("Path {} included by pattern {}", path_str, pattern);
                return true;
            }
        }
        
        // If there are include patterns but none matched, exclude the file
        false
    }
    
    /// Get the include patterns
    pub fn include_patterns(&self) -> &[Pattern] {
        &self.include_patterns
    }
    
    /// Get the exclude patterns
    pub fn exclude_patterns(&self) -> &[Pattern] {
        &self.exclude_patterns
    }
    
    /// Get the minimum size
    pub fn min_size(&self) -> Option<u64> {
        self.min_size
    }
    
    /// Get the maximum size
    pub fn max_size(&self) -> Option<u64> {
        self.max_size
    }
    
    /// Convert to a string representation
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        
        // Include patterns
        for pattern in &self.include_patterns {
            result.push_str(&format!("{}\n", pattern));
        }
        
        // Exclude patterns
        for pattern in &self.exclude_patterns {
            result.push_str(&format!("!{}\n", pattern));
        }
        
        // Size constraints
        if let Some(min_size) = self.min_size {
            result.push_str(&format!("min_size: {}\n", min_size));
        }
        
        if let Some(max_size) = self.max_size {
            result.push_str(&format!("max_size: {}\n", max_size));
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_include_patterns() {
        let mut filter = FileFilter::new();
        filter.parse_patterns("*.txt\n*.md").unwrap();
        
        assert!(filter.should_include(&PathBuf::from("test.txt"), 100));
        assert!(filter.should_include(&PathBuf::from("test.md"), 100));
        assert!(!filter.should_include(&PathBuf::from("test.jpg"), 100));
    }
    
    #[test]
    fn test_exclude_patterns() {
        let mut filter = FileFilter::new();
        filter.parse_patterns("!*.tmp\n!*.bak").unwrap();
        
        assert!(filter.should_include(&PathBuf::from("test.txt"), 100));
        assert!(!filter.should_include(&PathBuf::from("test.tmp"), 100));
        assert!(!filter.should_include(&PathBuf::from("test.bak"), 100));
    }
    
    #[test]
    fn test_size_constraints() {
        let mut filter = FileFilter::new();
        filter.set_min_size(100);
        filter.set_max_size(1000);
        
        assert!(!filter.should_include(&PathBuf::from("small.txt"), 50));
        assert!(filter.should_include(&PathBuf::from("medium.txt"), 500));
        assert!(!filter.should_include(&PathBuf::from("large.txt"), 2000));
    }
    
    #[test]
    fn test_extensions() {
        let mut filter = FileFilter::new();
        filter.parse_extensions("txt,md,!tmp,!bak").unwrap();
        
        assert!(filter.should_include(&PathBuf::from("test.txt"), 100));
        assert!(filter.should_include(&PathBuf::from("test.md"), 100));
        assert!(!filter.should_include(&PathBuf::from("test.tmp"), 100));
        assert!(!filter.should_include(&PathBuf::from("test.bak"), 100));
        assert!(!filter.should_include(&PathBuf::from("test.jpg"), 100));
    }
    
    #[test]
    fn test_clear() {
        let mut filter = FileFilter::new();
        filter.parse_patterns("*.txt\n!*.tmp").unwrap();
        filter.set_min_size(100);
        
        assert!(filter.should_include(&PathBuf::from("test.txt"), 200));
        assert!(!filter.should_include(&PathBuf::from("test.tmp"), 200));
        assert!(!filter.should_include(&PathBuf::from("test.txt"), 50));
        
        filter.clear();
        
        assert!(filter.should_include(&PathBuf::from("test.txt"), 50));
        assert!(filter.should_include(&PathBuf::from("test.tmp"), 50));
    }
}
