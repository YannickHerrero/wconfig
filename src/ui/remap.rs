use eframe::egui;

use crate::app::App;

pub fn show(_app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Key Remap");
    ui.label("Caps Lock remap — coming next.");
}
