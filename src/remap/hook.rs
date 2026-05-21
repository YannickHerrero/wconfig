//! Low-level keyboard hook (WH_KEYBOARD_LL) that drives the tap-vs-hold FSM.
//!
//! The hook is installed on a dedicated thread whose only job is to pump
//! Win32 messages. This matters: Windows enforces a per-callback timeout
//! (LowLevelHooksTimeout, default 300ms) and if exceeded the OS silently
//! breaks the hook chain for that event, which can drop other apps' LL
//! hooks (e.g. glazewm's Alt+1/2/3 workspace switching). Running on the
//! eframe UI thread risks that timeout whenever the GUI is doing real
//! work; a dedicated thread sidesteps the problem entirely. This is the
//! same approach kanata's `--win-llhook` backend and PowerToys use.

use std::sync::{Mutex, OnceLock};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, SendInput,
    VIRTUAL_KEY, VK_CAPITAL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, HHOOK, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG,
    PostThreadMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::state::{Effect, HookEvent, Machine};
use super::timer::TimerHandle;
use crate::config::CapsLockRemap;

/// Marker stored in `KBDLLHOOKSTRUCT.dwExtraInfo` (and SendInput's dwExtraInfo)
/// so the hook recognises and skips its own synthetic events. "WCNF" = 0x57434E46.
pub const WCONFIG_MARKER: usize = 0x57434E46;

struct HookState {
    machine: Machine,
    cfg: CapsLockRemap,
    tap_timeout_ms: u64,
    timer: Option<TimerHandle>,
}

struct HookThread {
    thread_id: u32,
    join: Option<JoinHandle<()>>,
}

static STATE: OnceLock<Mutex<HookState>> = OnceLock::new();
static HOOK_THREAD: OnceLock<Mutex<Option<HookThread>>> = OnceLock::new();

fn ensure_state(cfg: CapsLockRemap, tap_timeout_ms: u64) -> &'static Mutex<HookState> {
    STATE.get_or_init(|| {
        Mutex::new(HookState {
            machine: Machine::new(),
            cfg,
            tap_timeout_ms,
            timer: None,
        })
    })
}

/// Install (idempotent) the WH_KEYBOARD_LL hook on a dedicated thread.
/// Subsequent calls update the active CapsLockRemap / tap_timeout via the
/// shared STATE without re-spawning the thread.
pub fn install(cfg: CapsLockRemap, tap_timeout_ms: u64) -> Result<()> {
    ensure_state(cfg.clone(), tap_timeout_ms);
    reconfigure(cfg, tap_timeout_ms);

    let slot = HOOK_THREAD.get_or_init(|| Mutex::new(None));
    let mut slot = slot.lock().unwrap();
    if slot.is_some() {
        return Ok(());
    }

    let (tid_tx, tid_rx) = std::sync::mpsc::channel::<Result<u32>>();
    let join = thread::Builder::new()
        .name("wconfig-keyboard-hook".into())
        .spawn(move || hook_thread_main(tid_tx))
        .context("spawn hook thread")?;

    let tid = tid_rx
        .recv()
        .context("hook thread failed to report its id")?
        .context("hook thread init")?;
    *slot = Some(HookThread {
        thread_id: tid,
        join: Some(join),
    });
    tracing::info!("keyboard hook installed on dedicated thread tid={tid}");
    Ok(())
}

/// Signal the hook thread to exit. Safe to call when no hook is active.
pub fn uninstall() -> Result<()> {
    let Some(slot) = HOOK_THREAD.get() else {
        return Ok(());
    };
    let mut slot = slot.lock().unwrap();
    let Some(mut thread) = slot.take() else {
        return Ok(());
    };

    unsafe {
        PostThreadMessageW(thread.thread_id, WM_QUIT, WPARAM(0), LPARAM(0))
            .context("PostThreadMessageW WM_QUIT")?;
    }
    if let Some(join) = thread.join.take() {
        // The pump exits on WM_QUIT and the unhook+drop happens inside the
        // thread, so join is fast.
        let _ = join.join();
    }
    tracing::info!("keyboard hook uninstalled");
    Ok(())
}

/// Hot-update the active remap config without disturbing the hook.
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
    apply_effects(&effects);
}

fn hook_thread_main(tid_tx: std::sync::mpsc::Sender<Result<u32>>) {
    let hh = match unsafe {
        SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(low_level_proc),
            Some(HINSTANCE::default()),
            0,
        )
    } {
        Ok(h) => h,
        Err(e) => {
            let _ = tid_tx.send(Err(anyhow::anyhow!("SetWindowsHookExW: {e}")));
            return;
        }
    };

    let tid = unsafe { GetCurrentThreadId() };
    if tid_tx.send(Ok(tid)).is_err() {
        unsafe {
            let _ = UnhookWindowsHookEx(hh);
        }
        return;
    }

    // Tight Win32 message pump. GetMessageW blocks until a message arrives
    // (including the WM_KEYBOARD_LL hook callback, which is dispatched as a
    // message into this thread's queue). A PostThreadMessage(WM_QUIT) from
    // uninstall() makes GetMessageW return 0 and the loop exits.
    let mut msg = MSG::default();
    loop {
        let r = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if !r.as_bool() {
            break;
        }
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    unsafe {
        let _ = UnhookWindowsHookEx(hh);
    }
}

unsafe extern "system" fn low_level_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code != 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    let info: &KBDLLHOOKSTRUCT = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };

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

    // Fast-path: when not Caps and the FSM is Idle, almost every event is a
    // pure passthrough. Avoid the mutex + clone + Vec allocation on the hot
    // path so we stay well under the 300ms timeout even under load.
    if !is_caps {
        if let Some(state) = STATE.get() {
            if let Ok(s) = state.try_lock() {
                if matches!(s.machine.state_kind(), super::state::StateKind::Idle) {
                    drop(s);
                    return unsafe { CallNextHookEx(None, code, wparam, lparam) };
                }
            }
        }
    }

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

fn apply_effects(effects: &[Effect]) {
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

// We refer to HHOOK indirectly above; suppress the lint that thinks we don't.
#[allow(dead_code)]
fn _kept_for_hhook_use(_: HHOOK) {}
