use eframe::egui;

use crate::app::App;
use crate::config::Theme;
use crate::ui::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("General");
    ui.add_space(8.0);

    section(ui, "Appearance", |ui| {
        ui.horizontal(|ui| {
            ui.label("Theme");
            let prev = app.cfg.theme;
            egui::ComboBox::from_id_salt("theme")
                .selected_text(theme_label(app.cfg.theme))
                .show_ui(ui, |ui| {
                    for t in [
                        Theme::Paper,
                        Theme::Stone,
                        Theme::Sage,
                        Theme::Clay,
                        Theme::Ink,
                    ] {
                        ui.selectable_value(&mut app.cfg.theme, t, theme_label(t));
                    }
                });
            if app.cfg.theme != prev {
                theme::apply(ui.ctx(), app.cfg.theme);
            }
        });
    });

    ui.add_space(12.0);

    section(ui, "Startup", |ui| {
        ui.checkbox(&mut app.cfg.daemon.autostart, "Start with Windows");
        ui.checkbox(
            &mut app.cfg.daemon.start_minimized,
            "Start minimized to tray",
        );
    });

    ui.add_space(12.0);

    section(ui, "About", |ui| {
        ui.label(
            egui::RichText::new(format!("wconfig v{}", env!("CARGO_PKG_VERSION")))
                .small()
                .weak(),
        );
    });
}

fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    ui.label(egui::RichText::new(title).strong());
    ui.add_space(4.0);
    ui.indent(title, body);
}

fn theme_label(t: Theme) -> &'static str {
    match t {
        Theme::Paper => "Paper",
        Theme::Stone => "Stone",
        Theme::Sage => "Sage",
        Theme::Clay => "Clay",
        Theme::Ink => "Ink",
    }
}
