# S3Sync — Next Session Plan

## Current State (2026-04-26)

- **Builds**: ✅ (17 dead code warnings, no errors)
- **Tests**: ✅ 8 passing, 1 ignored
- **Version**: 0.5.0
- **Total code**: ~5,300 lines across 34 .rs files
- **Git**: clean, all pushed to `git@github.com:mjkubba/s3-explorer.git`

### What exists and works:
- Project structure: aws/, config/, sync/, ui/ modules
- AWS auth via system keyring (`keyring` crate, service name `s3sync`)
- Credential storage/retrieval from Windows Credential Manager
- S3 bucket operations (list, create)
- File transfer (upload/download with multipart support)
- Sync engine with diff detection and file filtering
- GUI framework (eframe/egui) with multiple views
- Settings, bucket view, folder content, filter view, progress view

### What's broken/incomplete:
- **Old dependencies** — the biggest issue:
  - `eframe 0.17` → current is 0.29+ (major API changes)
  - `egui 0.17` → current is 0.29+
  - `aws-sdk-s3 0.28` → current is 1.x (major API changes)
  - `aws-config 0.55` → current is 1.x
  - `keyring 1.2` → current is 3.x
- `eframe::run_native` API has changed (old: takes Box<dyn App>, new: takes App creator closure)
- `NativeOptions` fields changed (initial_window_size → viewport)
- Scheduler is stubbed out
- 17 dead code warnings (error_handling.rs mostly)

## Plan for Next Session

### Phase 1: Update Dependencies (do first)

Update `Cargo.toml` to modern versions and fix all compile errors:

1. **eframe/egui 0.17 → 0.29+**
   - `eframe::run_native` signature changed
   - `NativeOptions` uses `ViewportBuilder` now
   - `egui::CtxRef` → `egui::Context`
   - `App` trait changed (update method signature)
   - This will touch: `main.rs`, all `ui/*.rs` files

2. **aws-sdk-s3 0.28 → 1.x, aws-config 0.55 → 1.x**
   - Import paths changed significantly
   - `Credentials` construction changed
   - `Client` builder API changed
   - This will touch: `aws/auth.rs`, `aws/bucket.rs`, `aws/transfer.rs`

3. **keyring 1.2 → 3.x**
   - `Entry::new` now returns `Result`
   - `set_password`/`get_password` API slightly changed
   - This will touch: `config/credentials.rs`

4. **Other deps**: update `env_logger`, `native-dialog`, `winres` to latest

### Phase 2: Fix and Verify

1. Fix all compile errors from dependency updates
2. Run tests — fix any broken tests
3. Clean up dead code warnings (remove or use the error_handling functions)
4. Verify the GUI launches and shows the main window

### Phase 3: End-to-End Test

1. Launch the app
2. Enter AWS credentials (should save to Windows Credential Manager)
3. List S3 buckets
4. Browse a bucket
5. Upload a test file
6. Download a test file

### Notes

- AWS creds are in Windows Credential Manager under service `s3sync` (keys: `aws_access_key`, `aws_secret_key`, `aws_region`)
- No AWS CLI installed on this machine
- Git remote uses SSH (`git@github.com:mjkubba/s3-explorer.git`) — use `git.exe` for push
- The `eframe` update is the biggest change — expect to rewrite most of `ui/app_impl.rs` and `main.rs`
- Keep the sync engine and filter logic as-is — they work fine
- Use `cargo.exe` for build/test (WSL on Windows filesystem)
