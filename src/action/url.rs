use anyhow::Result;

#[cfg(windows)]
pub fn open(url: &str) -> Result<()> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::PCWSTR;

    let op: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    let url_w: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();

    let result = unsafe {
        ShellExecuteW(
            Some(HWND(std::ptr::null_mut())),
            PCWSTR(op.as_ptr()),
            PCWSTR(url_w.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        anyhow::bail!("ShellExecuteW failed for url (code {})", result.0 as isize);
    }
    tracing::debug!("opened url: {url}");
    Ok(())
}

#[cfg(not(windows))]
pub fn open(_url: &str) -> Result<()> {
    anyhow::bail!("url::open is only implemented on Windows")
}
