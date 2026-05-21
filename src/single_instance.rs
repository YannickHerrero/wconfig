use anyhow::Result;

#[cfg(windows)]
const MUTEX_NAME: &str = "Local\\wconfig-singleton-mutex";
#[cfg(windows)]
pub const SHOW_GUI_EVENT_NAME: &str = "Local\\wconfig-show-gui";

/// Outcome of trying to acquire the singleton.
pub enum Acquire {
    /// This is the first running instance — proceed normally.
    First,
    /// Another instance is already running. Caller should pulse the show-GUI
    /// event and exit.
    AlreadyRunning,
}

#[cfg(windows)]
pub fn acquire() -> Result<Acquire> {
    use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::PCWSTR;

    let name: Vec<u16> = MUTEX_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let _ = unsafe { CreateMutexW(None, true, PCWSTR(name.as_ptr())) }?;
    let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    Ok(if already {
        Acquire::AlreadyRunning
    } else {
        Acquire::First
    })
}

/// Signal the running daemon to open the GUI window.
#[cfg(windows)]
pub fn signal_show_gui() -> Result<()> {
    use windows::Win32::System::Threading::{OpenEventW, SetEvent, SYNCHRONIZATION_ACCESS_RIGHTS};
    use windows::Win32::Foundation::CloseHandle;
    use windows::core::PCWSTR;

    let name: Vec<u16> = SHOW_GUI_EVENT_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    const EVENT_MODIFY_STATE: SYNCHRONIZATION_ACCESS_RIGHTS = SYNCHRONIZATION_ACCESS_RIGHTS(0x0002);
    let handle = unsafe { OpenEventW(EVENT_MODIFY_STATE, false, PCWSTR(name.as_ptr())) }?;
    unsafe {
        let _ = SetEvent(handle);
        let _ = CloseHandle(handle);
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn acquire() -> Result<Acquire> {
    Ok(Acquire::First)
}

#[cfg(not(windows))]
pub fn signal_show_gui() -> Result<()> {
    Ok(())
}
