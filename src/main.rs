use log::info;

mod aws;
mod config;
mod sync;
mod ui;
#[allow(dead_code)]
mod error_handling;

fn main() -> eframe::Result {
    env_logger::init();
    info!("Starting S3Sync application");

    // Create a tokio runtime on a background thread so async tasks
    // can run while eframe owns the main thread event loop.
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let _guard = rt.enter();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "S3 Sync",
        native_options,
        Box::new(|_cc| Ok(Box::new(ui::app::S3SyncApp::default()))),
    )
}
