use eframe::egui;

use crate::app::App;
use crate::config::{Action, Binding, ShellKind};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Hotkey Bindings");
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "Each binding pairs a key combination with an action. Examples: Alt+Enter, Ctrl+Alt+G.",
        )
        .small()
        .weak(),
    );
    ui.add_space(8.0);

    let mut remove: Option<usize> = None;

    let errors = app.binding_errors.clone();
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (idx, binding) in app.cfg.bindings.iter_mut().enumerate() {
            ui.push_id(idx, |ui| {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Label");
                        ui.add(egui::TextEdit::singleline(&mut binding.label).desired_width(180.0));
                        ui.label("Key");
                        ui.add(egui::TextEdit::singleline(&mut binding.key).desired_width(160.0));
                        if ui.button("Remove").clicked() {
                            remove = Some(idx);
                        }
                    });
                    ui.add_space(4.0);
                    action_editor(ui, &mut binding.action);
                    for err in errors.iter().filter(|e| e.index == idx) {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(format!("⚠ {}", err.message))
                                .small()
                                .color(egui::Color32::from_rgb(0xC0, 0x39, 0x2B)),
                        );
                    }
                });
            });
            ui.add_space(6.0);
        }
    });

    if let Some(idx) = remove {
        app.cfg.bindings.remove(idx);
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("+ Add binding").clicked() {
            app.cfg.bindings.push(Binding {
                label: String::from("New binding"),
                key: String::from("Ctrl+Alt+N"),
                action: Action::Launch {
                    command: String::new(),
                },
            });
        }
    });
}

fn action_editor(ui: &mut egui::Ui, action: &mut Action) {
    ui.horizontal(|ui| {
        ui.label("Action");
        let label = action_type_label(action);
        egui::ComboBox::from_id_salt("action_type")
            .selected_text(label)
            .show_ui(ui, |ui| {
                if ui.selectable_label(matches!(action, Action::Launch { .. }), "Launch").clicked() {
                    *action = Action::Launch { command: take_command(action) };
                }
                if ui.selectable_label(matches!(action, Action::Url { .. }), "Open URL").clicked() {
                    *action = Action::Url { url: String::new() };
                }
                if ui.selectable_label(matches!(action, Action::Script { .. }), "Run script").clicked() {
                    *action = Action::Script {
                        shell: ShellKind::Powershell,
                        script: String::new(),
                    };
                }
                if ui.selectable_label(matches!(action, Action::FocusOrLaunch { .. }), "Focus or launch").clicked() {
                    *action = Action::FocusOrLaunch {
                        exe_path: String::new(),
                        match_basename: true,
                        launch_args: Vec::new(),
                    };
                }
            });
    });

    match action {
        Action::Launch { command } => {
            ui.horizontal(|ui| {
                ui.label("Command");
                ui.add(egui::TextEdit::singleline(command).desired_width(f32::INFINITY));
            });
        }
        Action::Url { url } => {
            ui.horizontal(|ui| {
                ui.label("URL");
                ui.add(egui::TextEdit::singleline(url).desired_width(f32::INFINITY));
            });
        }
        Action::Script { shell, script } => {
            ui.horizontal(|ui| {
                ui.label("Shell");
                egui::ComboBox::from_id_salt("shell_kind")
                    .selected_text(shell_label(*shell))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(shell, ShellKind::Powershell, "PowerShell");
                        ui.selectable_value(shell, ShellKind::Cmd, "cmd");
                        ui.selectable_value(shell, ShellKind::Pwsh, "pwsh (PowerShell 7+)");
                    });
            });
            ui.label("Script");
            ui.add(
                egui::TextEdit::multiline(script)
                    .desired_width(f32::INFINITY)
                    .desired_rows(4)
                    .code_editor(),
            );
        }
        Action::FocusOrLaunch {
            exe_path,
            match_basename,
            launch_args,
        } => {
            ui.horizontal(|ui| {
                ui.label("Exe path");
                ui.add(egui::TextEdit::singleline(exe_path).desired_width(f32::INFINITY));
            });
            ui.checkbox(match_basename, "Match by basename (firefox.exe) instead of full path");
            ui.horizontal(|ui| {
                ui.label("Launch args (one per line)");
            });
            let mut joined = launch_args.join("\n");
            let resp = ui.add(
                egui::TextEdit::multiline(&mut joined)
                    .desired_width(f32::INFINITY)
                    .desired_rows(2),
            );
            if resp.changed() {
                *launch_args = joined
                    .lines()
                    .map(|l| l.to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
            }
        }
    }
}

fn action_type_label(action: &Action) -> &'static str {
    match action {
        Action::Launch { .. } => "Launch",
        Action::Url { .. } => "Open URL",
        Action::Script { .. } => "Run script",
        Action::FocusOrLaunch { .. } => "Focus or launch",
    }
}

fn shell_label(s: ShellKind) -> &'static str {
    match s {
        ShellKind::Powershell => "PowerShell",
        ShellKind::Cmd => "cmd",
        ShellKind::Pwsh => "pwsh",
    }
}

fn take_command(action: &Action) -> String {
    match action {
        Action::Launch { command } => command.clone(),
        Action::Url { url } => url.clone(),
        Action::Script { script, .. } => script.clone(),
        Action::FocusOrLaunch { exe_path, .. } => exe_path.clone(),
    }
}
