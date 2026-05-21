# wconfig

A small Windows configuration utility, written in Rust. Acts as a **hotkey
daemon** — global key combinations like `Alt+Enter` or `Alt+B` can launch
apps, open URLs, run scripts, or focus a running window.

State lives in a single hand-editable TOML file. A tray icon hosts a small
egui-based settings GUI for editing it. Changes to the file from any external
editor are picked up within ~250ms.

> The previous tap-vs-hold / Caps Lock remap backend used `WH_KEYBOARD_LL`,
> which is routinely blocked by anti-virus and corporate EDR. It was removed;
> if it comes back it'll be on a different backend (e.g. the Interception
> driver or the registry Scancode Map).

## Build

This repo cross-compiles from WSL/Linux to Windows by default.

```sh
make build           # cargo build --release --target x86_64-pc-windows-gnu
make install         # copy wconfig.exe into your Windows apps folder
```

The output is at `target/x86_64-pc-windows-gnu/release/wconfig.exe`.

## Config file

The config lives at `%APPDATA%/wconfig/config/config.toml` (created on first
run with sensible defaults). Edit it directly or use the tray → Settings GUI.

```toml
version = 1
theme   = "paper"                    # paper | stone | sage | clay | ink

[daemon]
autostart        = false             # add to Windows startup (HKCU Run key)
start_minimized  = true              # boot to tray, no GUI on startup

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

### Key strings

For binding `key` strings, see the [`global-hotkey`](https://crates.io/crates/global-hotkey)
crate — `Alt+Enter`, `Ctrl+Shift+B`, `Win+1`, plain `F13`, etc.

## How it works

- Global hotkeys are registered through `RegisterHotKey` via the
  `global-hotkey` crate.
- Tray, autostart, single-instance, and the named-event "show GUI" signal
  all use straight Win32 APIs.

## Troubleshooting

Run from a console with `WCONFIG_LOG=wconfig=debug` set to get verbose logs
in the terminal. The tray's *Reload config* item re-reads the file by hand
if the watcher missed an edit. Logs are also written to
`%APPDATA%/wconfig/config/logs/wconfig.log.YYYY-MM-DD`.
