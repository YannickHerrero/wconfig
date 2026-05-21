#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod action;
mod app;
mod autostart;
mod config;
mod hotkey;
mod ipc;
mod remap;
mod single_instance;
mod tray;
mod ui;

use std::sync::mpsc::channel;

use eframe::egui;

use crate::single_instance::Acquire;

fn main() -> eframe::Result<()> {
    init_logging();

    match single_instance::acquire() {
        Ok(Acquire::First) => {}
        Ok(Acquire::AlreadyRunning) => {
            if let Err(e) = single_instance::signal_show_gui() {
                eprintln!("wconfig: failed to signal running instance: {e}");
            }
            return Ok(());
        }
        Err(e) => {
            eprintln!("wconfig: singleton check failed: {e}");
            return Ok(());
        }
    }

    let cfg = config::Config::load().unwrap_or_else(|e| {
        eprintln!("wconfig: failed to load config ({e}); using defaults");
        config::Config::default()
    });

    if let Err(e) = remap::install(cfg.remap.caps_lock.clone(), cfg.daemon.tap_timeout_ms) {
        eprintln!("wconfig: install keyboard hook: {e}");
    }

    let hotkey = match hotkey::Manager::new() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("wconfig: build hotkey manager: {e}");
            return Ok(());
        }
    };

    let tray = match tray::build() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("wconfig: build tray icon: {e}");
            return Ok(());
        }
    };

    let (show_tx, show_rx) = channel();
    if let Err(e) = ipc::start_listener(show_tx) {
        eprintln!("wconfig: start show-gui listener: {e}");
    }

    let viewport = egui::ViewportBuilder::default()
        .with_title("wconfig")
        .with_inner_size([app::WINDOW_W, app::WINDOW_H])
        .with_min_inner_size([520.0, 380.0])
        .with_visible(!cfg.daemon.start_minimized);

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "wconfig",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(app::App::new(
                cfg,
                hotkey,
                tray,
                &cc.egui_ctx,
                show_rx,
            )) as Box<dyn eframe::App>)
        }),
    )
}

fn init_logging() {
    use tracing_subscriber::filter::EnvFilter;
    use tracing_subscriber::fmt;

    let filter = EnvFilter::try_from_env("WCONFIG_LOG")
        .unwrap_or_else(|_| EnvFilter::new("wconfig=info"));
    let _ = fmt().with_env_filter(filter).try_init();
}
