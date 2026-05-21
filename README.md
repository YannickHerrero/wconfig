# wconfig

A small Windows configuration utility, written in Rust. It does two things:

1. **Remaps keys** at the OS level (currently Caps Lock), with both plain
   one-to-one remap and **tap-vs-hold** dual-function support — e.g. *tap
   Caps = Escape, hold Caps = Ctrl*.
2. **Acts as a hotkey daemon** — global key combinations like `Alt+Enter` or
   `Alt+B` can launch apps, open URLs, run scripts, or focus a running
   window.

Everything is stored in a single hand-editable TOML file. A tray icon hosts a
small egui-based settings GUI for editing it. Changes to the file from any
external editor are picked up within ~250ms.

## Build

This repo cross-compiles from WSL/Linux to Windows by default.

```sh
make build           # cargo build --release --target x86_64-pc-windows-gnu
make install         # copy wconfig.exe into your Windows apps folder
```

The output is at `target/x86_64-pc-windows-gnu/release/wconfig.exe`.

## Config file

The config lives at `%APPDATA%/wconfig/config.toml` (created on first run with
sensible defaults). Edit it directly or use the tray → Settings GUI.

```toml
version = 1
theme   = "paper"                    # paper | stone | sage | clay | ink

[daemon]
autostart        = false             # add to Windows startup (HKCU Run key)
start_minimized  = true              # boot to tray, no GUI on startup
tap_timeout_ms   = 180               # caps tap-vs-hold threshold

# Caps Lock remap. Modes:
#   "off"     pass through unchanged
#   "single"  caps always becomes `single_to`
#   "dual"    short tap = `tap`, hold-with-other-key or hold-past-threshold = `hold`
[remap.caps_lock]
mode      = "dual"
single_to = "left_ctrl"
tap       = "escape"
hold      = "left_ctrl"

# Each [[bindings]] is one global hotkey -> one action.
# `key` syntax is the global-hotkey crate's: "Alt+Enter", "Ctrl+Shift+B", etc.
# action.type is one of: launch | url | script | focus_or_launch

[[bindings]]
label = "Open terminal"
key   = "Alt+Enter"
[bindings.action]
type    = "launch"
command = "\"C:/Program Files/WezTerm/wezterm-gui.exe\""

[[bindings]]
label = "Open GitHub"
key   = "Ctrl+Alt+G"
[bindings.action]
type = "url"
url  = "https://github.com"

[[bindings]]
label = "Daily backup"
key   = "Ctrl+Alt+Shift+B"
[bindings.action]
type   = "script"
shell  = "powershell"                # powershell | cmd | pwsh
script = """
$dest = "D:/backups/$(Get-Date -Format yyyy-MM-dd).zip"
Compress-Archive -Path "$env:USERPROFILE/Documents" -DestinationPath $dest
"""

[[bindings]]
label = "Focus or launch Firefox"
key   = "Alt+B"
[bindings.action]
type           = "focus_or_launch"
exe_path       = "C:/Program Files/Mozilla Firefox/firefox.exe"
match_basename = true
launch_args    = []
```

### Key names

For the remap fields (`single_to`, `tap`, `hold`): `escape`, `left_ctrl`,
`right_ctrl`, `left_shift`, `right_shift`, `left_alt`, `right_alt`, `tab`,
`enter`, `backspace`, `space`, `f1`..`f24`, `a`..`z`, `0`..`9`, `up`/`down`/
`left`/`right`, `home`/`end`/`pageup`/`pagedown`/`insert`/`delete`. Names are
case-insensitive.

For binding `key` strings, see the [`global-hotkey`](https://crates.io/crates/global-hotkey)
crate — `Alt+Enter`, `Ctrl+Shift+B`, `Win+1`, plain `F13`, etc.

## How it works

- The keyboard hook uses `SetWindowsHookExW(WH_KEYBOARD_LL)`. The daemon must
  stay running (it lives in the tray) for remaps to take effect.
- Synthetic key events injected by wconfig carry a `dwExtraInfo` marker
  (`0x57434E46` = `WCNF`) so the hook recognises and skips its own events.
- Global hotkeys are registered through `RegisterHotKey` via the
  `global-hotkey` crate. Because LL hooks process events *before*
  `RegisterHotKey` matching, a Caps-→-Ctrl remap means that physical Caps+G
  will trigger a hotkey bound as `Ctrl+G`.
- Tray, autostart, single-instance, and the named-event "show GUI"
  signal all use straight Win32 APIs.

## Known limitations

- **UIPI**: a non-elevated wconfig can't inject input into elevated windows
  (Task Manager, regedit). Caps remap will appear inactive while one of those
  is focused. Run elevated only if you need this.
- **Antivirus**: LL keyboard hooks look like keyloggers to scanners. wconfig
  never logs or persists keystroke data, but third-party AV may quarantine
  the binary anyway.
- v0.1 ships free-text fields for key names; a structured dropdown is a
  follow-up.

## Troubleshooting

Run from a console with `WCONFIG_LOG=wconfig=debug` set to get verbose logs
in the terminal. The tray's *Reload config* item re-reads the file by hand
if the watcher missed an edit.
