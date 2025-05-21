use std::path::Path;
use std::collections::HashSet;
use glob::Pattern;
use log::debug;

/// Filter for determining which files to include/exclude in sync operations
#[derive(Clone, Debug)]
pub struct FileFilter {
    /// Patterns to include (if empty, all files are included by default)
    include_patterns: Vec<Pattern>,
    
    /// Patterns to exclude
    exclude_patterns: Vec<Pattern>,
    
    /// File extensions to include (if empty, all extensions are included by default)
    include_extensions: HashSet<String>,
    
    /// File extensions to exclude
    exclude_extensions: HashSet<String>,
    
    /// Minimum file size in bytes (None = no minimum)
    min_size: Option<u64>,
    
    /// Maximum file size in bytes (None = no maximum)
    max_size: Option<u64>,
}

impl Default for FileFilter {
    fn default() -> Self {
        Self {
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            include_extensions: HashSet::new(),
            exclude_extensions: HashSet::new(),
            min_size: None,
            max_size: None,
        }
    }
}

impl FileFilter {
    /// Create a new file filter
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a pattern to include
    pub fn add_include_pattern(&mut self, pattern: &str) -> Result<(), glob::PatternError> {
        let compiled = Pattern::new(pattern)?;
        self.include_patterns.push(compiled);
        Ok(())
    }
    
    /// Add a pattern to exclude
    pub fn add_exclude_pattern(&mut self, pattern: &str) -> Result<(), glob::PatternError> {
        let compiled = Pattern::new(pattern)?;
        self.exclude_patterns.push(compiled);
        Ok(())
    }
    
    /// Add an extension to include
    pub fn add_include_extension(&mut self, extension: &str) {
        let ext = extension.trim_start_matches('.');
        self.include_extensions.insert(ext.to_lowercase());
    }
    
    /// Add an extension to exclude
    pub fn add_exclude_extension(&mut self, extension: &str) {
        let ext = extension.trim_start_matches('.');
        self.exclude_extensions.insert(ext.to_lowercase());
    }
    
    /// Set minimum file size
    pub fn set_min_size(&mut self, size: u64) {
        self.min_size = Some(size);
    }
    
    /// Set maximum file size
    pub fn set_max_size(&mut self, size: u64) {
        self.max_size = Some(size);
    }
    
    /// Clear all filters
    pub fn clear(&mut self) {
        self.include_patterns.clear();
        self.exclude_patterns.clear();
        self.include_extensions.clear();
        self.exclude_extensions.clear();
        self.min_size = None;
        self.max_size = None;
    }
    
    /// Check if a file should be included based on the filter
    pub fn should_include(&self, path: &Path, size: u64) -> bool {
        // Check file size constraints
        if let Some(min_size) = self.min_size {
            if size < min_size {
                debug!("Excluding {} due to size < {}", path.display(), min_size);
                return false;
            }
        }
        
        if let Some(max_size) = self.max_size {
            if size > max_size {
                debug!("Excluding {} due to size > {}", path.display(), max_size);
                return false;
            }
        }
        
        // Get file extension
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        
        // Check extension exclusions
        if !self.exclude_extensions.is_empty() && self.exclude_extensions.contains(&extension) {
            debug!("Excluding {} due to extension {}", path.display(), extension);
            return false;
        }
        
        // Check extension inclusions
        if !self.include_extensions.is_empty() && !self.include_extensions.contains(&extension) && !extension.is_empty() {
            debug!("Excluding {} due to extension not in include list", path.display());
            return false;
        }
        
        // Convert path to string for pattern matching
        let path_str = path.to_string_lossy();
        
        // Check exclude patterns
        for pattern in &self.exclude_patterns {
            if pattern.matches(&path_str) {
                debug!("Excluding {} due to pattern {}", path.display(), pattern);
                return false;
            }
        }
        
        // Check include patterns
        if !self.include_patterns.is_empty() {
            let mut included = false;
            for pattern in &self.include_patterns {
                if pattern.matches(&path_str) {
                    included = true;
                    break;
                }
            }
            
            if !included {
                debug!("Excluding {} due to not matching any include pattern", path.display());
                return false;
            }
        }
        
        // If we get here, the file should be included
        true
    }
    
