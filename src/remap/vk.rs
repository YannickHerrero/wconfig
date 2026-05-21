//! Map between human key names (as written in config.toml) and Win32 VK_ codes.

use windows::Win32::UI::Input::KeyboardAndMouse::*;

/// Parse a config key name (case-insensitive) to a Windows VIRTUAL_KEY.
pub fn parse(name: &str) -> Option<VIRTUAL_KEY> {
    let n = name.trim().to_ascii_lowercase();
    Some(match n.as_str() {
        // Letters
        "a" => VK_A, "b" => VK_B, "c" => VK_C, "d" => VK_D, "e" => VK_E,
        "f" => VK_F, "g" => VK_G, "h" => VK_H, "i" => VK_I, "j" => VK_J,
        "k" => VK_K, "l" => VK_L, "m" => VK_M, "n" => VK_N, "o" => VK_O,
        "p" => VK_P, "q" => VK_Q, "r" => VK_R, "s" => VK_S, "t" => VK_T,
        "u" => VK_U, "v" => VK_V, "w" => VK_W, "x" => VK_X, "y" => VK_Y,
        "z" => VK_Z,
        // Digits
        "0" => VK_0, "1" => VK_1, "2" => VK_2, "3" => VK_3, "4" => VK_4,
        "5" => VK_5, "6" => VK_6, "7" => VK_7, "8" => VK_8, "9" => VK_9,
        // Function keys
        "f1" => VK_F1, "f2" => VK_F2, "f3" => VK_F3, "f4" => VK_F4,
        "f5" => VK_F5, "f6" => VK_F6, "f7" => VK_F7, "f8" => VK_F8,
        "f9" => VK_F9, "f10" => VK_F10, "f11" => VK_F11, "f12" => VK_F12,
        "f13" => VK_F13, "f14" => VK_F14, "f15" => VK_F15, "f16" => VK_F16,
        "f17" => VK_F17, "f18" => VK_F18, "f19" => VK_F19, "f20" => VK_F20,
        "f21" => VK_F21, "f22" => VK_F22, "f23" => VK_F23, "f24" => VK_F24,
        // Modifiers (side-specific where available)
        "ctrl" | "left_ctrl" | "lctrl" | "control" => VK_LCONTROL,
        "right_ctrl" | "rctrl" => VK_RCONTROL,
        "shift" | "left_shift" | "lshift" => VK_LSHIFT,
        "right_shift" | "rshift" => VK_RSHIFT,
        "alt" | "left_alt" | "lalt" | "menu" => VK_LMENU,
        "right_alt" | "ralt" => VK_RMENU,
        "win" | "left_win" | "lwin" | "super" | "meta" => VK_LWIN,
        "right_win" | "rwin" => VK_RWIN,
        // Whitespace / navigation
        "escape" | "esc" => VK_ESCAPE,
        "enter" | "return" => VK_RETURN,
        "tab" => VK_TAB,
        "backspace" | "back" => VK_BACK,
        "space" | "spacebar" => VK_SPACE,
        "delete" | "del" => VK_DELETE,
        "insert" | "ins" => VK_INSERT,
        "home" => VK_HOME,
        "end" => VK_END,
        "pageup" | "page_up" | "pgup" => VK_PRIOR,
        "pagedown" | "page_down" | "pgdn" => VK_NEXT,
        // Arrows
        "up" | "arrow_up" => VK_UP,
        "down" | "arrow_down" => VK_DOWN,
        "left" | "arrow_left" => VK_LEFT,
        "right" | "arrow_right" => VK_RIGHT,
        // Other
        "caps" | "capslock" | "caps_lock" => VK_CAPITAL,
        _ => return None,
    })
}

/// Human-readable label for a VK_ — used by the GUI key picker.
#[allow(dead_code)]
pub fn label(vk: VIRTUAL_KEY) -> &'static str {
    match vk {
        VK_LCONTROL => "left_ctrl",
        VK_RCONTROL => "right_ctrl",
        VK_LSHIFT => "left_shift",
        VK_RSHIFT => "right_shift",
        VK_LMENU => "left_alt",
        VK_RMENU => "right_alt",
        VK_LWIN => "left_win",
        VK_RWIN => "right_win",
        VK_ESCAPE => "escape",
        VK_RETURN => "enter",
        VK_TAB => "tab",
        VK_BACK => "backspace",
        VK_SPACE => "space",
        VK_CAPITAL => "caps_lock",
        VK_UP => "up",
        VK_DOWN => "down",
        VK_LEFT => "left",
        VK_RIGHT => "right",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_aliases() {
        assert_eq!(parse("escape"), Some(VK_ESCAPE));
        assert_eq!(parse("ESC"), Some(VK_ESCAPE));
        assert_eq!(parse(" left_ctrl "), Some(VK_LCONTROL));
        assert_eq!(parse("ctrl"), Some(VK_LCONTROL));
        assert_eq!(parse("f13"), Some(VK_F13));
        assert_eq!(parse("a"), Some(VK_A));
        assert_eq!(parse("nonsense"), None);
    }
}
