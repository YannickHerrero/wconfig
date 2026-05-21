use eframe::egui;

use crate::app::App;

pub fn show(_app: &mut App, ui: &mut egui::Ui) {
    ui.heading("General");
    ui.label("Theme, autostart, daemon options — coming next.");
}
