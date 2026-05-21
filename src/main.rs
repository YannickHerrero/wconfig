#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod action;
mod app;
mod autostart;
mod config;
mod hotkey;
mod remap;
mod ui;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let cfg = config::Config::load().unwrap_or_else(|e| {
        eprintln!("wconfig: failed to load config ({e}); using defaults");
        config::Config::default()
    });

    let viewport = egui::ViewportBuilder::default()
        .with_title("wconfig")
        .with_inner_size([app::WINDOW_W, app::WINDOW_H])
        .with_min_inner_size([480.0, 360.0]);

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "wconfig",
        native_options,
        Box::new(move |_cc| Ok(Box::new(app::App::new(cfg)) as Box<dyn eframe::App>)),
    )
}
