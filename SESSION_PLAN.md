# S3Sync — Session Plan

## Current State (2026-04-27)

- **Builds**: ✅ (16 dead code warnings, 0 deprecation warnings, no errors)
- **Tests**: ✅ 8 passing, 1 ignored, 0 failed
- **Version**: 0.5.0
- **Rust**: 1.95.0 (updated from 1.87.0)
- **Git**: clean, committed locally (not pushed)

### Dependencies Updated (Phase 1 ✅ COMPLETE):
- `eframe` 0.17 → 0.34.1
- `egui` 0.17 → 0.34.1
- `aws-sdk-s3` 0.28 → 1.x (1.123.0)
- `aws-config` 0.55 → 1.x (1.8.14)
- `aws-types` 0.55 → 1.x (1.3.12)
- `keyring` 1.2 → 3.6
- `env_logger` 0.9 → 0.11
- `native-dialog` 0.6 → 0.7
- `mockall` 0.11 → 0.13
- Added `aws-credential-types` 1.2

### API Changes Applied:
- `epi::App` → `eframe::App` with `fn ui()` instead of `fn update()`
- `NativeOptions::initial_window_size` → `viewport: ViewportBuilder`
- `eframe::run_native` now takes creator closure returning `Result`
- `egui::Layout::right_to_left()` → `right_to_left(Align::Center)`
- `from_id_source` → `from_id_salt`
- `Frame::none()` → `Frame::NONE`
- `TopBottomPanel` → `Panel::top/bottom`
- `menu::bar()` → `MenuBar::new().ui()`
- `close_menu()` → `close()`
- `clamp_to_range` → removed (default behavior)
- AWS SDK: `buckets()/contents()` return `&[T]` directly (no unwrap_or_default)
- AWS SDK: `size()` returns `Option<i64>`, `content_length()` returns `Option<i64>`
- AWS SDK: `is_truncated()` returns `Option<bool>`
- AWS SDK: `Credentials` moved to `aws-credential-types`
- AWS SDK: `RegionProviderChain` → `aws_config::defaults()` with `BehaviorVersion`
- Keyring: `Entry::new()` returns `Result`, `delete_password()` → `delete_credential()`

## Plan for Next Session

### Phase 2: Fix and Verify (NEXT)

1. ~~Fix all compile errors from dependency updates~~ ✅
2. ~~Run tests — fix any broken tests~~ ✅
3. Clean up dead code warnings (remove or `#[allow]` the error_handling functions)
4. **Verify the GUI launches and shows the main window**

### Phase 3: End-to-End Test

1. Launch the app
2. Enter AWS credentials (should save to Windows Credential Manager)
3. List S3 buckets
4. Browse a bucket
5. Upload a test file
6. Download a test file

### Notes

- AWS creds are in Windows Credential Manager under service `s3sync`
- Git remote uses SSH (`git@github.com:mjkubba/s3-explorer.git`) — use `git.exe` for push
- Use `cargo.exe` for build/test (WSL on Windows filesystem)
- The `app_progress_view.rs` file is dead code (not in mod.rs) — can be deleted
- 16 dead code warnings remain, mostly in `error_handling.rs` and unused UI methods
