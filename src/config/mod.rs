pub mod schema;

pub use schema::*;

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;

pub fn project_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "wconfig")
        .context("could not determine application data directory")?;
    Ok(dirs.config_dir().to_path_buf())
}

pub fn config_path() -> Result<PathBuf> {
    Ok(project_dir()?.join("config.toml"))
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("read config: {}", path.display()))?;
        let cfg =
            toml::from_str(&text).with_context(|| format!("parse config: {}", path.display()))?;
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create config dir: {}", parent.display()))?;
        }
        let text = toml::to_string_pretty(self).context("serialize config")?;
        fs::write(&path, text).with_context(|| format!("write config: {}", path.display()))?;
        Ok(())
    }
}
