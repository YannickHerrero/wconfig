use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use crate::config::ShellKind;

#[cfg(windows)]
pub fn run(shell: ShellKind, script: &str) -> Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let (path, args, body) = prepare(shell, script)?;

    fs::write(&path, body)
        .with_context(|| format!("write script tempfile: {}", path.display()))?;

    let mut cmd = std::process::Command::new(args[0]);
    for a in &args[1..] {
        cmd.arg(a);
    }
    cmd.arg(&path);
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd.spawn()
        .with_context(|| format!("spawn shell for script: {}", path.display()))?;

    tracing::debug!("ran script via {:?}: {}", shell, path.display());
    Ok(())
}

fn prepare(shell: ShellKind, script: &str) -> Result<(std::path::PathBuf, Vec<&'static str>, String)> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let (ext, args, body): (&str, Vec<&'static str>, String) = match shell {
        ShellKind::Powershell => (
            "ps1",
            vec!["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File"],
            script.to_string(),
        ),
        ShellKind::Pwsh => (
            "ps1",
            vec!["pwsh", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File"],
            script.to_string(),
        ),
        ShellKind::Cmd => (
            "cmd",
            vec!["cmd", "/c"],
            // cmd .bat needs CRLF line endings to behave well across versions.
            script.replace('\n', "\r\n"),
        ),
    };
    let mut path = std::env::temp_dir();
    path.push(format!("wconfig-{stamp}.{ext}"));
    Ok((path, args, body))
}

#[cfg(not(windows))]
pub fn run(_shell: ShellKind, _script: &str) -> Result<()> {
    anyhow::bail!("script::run is only implemented on Windows")
}
