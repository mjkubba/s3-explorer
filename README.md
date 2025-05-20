# S3Sync

A Windows GUI application for syncing local folders to AWS S3 buckets.

## Project Status

This project is currently in development. The initial code structure has been set up, but there are compatibility issues with the current Rust version (1.75.0) and some dependencies.

## Requirements

- Rust 1.82.0 or newer (current implementation requires newer Rust version than 1.75.0)
- Windows operating system (for the GUI application)

## Features

- Select one or more local folders for syncing
- Connect to AWS S3 using user credentials
- Upload new/modified files
- Delete files from S3 that were deleted locally (optional)
- Download files from S3 to local folders
- Scheduled syncs
- File filtering options

## Project Structure

```
s3sync/
├── src/
│   ├── main.rs           # Application entry point
│   ├── ui/               # UI components
│   ├── aws/              # AWS S3 operations
│   ├── sync/             # Sync logic
│   └── config/           # Configuration management
├── Cargo.toml            # Dependencies
├── build.rs              # Build script for Windows resources
└── README.md             # This file
```

## Next Steps

1. Upgrade Rust to version 1.82.0 or newer
2. Resolve dependency issues
3. Complete the implementation of the core functionality
4. Test on Windows systems
5. Add more advanced features

## License

This project is licensed under the MIT License - see the LICENSE file for details.
