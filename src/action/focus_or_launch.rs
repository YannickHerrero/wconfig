use anyhow::Result;

use crate::action::launch;

#[cfg(windows)]
pub fn run(exe_path: &str, match_basename: bool, launch_args: &[String]) -> Result<()> {
    let target = normalize_target(exe_path, match_basename);
    if target.is_empty() {
        anyhow::bail!("focus_or_launch: exe_path is empty");
    }

    let candidates = enumerate_top_windows();
    let mut matched: Option<windows::Win32::Foundation::HWND> = None;
    for (hwnd, exe) in &candidates {
        if exe_matches(exe, &target, match_basename) {
            matched = Some(*hwnd);
            break;
        }
    }

    if let Some(hwnd) = matched {
        let fg = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
        if fg == hwnd {
            tracing::debug!("focus_or_launch: already focused {:?}", target);
            return Ok(());
        }
        focus_window(hwnd)?;
        tracing::debug!("focus_or_launch: focused existing window for {:?}", target);
        return Ok(());
    }

    launch::run_path_with_args(exe_path, launch_args)
}

#[cfg(windows)]
fn normalize_target(exe_path: &str, match_basename: bool) -> String {
    let lowered = exe_path.to_lowercase();
    if match_basename {
        std::path::Path::new(&lowered)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(lowered)
    } else {
        lowered.replace('\\', "/")
    }
}

#[cfg(windows)]
fn exe_matches(exe: &str, target: &str, match_basename: bool) -> bool {
    let lowered = exe.to_lowercase();
    if match_basename {
        std::path::Path::new(&lowered)
            .file_name()
            .map(|s| s.to_string_lossy() == target)
            .unwrap_or(false)
    } else {
        lowered.replace('\\', "/") == *target
    }
}

#[cfg(windows)]
fn enumerate_top_windows() -> Vec<(windows::Win32::Foundation::HWND, String)> {
    use std::cell::RefCell;

    use windows::Win32::Foundation::{HWND, LPARAM, TRUE};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GWL_EXSTYLE, GetWindowLongPtrW, IsWindowVisible, WS_EX_TOOLWINDOW,
    };
    use windows::core::BOOL;

    thread_local! {
        static RESULTS: RefCell<Vec<(HWND, String)>> = const { RefCell::new(Vec::new()) };
    }

    unsafe extern "system" fn cb(hwnd: HWND, _: LPARAM) -> BOOL {
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() {
                return TRUE;
            }
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            if (ex_style as u32) & WS_EX_TOOLWINDOW.0 != 0 {
                return TRUE;
            }
            if let Some(exe) = exe_for_window(hwnd) {
                RESULTS.with(|r| r.borrow_mut().push((hwnd, exe)));
            }
        }
        TRUE
    }

    RESULTS.with(|r| r.borrow_mut().clear());
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(0));
    }
    RESULTS.with(|r| r.borrow().clone())
}

#[cfg(windows)]
unsafe fn exe_for_window(hwnd: windows::Win32::Foundation::HWND) -> Option<String> {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let handle: HANDLE = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(h) => h,
            Err(_) => return None,
        };
        let mut buf = vec![0u16; 1024];
        let mut size = buf.len() as u32;
        let res = QueryFullProcessImageNameW(handle, PROCESS_NAME_FORMAT(0), windows::core::PWSTR(buf.as_mut_ptr()), &mut size);
        let _ = CloseHandle(handle);
        if res.is_err() {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..size as usize]))
    }
}

#[cfg(windows)]
fn focus_window(hwnd: windows::Win32::Foundation::HWND) -> Result<()> {
    use windows::Win32::UI::WindowsAndMessaging::{
        IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow,
    };

    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn run(_exe_path: &str, _match_basename: bool, _launch_args: &[String]) -> Result<()> {
    anyhow::bail!("focus_or_launch is only implemented on Windows")
}
