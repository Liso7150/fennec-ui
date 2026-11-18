//! Theme presets shared by both Fennec gpui-kit shells.

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::*;

#[derive(Clone, Debug)]
pub struct ThemePreset {
    pub name: &'static str,
    pub mode: ThemeMode,
    pub accent: [f32; 4],
    pub background: [f32; 4],
    pub muted: [f32; 4],
    pub border: [f32; 4],
    pub theme_id: &'static str,
}

impl ThemePreset {
    pub fn apply(&self, cx: &mut gpui::App) {
        Theme::update(cx, |theme| {
            theme.mode = self.mode;
            theme.colors.accent = hsla(self.accent[0], self.accent[1], self.accent[2], self.accent[3]);
            theme.colors.primary = hsla(self.accent[0], self.accent[1], self.accent[2], self.accent[3]);
            theme.colors.button_primary = hsla(self.accent[0], self.accent[1], self.accent[2], self.accent[3]);
            theme.colors.link = hsla(self.accent[0], self.accent[1], self.accent[2], self.accent[3]);
            theme.colors.background = hsla(self.background[0], self.background[1], self.background[2], self.background[3]);
            theme.colors.muted = hsla(self.muted[0], self.muted[1], self.muted[2], self.muted[3]);
            theme.colors.border = hsla(self.border[0], self.border[1], self.border[2], self.border[3]);
        });
    }

    pub fn id(&self) -> &str {
        self.theme_id
    }
}

pub fn theme_presets() -> Vec<ThemePreset> {
    vec![
        ThemePreset {
            name: "Dark", mode: ThemeMode::Dark,
            accent: [210.0/360.0, 0.5, 0.5, 1.0],
            background: [0.0, 0.0, 0.08, 1.0],
            muted: [0.0, 0.0, 0.12, 1.0],
            border: [0.0, 0.0, 0.18, 1.0],
            theme_id: "dark",
        },
        ThemePreset {
            name: "Light", mode: ThemeMode::Light,
            accent: [210.0/360.0, 0.5, 0.5, 1.0],
            background: [0.0, 0.0, 0.98, 1.0],
            muted: [0.0, 0.0, 0.94, 1.0],
            border: [0.0, 0.0, 0.88, 1.0],
            theme_id: "light",
        },
        ThemePreset {
            name: "Ocean", mode: ThemeMode::Dark,
            accent: [55.0/360.0, 0.7, 0.5, 1.0],
            background: [0.6, 0.4, 0.08, 1.0],
            muted: [0.6, 0.35, 0.12, 1.0],
            border: [0.6, 0.3, 0.2, 1.0],
            theme_id: "ocean",
        },
        ThemePreset {
            name: "Forest", mode: ThemeMode::Dark,
            accent: [35.0/360.0, 0.6, 0.45, 1.0],
            background: [0.33, 0.3, 0.06, 1.0],
            muted: [0.33, 0.25, 0.1, 1.0],
            border: [0.33, 0.2, 0.16, 1.0],
            theme_id: "forest",
        },
        ThemePreset {
            name: "Sunset", mode: ThemeMode::Dark,
            accent: [10.0/360.0, 0.75, 0.55, 1.0],
            background: [0.05, 0.5, 0.08, 1.0],
            muted: [0.05, 0.45, 0.12, 1.0],
            border: [0.05, 0.4, 0.2, 1.0],
            theme_id: "sunset",
        },
        ThemePreset {
            name: "Lavender", mode: ThemeMode::Dark,
            accent: [75.0/360.0, 0.55, 0.6, 1.0],
            background: [0.75, 0.3, 0.08, 1.0],
            muted: [0.75, 0.25, 0.12, 1.0],
            border: [0.75, 0.2, 0.2, 1.0],
            theme_id: "lavender",
        },
        ThemePreset {
            name: "Coffee", mode: ThemeMode::Dark,
            accent: [30.0/360.0, 0.6, 0.65, 1.0],
            background: [0.07, 0.5, 0.1, 1.0],
            muted: [0.07, 0.45, 0.15, 1.0],
            border: [0.07, 0.4, 0.22, 1.0],
            theme_id: "coffee",
        },
    ]
}

pub fn find_preset_index(theme_id: &str) -> usize {
    theme_presets().iter().position(|p| p.id() == theme_id).unwrap_or(0)
}