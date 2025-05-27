# S3Sync: A Cross-Platform GUI Tool for AWS S3 File Synchronization

S3Sync is a modern, user-friendly desktop application that provides seamless file synchronization between local directories and AWS S3 buckets. Built with Rust and eframe/egui, it offers a responsive graphical interface for managing your S3 storage with features like real-time progress tracking, file filtering, and secure credential management.

The application simplifies common S3 operations by providing an intuitive interface for uploading, downloading, and synchronizing files. It supports advanced features such as file filtering based on patterns and sizes, bandwidth limiting, and secure credential storage using the system keyring. With its multi-threaded architecture and asynchronous operations, S3Sync ensures efficient file transfers while maintaining a responsive user interface.

## Repository Structure
```
.
├── src/                          # Source code directory
│   ├── aws/                      # AWS integration modules
│   │   ├── auth.rs              # AWS authentication handling
│   │   ├── bucket.rs            # S3 bucket operations
│   │   ├── s3.rs                # S3 object representation
│   │   └── transfer.rs          # File transfer operations
│   ├── config/                   # Configuration management
│   │   ├── credentials.rs       # AWS credential management
│   │   └── settings.rs          # Application settings
│   ├── sync/                    # Synchronization logic
│   │   ├── diff.rs              # File difference detection
│   │   ├── engine.rs            # Core sync engine
│   │   └── filter.rs            # File filtering logic
│   ├── ui/                      # User interface components
│   │   ├── app.rs              # Main application window
│   │   ├── bucket_view.rs      # S3 bucket viewer
│   │   └── folder_content.rs   # Local folder viewer
├── build.rs                     # Build configuration
└── Cargo.toml                   # Project dependencies and metadata
```

## Usage Instructions
### Prerequisites
- Rust toolchain (1.56.0 or later)
- System dependencies:
  - Linux: libxcb libraries for GUI rendering
  - Windows: None additional
  - macOS: None additional
- AWS account with access credentials
- Stable internet connection

### Installation

#### From Source
```bash
# Clone the repository
git clone https://github.com/yourusername/s3sync.git
cd s3sync

# Build the application
cargo build --release

# Run the application
cargo run --release
```

#### Platform-Specific Instructions

**Linux (Debian/Ubuntu)**
```bash
# Install required dependencies
sudo apt-get update
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev

# Build and install
cargo install --path .
```

**macOS**
```bash
# Using Homebrew (recommended)
brew install rust
cargo install --path .
```

**Windows**
```powershell
# No additional dependencies required
cargo install --path .
```

### Quick Start
1. Launch the application
2. Go to Settings and enter your AWS credentials
3. Click "Connect to AWS" to verify your credentials
4. Select or add local folders for synchronization
5. Choose an S3 bucket from the bucket view
6. Click "Sync" to start synchronization

### More Detailed Examples

**Setting Up File Filters**
```rust
// Configure file filters in the UI
1. Click "View" -> "Filters"
2. Add include patterns (e.g., "*.jpg", "*.png")
3. Set exclude patterns (e.g., "*.tmp", "*.log")
4. Configure size limits if needed
5. Click "Apply" to save filters
```

**Synchronizing a Local Folder**
```rust
1. Select a local folder from the folder list
2. Choose a destination S3 bucket
3. Configure sync options:
   - Enable/disable deletion sync
   - Set bandwidth limits
   - Apply filters
4. Click "Sync" to start the process
```

### Troubleshooting

**Common Issues**

1. AWS Authentication Failures
```
Error: InvalidHeaderValue
Solution: 
- Verify AWS credentials in Settings
- Ensure region is correctly set
- Check for special characters in access keys
```

2. File Transfer Issues
```
Error: Connection timeout
Solutions:
- Check internet connection
- Verify firewall settings
- Reduce concurrent transfer limit
```

**Debug Mode**
```bash
# Enable debug logging
RUST_LOG=debug cargo run

# Log file locations:
- Linux/macOS: ~/.local/share/s3sync/logs/
- Windows: %APPDATA%\s3sync\logs\
```

## Data Flow
S3Sync processes file synchronization through a multi-stage pipeline that handles authentication, file comparison, and transfer operations asynchronously.

```ascii
[Local Filesystem] --> [File Scanner] --> [Diff Engine] --> [Transfer Manager] --> [S3 Bucket]
       ^                     |                |                     |                  |
       |                     v                v                     v                  |
       +----------------[File Filter]-->[Sync Engine]-->[Progress Tracker]------------+
```

Component Interactions:
1. File Scanner reads local directory structure and metadata
2. Diff Engine compares local and remote file states
3. File Filter applies user-defined inclusion/exclusion rules
4. Sync Engine coordinates the overall synchronization process
5. Transfer Manager handles file uploads/downloads with AWS S3
6. Progress Tracker monitors and reports operation status
7. UI components update in real-time based on operation progress