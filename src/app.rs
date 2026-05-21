use eframe::egui;

use crate::config::Config;
use crate::ui::{bindings, general, remap, theme};

pub const WINDOW_W: f32 = 720.0;
pub const WINDOW_H: f32 = 520.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    General,
    Remap,
    Bindings,
}

pub struct App {
    pub cfg: Config,
    pub page: Page,
    theme_applied: bool,
}

impl App {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            page: Page::General,
            theme_applied: false,
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
        if !self.theme_applied {
            theme::apply(ui.ctx(), self.cfg.theme);
            self.theme_applied = true;
        }

        egui::SidePanel::left("nav")
            .resizable(false)
            .default_width(180.0)
            .show_inside(ui, |ui| {
                ui.add_space(12.0);
                ui.heading("wconfig");
                ui.add_space(16.0);
                nav_button(ui, &mut self.page, Page::General, "General");
                nav_button(ui, &mut self.page, Page::Remap, "Key Remap");
                nav_button(ui, &mut self.page, Page::Bindings, "Hotkey Bindings");
            });

        egui::CentralPanel::default().show_inside(ui, |ui| match self.page {
            Page::General => general::show(self, ui),
            Page::Remap => remap::show(self, ui),
            Page::Bindings => bindings::show(self, ui),
        });
    }
}

fn nav_button(ui: &mut egui::Ui, current: &mut Page, target: Page, label: &str) {
    let selected = *current == target;
    if ui.selectable_label(selected, label).clicked() {
        *current = target;
    }
}
