//! Low-level keyboard hook (WH_KEYBOARD_LL) that drives the tap-vs-hold FSM.
//!
//! The hook is installed on the thread that calls `install()` — the caller
//! must already be a message-pumping thread (i.e. eframe's main thread).
//! Worker threads created via std::thread::spawn don't reliably receive
//! WH_KEYBOARD_LL callbacks on Windows: empirically, the OS-side dispatch
//! often no-ops if the installing thread has no window context.
//!
//! Because the callback runs on the GUI thread, every cycle must stay well
//! under Windows' `LowLevelHooksTimeout` (default 300ms) — if exceeded the
//! OS silently drops this event from the chain, which can break other apps'
//! LL hooks (e.g. glazewm). To keep the hot path microsecond-fast we read
//! the active config via `ArcSwap` (lock-free) and the FSM state via an
//! atomic, only acquiring the `Mutex<Machine>` when we actually need to
//! mutate FSM state — i.e. on caps events.

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use std::sync::Arc;
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, SendInput,
    VIRTUAL_KEY, VK_CAPITAL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HHOOK, KBDLLHOOKSTRUCT, LLKHF_INJECTED, SetWindowsHookExW,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};
use windows::core::PCWSTR;

use super::state::{Effect, HookEvent, Machine, StateKind};
use super::timer::TimerHandle;
use crate::config::{CapsLockRemap, RemapMode};

/// Marker stored in `KBDLLHOOKSTRUCT.dwExtraInfo` (and SendInput's dwExtraInfo)
/// so the hook recognises and skips its own synthetic events. "WCNF" = 0x57434E46.
pub const WCONFIG_MARKER: usize = 0x57434E46;

// FSM state kept here as an atomic so the hook callback can read it without
// locking. Mutations go through the Mutex<Machine> below.
const STATE_IDLE: u8 = 0;
const STATE_PENDING: u8 = 1;
const STATE_HOLD: u8 = 2;
static STATE_TAG: AtomicU8 = AtomicU8::new(STATE_IDLE);

struct HookSlot(HHOOK);
unsafe impl Send for HookSlot {}
unsafe impl Sync for HookSlot {}

struct HookState {
    machine: Machine,
    tap_timeout_ms: u64,
    timer: Option<TimerHandle>,
}

static STATE: OnceLock<Mutex<HookState>> = OnceLock::new();
static CFG: OnceLock<ArcSwap<CapsLockRemap>> = OnceLock::new();
static HOOK_HANDLE: OnceLock<Mutex<Option<HookSlot>>> = OnceLock::new();

static CALLBACK_FIRES: AtomicU64 = AtomicU64::new(0);
static CAPS_EVENTS: AtomicU64 = AtomicU64::new(0);

/// Read the running totals so callers (e.g. the app's periodic heartbeat
/// logger) can confirm whether the OS is actually dispatching events.
pub fn counters() -> (u64, u64) {
    (
        CALLBACK_FIRES.load(Ordering::Relaxed),
        CAPS_EVENTS.load(Ordering::Relaxed),
    )
}

fn ensure_state(cfg: CapsLockRemap, tap_timeout_ms: u64) {
    STATE.get_or_init(|| {
        Mutex::new(HookState {
            machine: Machine::new(),
            tap_timeout_ms,
            timer: None,
        })
    });
    CFG.get_or_init(|| ArcSwap::new(Arc::new(cfg.clone())));
}

/// Install (idempotent) the WH_KEYBOARD_LL hook on the *calling thread*.
/// The caller must be the thread running the application's message pump
/// (eframe's main thread).
pub fn install(cfg: CapsLockRemap, tap_timeout_ms: u64) -> Result<()> {
    ensure_state(cfg.clone(), tap_timeout_ms);
    reconfigure(cfg, tap_timeout_ms);

    let slot = HOOK_HANDLE.get_or_init(|| Mutex::new(None));
    let mut slot = slot.lock().unwrap();
    if slot.is_some() {
        return Ok(());
    }

    let hmod = unsafe {
        GetModuleHandleW(PCWSTR::null()).context("GetModuleHandleW(NULL)")?
    };
    let hh = unsafe {
        SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(low_level_proc),
            Some(HINSTANCE(hmod.0)),
            0,
        )
        .context("SetWindowsHookExW(WH_KEYBOARD_LL)")?
    };
    *slot = Some(HookSlot(hh));
    tracing::info!("keyboard hook installed on calling thread");
    Ok(())
}

