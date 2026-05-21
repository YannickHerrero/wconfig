use eframe::egui;

use crate::app::App;
use crate::config::{KeyName, RemapMode};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Key Remap");
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "Rewrites key events at the OS level via a low-level keyboard hook. The daemon \
             must be running. Changes take effect immediately on save.",
        )
        .small()
        .weak(),
    );
    ui.add_space(12.0);

    ui.label(egui::RichText::new("Caps Lock").strong());
    ui.add_space(4.0);
    ui.indent("caps_lock", |ui| {
        ui.horizontal(|ui| {
            ui.label("Mode");
            egui::ComboBox::from_id_salt("caps_mode")
                .selected_text(mode_label(app.cfg.remap.caps_lock.mode))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut app.cfg.remap.caps_lock.mode,
                        RemapMode::Off,
                        "Off (pass through)",
                    );
                    ui.selectable_value(
                        &mut app.cfg.remap.caps_lock.mode,
                        RemapMode::Single,
                        "Single — always remap to one key",
                    );
                    ui.selectable_value(
                        &mut app.cfg.remap.caps_lock.mode,
                        RemapMode::Dual,
                        "Dual — tap = X, hold = Y",
                    );
                });
        });

        ui.add_space(6.0);

        match app.cfg.remap.caps_lock.mode {
            RemapMode::Off => {
                ui.label(
                    egui::RichText::new("Caps Lock behaves normally.")
                        .small()
                        .weak(),
                );
            }
            RemapMode::Single => {
                key_row(ui, "Remap to", &mut app.cfg.remap.caps_lock.single_to);
            }
            RemapMode::Dual => {
                key_row(ui, "Tap sends", &mut app.cfg.remap.caps_lock.tap);
                key_row(ui, "Hold acts as", &mut app.cfg.remap.caps_lock.hold);
                ui.label(
                    egui::RichText::new(
                        "Held alone past the tap threshold (set on the General tab) or held \
                         with another key sends the hold key. Short taps send the tap key.",
                    )
                    .small()
                    .weak(),
                );
            }
        }
    });
}

fn key_row(ui: &mut egui::Ui, label: &str, key: &mut KeyName) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(
            egui::TextEdit::singleline(&mut key.0)
                .desired_width(180.0)
                .hint_text("e.g. escape, left_ctrl, f13"),
        );
    });
}

fn mode_label(m: RemapMode) -> &'static str {
    match m {
        RemapMode::Off => "Off",
        RemapMode::Single => "Single",
        RemapMode::Dual => "Dual",
    }
}
