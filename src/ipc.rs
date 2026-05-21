//! Named-event "show GUI" IPC: when a second wconfig.exe is launched it
//! signals the running daemon by setting this event, rather than starting
//! a duplicate process. A background thread waits on the event and pushes
//! a request through a channel so the GUI thread can show the window.

use std::sync::mpsc::Sender;
use std::thread;

use anyhow::Result;

use crate::single_instance::SHOW_GUI_EVENT_NAME;

#[derive(Debug, Clone, Copy)]
pub struct ShowGui;

#[cfg(windows)]
pub fn start_listener(sender: Sender<ShowGui>) -> Result<()> {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
    use windows::core::PCWSTR;

    struct SendHandle(HANDLE);
    unsafe impl Send for SendHandle {}

    let name: Vec<u16> = SHOW_GUI_EVENT_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // Auto-reset event, initially unsignaled.
    let event = unsafe { CreateEventW(None, false, false, PCWSTR(name.as_ptr())) }?;
    let event = SendHandle(event);

    thread::spawn(move || {
        let event = event;
        loop {
            let res = unsafe { WaitForSingleObject(event.0, u32::MAX) };
            if res.0 != 0 {
                tracing::warn!("show-gui wait failed: {:?}", res);
                break;
            }
            if sender.send(ShowGui).is_err() {
                break;
            }
        }
        unsafe {
            let _ = CloseHandle(event.0);
        }
    });
    Ok(())
}

#[cfg(not(windows))]
pub fn start_listener(_sender: Sender<ShowGui>) -> Result<()> {
    Ok(())
}
