#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod action;
mod app;
mod autostart;
mod config;
mod hotkey;
mod ipc;
mod logging;
mod remap;
mod single_instance;
mod tray;
mod ui;

use std::sync::mpsc::channel;

use eframe::egui;

use crate::single_instance::Acquire;

fn main() -> eframe::Result<()> {
    let _log_guard = match logging::init() {
        Ok(g) => Some(g),
        Err(e) => {
            eprintln!("wconfig: init logging: {e}");
            None
        }
    };
    tracing::info!("wconfig starting (exe: {:?})", std::env::current_exe().ok());

    match single_instance::acquire() {
        Ok(Acquire::First) => tracing::info!("single-instance: acquired mutex"),
        Ok(Acquire::AlreadyRunning) => {
            tracing::info!("single-instance: another instance already running, signalling and exiting");
            if let Err(e) = single_instance::signal_show_gui() {
                tracing::warn!("signal show-gui: {e}");
            }
            return Ok(());
        }
        Err(e) => {
            tracing::error!("single-instance check failed: {e:#}");
            return Ok(());
        }
    }

    let cfg = match config::Config::load() {
        Ok(c) => {
            tracing::info!("config loaded ({} bindings)", c.bindings.len());
            c
        }
        Err(e) => {
            tracing::warn!("load config: {e:#}; using defaults");
            config::Config::default()
        }
    };

    let hotkey = match hotkey::Manager::new() {
        Ok(m) => {
            tracing::info!("hotkey manager built");
            m
        }
        Err(e) => {
            tracing::error!("build hotkey manager: {e:#}");
            return Ok(());
        }
    };

    let tray = match tray::build() {
        Ok(t) => {
            tracing::info!("tray icon built");
            t
        }
        Err(e) => {
            tracing::error!("build tray icon: {e:#}");
            return Ok(());
        }
    };

    let (show_tx, show_rx) = channel();
    if let Err(e) = ipc::start_listener(show_tx) {
        tracing::warn!("start show-gui listener: {e:#}");
    } else {
        tracing::info!("show-gui listener started");
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

    tracing::info!("entering eframe::run_native");
    let res = eframe::run_native(
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
    );
    tracing::info!("eframe::run_native returned: {:?}", res.as_ref().map(|_| "Ok"));
    res
}
