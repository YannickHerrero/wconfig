//! Low-level keyboard hook (WH_KEYBOARD_LL) that drives the tap-vs-hold FSM.
//!
//! The hook runs on the GUI thread's message loop. The callback is small:
//! decide whether the event is a Caps event vs an other-key event, ask the
//! FSM what to do, then apply each `Effect` (suppress, pass through, inject,
//! schedule/cancel timeout).

use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, SendInput,
    VIRTUAL_KEY, VK_CAPITAL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HHOOK, KBDLLHOOKSTRUCT, LLKHF_INJECTED, SetWindowsHookExW, UnhookWindowsHookEx,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::state::{Effect, HookEvent, Machine};
use super::timer::TimerHandle;
use crate::config::CapsLockRemap;

/// Wrapper that lets us stash the HHOOK in a static. The OS hook handle is
/// only ever touched from a single thread (the one that called SetWindowsHookEx),
/// but the type isn't auto-Send/Sync so we mark it explicitly.
struct HookSlot(HHOOK);
unsafe impl Send for HookSlot {}
unsafe impl Sync for HookSlot {}

/// Marker stored in `KBDLLHOOKSTRUCT.dwExtraInfo` (and SendInput's dwExtraInfo)
/// so the hook recognises and skips its own synthetic events. "WCNF" = 0x57434E46.
pub const WCONFIG_MARKER: usize = 0x57434E46;

struct HookState {
    machine: Machine,
    cfg: CapsLockRemap,
    tap_timeout_ms: u64,
    timer: Option<TimerHandle>,
}

static STATE: OnceLock<Mutex<HookState>> = OnceLock::new();
static HOOK_HANDLE: OnceLock<Mutex<Option<HookSlot>>> = OnceLock::new();

pub fn install(initial_cfg: CapsLockRemap, tap_timeout_ms: u64) -> Result<()> {
    STATE
        .set(Mutex::new(HookState {
            machine: Machine::new(),
            cfg: initial_cfg,
            tap_timeout_ms,
            timer: None,
        }))
        .map_err(|_| anyhow::anyhow!("hook state already initialised"))?;

    let hh = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_proc), Some(HINSTANCE::default()), 0)
            .context("SetWindowsHookExW(WH_KEYBOARD_LL)")?
    };
    HOOK_HANDLE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .replace(HookSlot(hh));
    tracing::info!("keyboard hook installed");
    Ok(())
}

pub fn uninstall() -> Result<()> {
    if let Some(slot) = HOOK_HANDLE.get() {
        if let Some(HookSlot(hh)) = slot.lock().unwrap().take() {
            unsafe {
                UnhookWindowsHookEx(hh).context("UnhookWindowsHookEx")?;
            }
            tracing::info!("keyboard hook uninstalled");
        }
    }
    Ok(())
}

/// Hot-update the active remap config without reinstalling the hook.
pub fn reconfigure(new_cfg: CapsLockRemap, tap_timeout_ms: u64) {
    if let Some(state) = STATE.get() {
        let mut s = state.lock().unwrap();
        s.cfg = new_cfg;
        s.tap_timeout_ms = tap_timeout_ms;
    }
}

/// Re-enter the FSM with a Timeout event. Called by the timer thread.
pub(super) fn fire_timeout(timestamp_ms: u64) {
    let Some(state) = STATE.get() else {
        return;
    };
    let mut s = state.lock().unwrap();
    let cfg = s.cfg.clone();
    let effects = s.machine.handle(HookEvent::Timeout { timestamp_ms }, &cfg);
    drop(s);
    apply_effects(&effects, &cfg, /* origin_vk = */ None);
}

unsafe extern "system" fn low_level_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code != 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    let info: &KBDLLHOOKSTRUCT = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };

    // Skip our own injected events, and any other process's injected events
    // for safety — only operate on physical key presses.
    if info.dwExtraInfo == WCONFIG_MARKER || (info.flags.0 & LLKHF_INJECTED.0) != 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let msg = wparam.0 as u32;
    let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
    if !is_down && !is_up {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let vk = info.vkCode;
    let now = info.time as u64;
    let is_caps = vk == VK_CAPITAL.0 as u32;

    let event = match (is_caps, is_down) {
        (true, true) => HookEvent::CapsDown { timestamp_ms: now },
        (true, false) => HookEvent::CapsUp { timestamp_ms: now },
        (false, true) => HookEvent::KeyDown {
            vk,
            timestamp_ms: now,
        },
        (false, false) => HookEvent::KeyUp {
            vk,
            timestamp_ms: now,
        },
    };

    let Some(state) = STATE.get() else {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    };

    let mut s = state.lock().unwrap();
    let cfg = s.cfg.clone();
    let effects = s.machine.handle(event, &cfg);
    drop(s);

    let mut suppress = false;
    let mut pass = false;
    for eff in &effects {
        match eff {
            Effect::Suppress => suppress = true,
            Effect::PassThrough => pass = true,
            Effect::Inject { vk, down } => send_key(*vk, *down),
            Effect::ScheduleTimeout { .. } => schedule_timeout(now),
            Effect::CancelTimeout => cancel_timeout(),
        }
    }

    if suppress {
        return LRESULT(1);
    }
    if pass || effects.is_empty() {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    LRESULT(0)
}

fn apply_effects(effects: &[Effect], _cfg: &CapsLockRemap, _origin_vk: Option<u32>) {
    for eff in effects {
        match eff {
            Effect::Inject { vk, down } => send_key(*vk, *down),
            Effect::ScheduleTimeout { .. } => {
                schedule_timeout(now_ms());
            }
            Effect::CancelTimeout => cancel_timeout(),
            // Suppress/PassThrough are meaningless outside the hook callback.
            _ => {}
        }
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn schedule_timeout(now_ms: u64) {
    let Some(state) = STATE.get() else {
        return;
    };
    let mut s = state.lock().unwrap();
    let delay = s.tap_timeout_ms;
    let handle = s.timer.take();
    drop(s);

    if let Some(h) = handle {
        h.cancel();
    }
    let new_handle = TimerHandle::start(delay, move || {
        super::hook::fire_timeout(now_ms + delay);
    });
    let mut s = STATE.get().unwrap().lock().unwrap();
    s.timer = Some(new_handle);
}

fn cancel_timeout() {
    if let Some(state) = STATE.get() {
        if let Some(handle) = state.lock().unwrap().timer.take() {
            handle.cancel();
        }
    }
}

fn send_key(vk: u32, down: bool) {
    let flags: KEYBD_EVENT_FLAGS = if down {
        KEYBD_EVENT_FLAGS(0)
    } else {
        KEYEVENTF_KEYUP
    };
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk as u16),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: WCONFIG_MARKER,
            },
        },
    };
    let inputs = [input];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}
