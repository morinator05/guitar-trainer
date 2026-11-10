#![cfg_attr(not(test), windows_subsystem = "windows")]

mod app;
mod audio;
mod screens;
mod settings;
mod srs;
mod storage;
mod theme;
mod theory;
mod util;
mod widgets;

fn main() -> eframe::Result {
    // Developer option: window size, e.g. GT_SIZE=960x640
    let size = std::env::var("GT_SIZE")
        .ok()
        .and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some([w.parse().ok()?, h.parse().ok()?])
        })
        .unwrap_or([1360.0, 900.0]);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Guitar Trainer")
            .with_app_id("guitar-trainer")
            .with_inner_size(size)
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Guitar Trainer",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
