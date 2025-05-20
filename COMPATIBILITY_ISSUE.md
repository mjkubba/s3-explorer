# Compatibility Issue

## Current Problem

The project is currently facing compatibility issues with Rust 1.75.0. Several dependencies require Rust 1.82.0 or newer:

- `icu_normalizer_data v2.0.0`
- `icu_properties_data v2.0.1`
- `icu_collections v2.0.0`
- `tinystr v0.8.1`
- `idna_adapter v1.2.1`

These dependencies are transitive dependencies of the egui/eframe GUI framework and the AWS SDK.

## Solution Options

1. **Upgrade Rust**: Install Rust 1.82.0 or newer using rustup:
   ```
   rustup update stable
   ```
   or
   ```
   rustup install 1.82.0
   rustup default 1.82.0
   ```

2. **Use an alternative GUI framework**: Consider using a different GUI framework that's compatible with Rust 1.75.0, such as:
   - [iced](https://github.com/iced-rs/iced)
   - [druid](https://github.com/linebender/druid)
   - [gtk-rs](https://gtk-rs.org/)

3. **Use older versions of dependencies**: Try to pin specific versions of transitive dependencies that are compatible with Rust 1.75.0 using cargo's `[patch]` section.

## Recommended Approach

The recommended approach is to upgrade Rust to version 1.82.0 or newer, as this will provide the most straightforward path forward and ensure compatibility with the latest versions of dependencies.

If upgrading Rust is not possible, consider switching to the iced GUI framework, which has better compatibility with older Rust versions.
