use anyhow::{Context, Result};

#[cfg(windows)]
const KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const VALUE_NAME: &str = "wconfig";

#[cfg(windows)]
pub fn is_enabled() -> Result<bool> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(key) = hkcu.open_subkey(KEY_PATH) else {
        return Ok(false);
    };
    let value: std::io::Result<String> = key.get_value(VALUE_NAME);
    Ok(value.is_ok())
}

#[cfg(windows)]
pub fn enable() -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let exe = std::env::current_exe().context("locate current exe")?;
    let quoted = format!("\"{}\"", exe.display());

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(KEY_PATH).context("open HKCU Run key")?;
    key.set_value(VALUE_NAME, &quoted)
        .context("write autostart value")?;
    Ok(())
}

#[cfg(windows)]
pub fn disable() -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(key) = hkcu.open_subkey_with_flags(KEY_PATH, winreg::enums::KEY_WRITE) else {
        return Ok(());
    };
    let _ = key.delete_value(VALUE_NAME);
    Ok(())
}

#[cfg(windows)]
pub fn sync(desired: bool) -> Result<()> {
    if desired {
        enable()
    } else {
        disable()
    }
}

#[cfg(not(windows))]
pub fn is_enabled() -> Result<bool> {
    Ok(false)
}

#[cfg(not(windows))]
pub fn enable() -> Result<()> {
    anyhow::bail!("autostart is only implemented on Windows")
}

#[cfg(not(windows))]
pub fn disable() -> Result<()> {
    Ok(())
}

#[cfg(not(windows))]
pub fn sync(_desired: bool) -> Result<()> {
    Ok(())
}
