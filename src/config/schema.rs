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
    pub tap_timeout_ms: u64,
}

impl Default for DaemonCfg {
    fn default() -> Self {
        Self {
            autostart: false,
            start_minimized: true,
            tap_timeout_ms: 180,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemapMode {
    #[default]
    Off,
    Single,
    Dual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyName(pub String);

impl KeyName {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Default for KeyName {
    fn default() -> Self {
        Self(String::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CapsLockRemap {
    pub mode: RemapMode,
    pub single_to: KeyName,
    pub tap: KeyName,
    pub hold: KeyName,
}

impl Default for CapsLockRemap {
    fn default() -> Self {
        Self {
            mode: RemapMode::Off,
            single_to: KeyName::new("left_ctrl"),
            tap: KeyName::new("escape"),
            hold: KeyName::new("left_ctrl"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RemapCfg {
    pub caps_lock: CapsLockRemap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub theme: Theme,
    pub daemon: DaemonCfg,
    pub remap: RemapCfg,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            theme: Theme::default(),
            daemon: DaemonCfg::default(),
            remap: RemapCfg::default(),
        }
    }
}
