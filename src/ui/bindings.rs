use eframe::egui;

use crate::app::App;

pub fn show(_app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Hotkey Bindings");
    ui.label("Bindings list editor — coming next.");
}