    /// Parse filter patterns from a string
    pub fn parse_patterns(&mut self, patterns: &str) -> Result<(), glob::PatternError> {
        for line in patterns.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if line.starts_with('!') {
                // Exclude pattern
                self.add_exclude_pattern(&line[1..])?;
            } else {
                // Include pattern
                self.add_include_pattern(line)?;
            }
        }
        
        Ok(())
    }
    
    /// Parse extension filters from a string
    pub fn parse_extensions(&mut self, extensions: &str) -> Result<(), ()> {
        for ext in extensions.split(',') {
            let ext = ext.trim();
            if ext.is_empty() {
                continue;
            }
            
            if ext.starts_with('!') {
                // Exclude extension
                self.add_exclude_extension(&ext[1..]);
            } else {
                // Include extension
                self.add_include_extension(ext);
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_extension_filtering() {
        let mut filter = FileFilter::new();
        
        // Include only .txt and .md files
        filter.add_include_extension("txt");
        filter.add_include_extension(".md");
        
        assert!(filter.should_include(&PathBuf::from("test.txt"), 100));
        assert!(filter.should_include(&PathBuf::from("test.md"), 100));
        assert!(!filter.should_include(&PathBuf::from("test.jpg"), 100));
        assert!(!filter.should_include(&PathBuf::from("test.docx"), 100));
    }
    
    #[test]
    fn test_exclude_extensions() {
        let mut filter = FileFilter::new();
        
        // Exclude .tmp and .bak files
        filter.add_exclude_extension("tmp");
        filter.add_exclude_extension(".bak");
        
        assert!(filter.should_include(&PathBuf::from("test.txt"), 100));
        assert!(filter.should_include(&PathBuf::from("test.md"), 100));
        assert!(!filter.should_include(&PathBuf::from("test.tmp"), 100));
        assert!(!filter.should_include(&PathBuf::from("test.bak"), 100));
    }
    
    #[test]
    fn test_size_filtering() {
        let mut filter = FileFilter::new();
        
        // Only include files between 100 and 1000 bytes
        filter.set_min_size(100);
        filter.set_max_size(1000);
        
        assert!(filter.should_include(&PathBuf::from("test.txt"), 500));
        assert!(!filter.should_include(&PathBuf::from("test.txt"), 50));
        assert!(!filter.should_include(&PathBuf::from("test.txt"), 1500));
    }
    
    #[test]
    fn test_pattern_filtering() {
        let mut filter = FileFilter::new();
        
        // Include only files in the docs directory
        filter.add_include_pattern("docs/**/*").unwrap();
        
        assert!(filter.should_include(&PathBuf::from("docs/test.txt"), 100));
        assert!(filter.should_include(&PathBuf::from("docs/subdir/test.md"), 100));
        assert!(!filter.should_include(&PathBuf::from("src/test.txt"), 100));
    }
    
    #[test]
    fn test_exclude_patterns() {
        let mut filter = FileFilter::new();
        
        // Exclude all files in the temp directory
        filter.add_exclude_pattern("temp/**/*").unwrap();
        
        assert!(filter.should_include(&PathBuf::from("docs/test.txt"), 100));
        assert!(!filter.should_include(&PathBuf::from("temp/test.txt"), 100));
        assert!(!filter.should_include(&PathBuf::from("temp/subdir/test.md"), 100));
    }
    
    #[test]
    fn test_combined_filtering() {
        let mut filter = FileFilter::new();
        
        // Include only .txt files in the docs directory
        filter.add_include_pattern("docs/**/*").unwrap();
        filter.add_include_extension("txt");
        
        assert!(filter.should_include(&PathBuf::from("docs/test.txt"), 100));
        assert!(!filter.should_include(&PathBuf::from("docs/test.md"), 100));
        assert!(!filter.should_include(&PathBuf::from("src/test.txt"), 100));
    }
    
    #[test]
    fn test_parse_patterns() {
        let mut filter = FileFilter::new();
        
        let patterns = r#"
        # Include patterns
        docs/**/*
        src/**/*.rs
        
        # Exclude patterns
        !temp/**/*
        !**/*.tmp
        "#;
        
        filter.parse_patterns(patterns).unwrap();
        
        assert!(filter.should_include(&PathBuf::from("docs/test.txt"), 100));
        assert!(filter.should_include(&PathBuf::from("src/main.rs"), 100));
        assert!(!filter.should_include(&PathBuf::from("temp/test.txt"), 100));
        assert!(!filter.should_include(&PathBuf::from("docs/test.tmp"), 100));
    }
}
