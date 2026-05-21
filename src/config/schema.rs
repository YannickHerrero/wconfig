use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Paper,
    Stone,
    Sage,
    Clay,
    Ink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DaemonCfg {
    pub autostart: bool,
    pub start_minimized: bool,
}

impl Default for DaemonCfg {
    fn default() -> Self {
        Self {
            autostart: false,
            start_minimized: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellKind {
    #[default]
    Powershell,
    Cmd,
    Pwsh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Launch {
        command: String,
    },
    Url {
        url: String,
    },
    Script {
        #[serde(default)]
        shell: ShellKind,
        script: String,
    },
    FocusOrLaunch {
        exe_path: String,
        #[serde(default = "default_true")]
        match_basename: bool,
        #[serde(default)]
        launch_args: Vec<String>,
    },
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    pub label: String,
    pub key: String,
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub theme: Theme,
    pub daemon: DaemonCfg,
    pub bindings: Vec<Binding>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            theme: Theme::default(),
            daemon: DaemonCfg::default(),
            bindings: default_bindings(),
        }
    }
}

fn default_bindings() -> Vec<Binding> {
    vec![
        Binding {
            label: "Open terminal".into(),
            key: "Alt+Enter".into(),
            action: Action::Launch {
                command: "\"C:/Program Files/WezTerm/wezterm-gui.exe\"".into(),
            },
        },
        Binding {
            label: "Focus or launch Firefox".into(),
            key: "Alt+B".into(),
            action: Action::FocusOrLaunch {
                exe_path: "C:/Program Files/Mozilla Firefox/firefox.exe".into(),
                match_basename: true,
                launch_args: Vec::new(),
            },
        },
    ]
}
