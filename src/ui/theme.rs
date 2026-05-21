use eframe::egui::{Color32, Context, Stroke, Visuals};

use crate::config::Theme;

#[allow(dead_code)]
pub struct Palette {
    pub paper: Color32,
    pub ink: Color32,
    pub accent: Color32,
    pub ink_soft: Color32,
    pub ink_faint: Color32,
    pub muted: Color32,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

pub fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Paper => Palette {
            paper: rgb(0xF4, 0xEB, 0xD9),
            ink: rgb(0x2B, 0x24, 0x1B),
            accent: rgb(0xB5, 0x59, 0x3A),
            ink_soft: rgb(0x6B, 0x5E, 0x4E),
            ink_faint: rgb(0xA4, 0x95, 0x80),
            muted: rgb(0xE5, 0xD8, 0xC0),
        },
        Theme::Stone => Palette {
            paper: rgb(0xE6, 0xE8, 0xEA),
            ink: rgb(0x2D, 0x33, 0x38),
            accent: rgb(0x4A, 0x6B, 0x8A),
            ink_soft: rgb(0x5F, 0x69, 0x72),
            ink_faint: rgb(0x9B, 0xA3, 0xAB),
            muted: rgb(0xD3, 0xD7, 0xDB),
        },
        Theme::Sage => Palette {
            paper: rgb(0xDD, 0xE4, 0xD2),
            ink: rgb(0x2C, 0x35, 0x26),
            accent: rgb(0x3F, 0x5C, 0x32),
            ink_soft: rgb(0x5E, 0x69, 0x54),
            ink_faint: rgb(0x97, 0xA2, 0x87),
            muted: rgb(0xCC, 0xD4, 0xBE),
        },
        Theme::Clay => Palette {
            paper: rgb(0xE8, 0xD4, 0xC2),
            ink: rgb(0x3A, 0x28, 0x20),
            accent: rgb(0x9E, 0x45, 0x21),
            ink_soft: rgb(0x6E, 0x55, 0x48),
            ink_faint: rgb(0xA8, 0x90, 0x7F),
            muted: rgb(0xD9, 0xC0, 0xA8),
        },
        Theme::Ink => Palette {
            paper: rgb(0x00, 0x00, 0x00),
            ink: rgb(0xE4, 0xE1, 0xD8),
            accent: rgb(0xF5, 0xEF, 0xE0),
            ink_soft: rgb(0x9A, 0x96, 0x90),
            ink_faint: rgb(0x5A, 0x57, 0x52),
            muted: rgb(0x15, 0x15, 0x15),
        },
    }
}

pub fn apply(ctx: &Context, theme: Theme) {
    let p = palette(theme);
    let mut v = if matches!(theme, Theme::Ink) {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    v.window_fill = p.paper;
    v.panel_fill = p.paper;
    v.extreme_bg_color = p.paper;
    v.override_text_color = Some(p.ink);

    v.selection.bg_fill = p.accent;
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.hyperlink_color = p.accent;

    let ink_stroke = Stroke::new(1.0, p.ink);
    v.widgets.noninteractive.fg_stroke = ink_stroke;
    v.widgets.inactive.fg_stroke = ink_stroke;
    v.widgets.active.fg_stroke = ink_stroke;
    v.widgets.hovered.fg_stroke = ink_stroke;

    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.ink_faint);

    v.widgets.inactive.bg_fill = p.muted;
    v.widgets.inactive.weak_bg_fill = p.muted;
    v.widgets.hovered.bg_fill = p.muted;
    v.widgets.hovered.weak_bg_fill = p.muted;

    ctx.set_visuals(v);
}