/// Uninstall the OS hook if installed. Safe to call when no hook is active.
pub fn uninstall() -> Result<()> {
    let Some(slot) = HOOK_HANDLE.get() else {
        return Ok(());
    };
    let mut slot = slot.lock().unwrap();
    if let Some(HookSlot(hh)) = slot.take() {
        unsafe {
            UnhookWindowsHookEx(hh).context("UnhookWindowsHookEx")?;
        }
        tracing::info!("keyboard hook uninstalled");
    }
    Ok(())
}

/// Hot-update the active remap config without disturbing the hook.
pub fn reconfigure(new_cfg: CapsLockRemap, tap_timeout_ms: u64) {
    if let Some(cfg) = CFG.get() {
        cfg.store(Arc::new(new_cfg));
    }
    if let Some(state) = STATE.get() {
        state.lock().unwrap().tap_timeout_ms = tap_timeout_ms;
    }
}

/// Re-enter the FSM with a Timeout event. Called by the timer thread.
pub(super) fn fire_timeout(timestamp_ms: u64) {
    let (Some(state), Some(cfg)) = (STATE.get(), CFG.get()) else {
        return;
    };
    let cfg = cfg.load_full();
    if cfg.mode != RemapMode::Dual {
        return;
    }
    let mut s = state.lock().unwrap();
    let effects = s.machine.handle(HookEvent::Timeout { timestamp_ms }, &cfg);
    refresh_state_tag(&s.machine);
    drop(s);
    apply_effects(&effects);
}

fn refresh_state_tag(machine: &Machine) {
    let tag = match machine.state_kind() {
        StateKind::Idle => STATE_IDLE,
        StateKind::Pending => STATE_PENDING,
        StateKind::HoldActive => STATE_HOLD,
    };
    STATE_TAG.store(tag, Ordering::Release);
}

unsafe extern "system" fn low_level_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        low_level_proc_inner(code, wparam, lparam)
    }));
    match result {
        Ok(r) => r,
        Err(_) => {
            tracing::error!("hook callback panicked, passing event through");
            unsafe { CallNextHookEx(None, code, wparam, lparam) }
        }
    }
}

unsafe fn low_level_proc_inner(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code != 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    let info: &KBDLLHOOKSTRUCT = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };

    // Diagnostic: log the very first few callback fires so the log makes it
    // clear that the OS is actually dispatching to us.
    let n = CALLBACK_FIRES.fetch_add(1, Ordering::Relaxed);
    if n < 5 {
        tracing::info!(
            "hook fire #{n}: wparam={:#x}, vk={:#x}, flags={:#x}",
            wparam.0,
            info.vkCode,
            info.flags.0
        );
    }

    // Skip our own injected events and other injected events.
    let injected = info.dwExtraInfo == WCONFIG_MARKER || (info.flags.0 & LLKHF_INJECTED.0) != 0;
    if injected {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let msg = wparam.0 as u32;
    let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
    if !is_down && !is_up {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let vk = info.vkCode;
    let is_caps = vk == VK_CAPITAL.0 as u32;

    // Ultra-fast path. Reading both atomics is a few nanoseconds. For the
    // overwhelming majority of keystrokes (non-caps, FSM idle, mode != Dual),
    // we fall through to CallNextHookEx without touching the mutex at all.
    let state_tag = STATE_TAG.load(Ordering::Acquire);
    if !is_caps && state_tag == STATE_IDLE {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    // Mode check via lock-free ArcSwap.
    let Some(cfg) = CFG.get() else {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    };
    let cfg = cfg.load_full();

    // Off mode: pure passthrough.
    if cfg.mode == RemapMode::Off {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let now = info.time as u64;
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
    let effects = s.machine.handle(event, &cfg);
    refresh_state_tag(&s.machine);
    drop(s);

    if is_caps {
        let n = CAPS_EVENTS.fetch_add(1, Ordering::Relaxed);
        tracing::info!(
            "caps {} #{n}, effects={:?}",
            if is_down { "down" } else { "up" },
            effects
        );
    }

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

fn apply_effects(effects: &[Effect]) {
    for eff in effects {
        match eff {
            Effect::Inject { vk, down } => send_key(*vk, *down),
            Effect::ScheduleTimeout { .. } => {
                schedule_timeout(now_ms());
            }
            Effect::CancelTimeout => cancel_timeout(),
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
