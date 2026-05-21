use std::sync::mpsc::{Receiver, channel};

use eframe::egui;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use notify::RecommendedWatcher;
use tray_icon::menu::MenuEvent;

use crate::action;
use crate::autostart;
use crate::config::{Config, RemapMode, watcher as config_watcher};
use crate::hotkey::{BindingError, Manager as HotkeyMgr};
use crate::ipc::ShowGui;
use crate::remap;
use crate::tray::Tray;
use crate::ui::{bindings, general, remap as remap_page, theme};

pub const WINDOW_W: f32 = 760.0;
pub const WINDOW_H: f32 = 560.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    General,
    Remap,
    Bindings,
}

pub struct App {
    pub cfg: Config,
    pub page: Page,
    pub binding_errors: Vec<BindingError>,

    pub hotkey: HotkeyMgr,
    pub tray: Tray,
    pub _watcher: RecommendedWatcher,

    pub hotkey_rx: Receiver<GlobalHotKeyEvent>,
    pub menu_rx: Receiver<MenuEvent>,
    pub cfg_rx: Receiver<Config>,
    pub show_rx: Receiver<ShowGui>,

    pub visible: bool,
    theme_applied: bool,
    dirty: bool,
    last_status: Option<String>,
    quitting: bool,
}

impl App {
    pub fn new(
        cfg: Config,
        mut hotkey: HotkeyMgr,
        tray: Tray,
        ctx: &egui::Context,
        show_rx: Receiver<ShowGui>,
    ) -> Self {
        let (hk_tx, hk_rx) = channel();
        let ctx_hk = ctx.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |ev| {
            let _ = hk_tx.send(ev);
            ctx_hk.request_repaint();
        }));

        let (menu_tx, menu_rx) = channel();
        let ctx_menu = ctx.clone();
        MenuEvent::set_event_handler(Some(move |ev| {
            let _ = menu_tx.send(ev);
            ctx_menu.request_repaint();
        }));

        let (cfg_tx, cfg_rx) = channel();
        let watcher = config_watcher::spawn(cfg_tx).expect("spawn config watcher");

        let binding_errors = hotkey.set_bindings(&cfg.bindings);

        let initial_visible = !cfg.daemon.start_minimized;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(initial_visible));

        Self {
            cfg,
            page: Page::General,
            binding_errors,
            hotkey,
            tray,
            _watcher: watcher,
            hotkey_rx: hk_rx,
            menu_rx,
            cfg_rx,
            show_rx,
            visible: initial_visible,
            theme_applied: false,
            dirty: false,
            last_status: None,
            quitting: false,
        }
    }

    fn show_window(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.visible = true;
    }

    fn hide_window(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        self.visible = false;
    }

    fn save_and_apply(&mut self, ctx: &egui::Context) {
        match self.cfg.save() {
            Ok(()) => {
                self.dirty = false;
                self.last_status = Some(format!(
                    "Saved to {}",
                    crate::config::config_path()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| "(unknown)".into())
                ));
            }
            Err(e) => {
                self.last_status = Some(format!("Save failed: {e}"));
                tracing::warn!("save config: {e}");
                return;
            }
        }
        self.apply_runtime(ctx);
    }

    fn apply_runtime(&mut self, ctx: &egui::Context) {
        theme::apply(ctx, self.cfg.theme);
        self.binding_errors = self.hotkey.set_bindings(&self.cfg.bindings);
        // The OS hook is only installed when there's a real remap to apply.
        // This avoids interfering with other apps' global hotkeys (e.g. window
        // managers) via the WH_KEYBOARD_LL chain when wconfig has nothing to do.
        if self.cfg.remap.caps_lock.mode == RemapMode::Off {
            if let Err(e) = remap::uninstall() {
                tracing::warn!("uninstall keyboard hook: {e:#}");
            }
        } else if let Err(e) = remap::install(
            self.cfg.remap.caps_lock.clone(),
            self.cfg.daemon.tap_timeout_ms,
        ) {
            tracing::warn!("install keyboard hook: {e:#}");
        }
        if let Err(e) = autostart::sync(self.cfg.daemon.autostart) {
            tracing::warn!("sync autostart: {e}");
        }
    }

    fn poll_tray(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.menu_rx.try_recv() {
            if ev.id == self.tray.settings_id {
                self.show_window(ctx);
            } else if ev.id == self.tray.reload_id {
                match Config::load() {
                    Ok(cfg) => {
                        self.cfg = cfg;
                        self.apply_runtime(ctx);
                        self.last_status = Some("Reloaded from disk".into());
                    }
                    Err(e) => {
                        self.last_status = Some(format!("Reload failed: {e}"));
                    }
                }
            } else if ev.id == self.tray.quit_id {
                tracing::info!("quit requested from tray");
                self.quitting = true;
                let _ = remap::uninstall();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn poll_hotkey(&mut self) {
        while let Ok(ev) = self.hotkey_rx.try_recv() {
            if ev.state() != HotKeyState::Pressed {
                continue;
            }
            let Some(idx) = self.hotkey.binding_index_for(ev.id()) else {
                continue;
            };
            let Some(binding) = self.cfg.bindings.get(idx) else {
                continue;
            };
            tracing::info!("hotkey fired: '{}' -> {:?}", binding.label, binding.action);
            if let Err(e) = action::run(&binding.action) {
                tracing::warn!("action failed: {e}");
            }
        }
    }

    fn poll_cfg(&mut self, ctx: &egui::Context) {
        while let Ok(cfg) = self.cfg_rx.try_recv() {
            self.cfg = cfg;
            self.dirty = false;
            self.apply_runtime(ctx);
            self.last_status = Some("Hot-reloaded from disk".into());
        }
    }

    fn poll_show(&mut self, ctx: &egui::Context) {
        if self.show_rx.try_recv().is_ok() {
            // Drain any extras.
            while self.show_rx.try_recv().is_ok() {}
            self.show_window(ctx);
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::palette(self.cfg.theme)
            .paper
            .to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.theme_applied {
            theme::apply(&ctx, self.cfg.theme);
            self.theme_applied = true;
        }

        self.poll_tray(&ctx);
        self.poll_hotkey();
        self.poll_cfg(&ctx);
        self.poll_show(&ctx);

        // Intercept close: hide to tray instead of exiting — unless the user
        // explicitly chose Quit from the tray menu.
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.quitting {
                tracing::info!("close honoured: quitting");
                return;
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide_window(&ctx);
            return;
        }

        let prev_cfg_hash = config_signature(&self.cfg);

        egui::Panel::left("nav")
            .resizable(false)
            .default_size(180.0)
            .show_inside(ui, |ui| {
                ui.add_space(12.0);
                ui.heading("wconfig");
                ui.add_space(16.0);
                nav_button(ui, &mut self.page, Page::General, "General");
                nav_button(ui, &mut self.page, Page::Remap, "Key Remap");
                nav_button(ui, &mut self.page, Page::Bindings, "Hotkey Bindings");
            });

        egui::Panel::bottom("status").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    self.save_and_apply(&ctx);
                }
                if self.dirty {
                    ui.label(
                        egui::RichText::new("• unsaved changes")
                            .weak()
                            .small(),
                    );
                }
                if let Some(s) = &self.last_status {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(s).small().weak());
                    });
                }
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| match self.page {
            Page::General => general::show(self, ui),
            Page::Remap => remap_page::show(self, ui),
            Page::Bindings => bindings::show(self, ui),
        });

        if config_signature(&self.cfg) != prev_cfg_hash {
            self.dirty = true;
        }
    }
}

fn nav_button(ui: &mut egui::Ui, current: &mut Page, target: Page, label: &str) {
    let selected = *current == target;
    if ui.selectable_label(selected, label).clicked() {
        *current = target;
    }
}

fn config_signature(cfg: &Config) -> Option<String> {
    toml::to_string(cfg).ok()
}
