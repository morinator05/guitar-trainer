#![cfg_attr(not(test), windows_subsystem = "windows")]

mod app;
mod audio;
mod screens;
mod settings;
mod srs;
mod storage;
mod theory;
mod util;
mod widgets;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Guitar Trainer")
            .with_app_id("guitar-trainer")
            .with_inner_size([1360.0, 900.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Guitar Trainer",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
