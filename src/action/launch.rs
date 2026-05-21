use anyhow::{Context, Result};

#[cfg(windows)]
pub fn run(command: &str) -> Result<()> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    // `cmd /c start "" <cmd>` routes through ShellExecute, which uses the
    // App Paths registry (so bare names like `firefox` resolve) and handles
    // .lnk activation. raw_arg is required because cmd.exe's parser doesn't
    // understand the MS C runtime backslash-quote escapes that Rust uses by
    // default.
    let line = format!("start \"\" {command}");
    std::process::Command::new("cmd")
        .arg("/c")
        .raw_arg(&line)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .with_context(|| format!("spawn `cmd /c {line}`"))?;
    tracing::debug!("launched: {command}");
    Ok(())
}

#[cfg(windows)]
pub fn run_path_with_args(exe_path: &str, args: &[String]) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;

    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::PCWSTR;

    let op: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    let path_w: Vec<u16> = std::ffi::OsStr::new(exe_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let params_string = args.join(" ");
    let params_w: Vec<u16> = if params_string.is_empty() {
        Vec::new()
    } else {
        params_string
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect()
    };
    let params_ptr = if params_w.is_empty() {
        PCWSTR::null()
    } else {
        PCWSTR(params_w.as_ptr())
    };

    let result = unsafe {
        ShellExecuteW(
            Some(HWND(std::ptr::null_mut())),
            PCWSTR(op.as_ptr()),
            PCWSTR(path_w.as_ptr()),
            params_ptr,
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        anyhow::bail!("ShellExecuteW failed (code {})", result.0 as isize);
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn run(_command: &str) -> Result<()> {
    anyhow::bail!("launch is only implemented on Windows")
}

#[cfg(not(windows))]
pub fn run_path_with_args(_exe_path: &str, _args: &[String]) -> Result<()> {
    anyhow::bail!("launch is only implemented on Windows")
}
