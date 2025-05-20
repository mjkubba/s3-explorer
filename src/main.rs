use eframe::egui;
use log::info;

mod aws;
mod config;
mod sync;
mod ui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    info!("Starting S3Sync application");

    // Application options
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2::new(1024.0, 768.0)),
        min_window_size: Some(egui::Vec2::new(800.0, 600.0)),
        centered: true,
        ..Default::default()
    };

    // Run the application
    eframe::run_native(
        "S3Sync",
        options,
        Box::new(|cc| Box::new(ui::app::S3SyncApp::new(cc))),
    )?;
    
    Ok(())
}
