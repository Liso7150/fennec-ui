//! Theme presets and the extended ground palette for the Fennec gpui-kit
//! shell. The `Dark` preset is the web PWA's own "Fennec Night" palette, token
//! for token, so the two surfaces are the same design rather than two
//! approximations of it. The other presets are native-only departures a user
//! can pick in Settings.
//!
//! The PWA paints four different grounds: `--bg` behind the timeline,
//! `--panel` behind the top bar and the two side panels, `--surface` for cards,
//! and `--hover` for the row under the pointer. gpui's `Colors` has slots for
//! only two of them (`background` and `muted`), so the other two travel in
//! [`Palette`] beside the theme and are read through [`panel`] and [`hover`].

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::*;
use std::cell::RefCell;

/// Status colors as the PWA defines them on a dark ground: `--ok`, `--warn`,
/// `--err`, `--blue`. They are shared by every dark preset because a status
/// color has to stay readable whichever surface color sits behind it.
const OK: [f32; 4] = [0.410, 0.498, 0.578, 1.0];
const WARN: [f32; 4] = [0.093, 0.687, 0.612, 1.0];
const ERR: [f32; 4] = [0.006, 0.821, 0.649, 1.0];
const INFO: [f32; 4] = [0.594, 0.874, 0.657, 1.0];

/// The same four, darkened for the light ground.
const OK_LIGHT: [f32; 4] = [0.410, 0.500, 0.360, 1.0];
const WARN_LIGHT: [f32; 4] = [0.093, 0.600, 0.400, 1.0];
const ERR_LIGHT: [f32; 4] = [0.006, 0.650, 0.420, 1.0];
const INFO_LIGHT: [f32; 4] = [0.594, 0.600, 0.420, 1.0];

fn hsla4(c: [f32; 4]) -> Hsla {
    hsla(c[0], c[1], c[2], c[3])
}

/// The two grounds gpui's `Colors` cannot carry.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// `--panel`: the top bar and the side panels.
    pub panel: Hsla,
    /// `--hover`: the row the pointer is on.
    pub hover: Hsla,
}

thread_local! {
    static PALETTE: RefCell<Palette> = RefCell::new(Palette {
        panel: hsla(0.0, 0.0, 0.098, 1.0),
        hover: hsla(0.0, 0.0, 0.165, 1.0),
    });
}

/// The panel ground of the preset that is currently applied.
pub fn panel() -> Hsla {
    PALETTE.with(|p| p.borrow().panel)
}

/// The hover ground of the preset that is currently applied.
pub fn hover() -> Hsla {
    PALETTE.with(|p| p.borrow().hover)
}

#[derive(Clone, Debug)]
pub struct ThemePreset {
    pub name: &'static str,
    pub mode: ThemeMode,
    pub accent: [f32; 4],
    pub background: [f32; 4],
    pub muted: [f32; 4],
    pub border: [f32; 4],
    pub foreground: [f32; 4],
    pub muted_fg: [f32; 4],
    pub panel: [f32; 4],
    pub hover: [f32; 4],
    pub ok: [f32; 4],
    pub warn: [f32; 4],
    pub err: [f32; 4],
    pub info: [f32; 4],
    pub theme_id: &'static str,
}

impl ThemePreset {
    pub fn apply(&self, cx: &mut gpui::App) {
        Theme::update(cx, |theme| {
            theme.mode = self.mode;
            theme.colors.accent = hsla4(self.accent);
            theme.colors.primary = hsla4(self.accent);
            theme.colors.button_primary = hsla4(self.accent);
            theme.colors.link = hsla4(self.accent);
            theme.colors.background = hsla4(self.background);
            theme.colors.muted = hsla4(self.muted);
            theme.colors.border = hsla4(self.border);
            theme.colors.foreground = hsla4(self.foreground);
            theme.colors.muted_foreground = hsla4(self.muted_fg);
            theme.colors.green = hsla4(self.ok);
            theme.colors.yellow = hsla4(self.warn);
            theme.colors.red = hsla4(self.err);
            theme.colors.blue = hsla4(self.info);
        });
        PALETTE.with(|p| {
            *p.borrow_mut() = Palette { panel: hsla4(self.panel), hover: hsla4(self.hover) };
        });
    }

    pub fn id(&self) -> &str {
        self.theme_id
    }
}

pub fn theme_presets() -> Vec<ThemePreset> {
    vec![
        // #141414 ground, #191919 panels, #1F1F1F cards, #2A2A2A hover,
        // #2E2E2E border, #E9865D terracotta, #EDEDED text, #A0A0A0 muted —
        // the PWA's `:root` values.
        ThemePreset {
            name: "Dark", mode: ThemeMode::Dark,
            accent: [0.049, 0.761, 0.620, 1.0],
            background: [0.0, 0.0, 0.078, 1.0],
            muted: [0.0, 0.0, 0.122, 1.0],
            border: [0.0, 0.0, 0.180, 1.0],
            foreground: [0.0, 0.0, 0.929, 1.0],
            muted_fg: [0.0, 0.0, 0.627, 1.0],
            panel: [0.0, 0.0, 0.098, 1.0],
            hover: [0.0, 0.0, 0.165, 1.0],
            ok: OK, warn: WARN, err: ERR, info: INFO,
            theme_id: "dark",
        },
        ThemePreset {
            name: "Light", mode: ThemeMode::Light,
            accent: [210.0 / 360.0, 0.5, 0.5, 1.0],
            background: [0.0, 0.0, 0.98, 1.0],
            muted: [0.0, 0.0, 0.94, 1.0],
            border: [0.0, 0.0, 0.88, 1.0],
            foreground: [0.0, 0.0, 0.12, 1.0],
            muted_fg: [0.0, 0.0, 0.38, 1.0],
            panel: [0.0, 0.0, 0.96, 1.0],
            hover: [0.0, 0.0, 0.90, 1.0],
            ok: OK_LIGHT, warn: WARN_LIGHT, err: ERR_LIGHT, info: INFO_LIGHT,
            theme_id: "light",
        },
        ThemePreset {
            name: "Ocean", mode: ThemeMode::Dark,
            accent: [55.0 / 360.0, 0.7, 0.5, 1.0],
            background: [0.6, 0.4, 0.08, 1.0],
            muted: [0.6, 0.35, 0.12, 1.0],
            border: [0.6, 0.3, 0.2, 1.0],
            foreground: [0.0, 0.0, 0.93, 1.0],
            muted_fg: [0.0, 0.0, 0.62, 1.0],
            panel: [0.6, 0.4, 0.10, 1.0],
            hover: [0.6, 0.35, 0.17, 1.0],
            ok: OK, warn: WARN, err: ERR, info: INFO,
            theme_id: "ocean",
        },
        ThemePreset {
            name: "Forest", mode: ThemeMode::Dark,
            accent: [35.0 / 360.0, 0.6, 0.45, 1.0],
            background: [0.33, 0.3, 0.06, 1.0],
            muted: [0.33, 0.25, 0.1, 1.0],
            border: [0.33, 0.2, 0.16, 1.0],
            foreground: [0.0, 0.0, 0.93, 1.0],
            muted_fg: [0.0, 0.0, 0.62, 1.0],
            panel: [0.33, 0.3, 0.08, 1.0],
            hover: [0.33, 0.25, 0.15, 1.0],
            ok: OK, warn: WARN, err: ERR, info: INFO,
            theme_id: "forest",
        },
        ThemePreset {
            name: "Sunset", mode: ThemeMode::Dark,
            accent: [10.0 / 360.0, 0.75, 0.55, 1.0],
            background: [0.05, 0.5, 0.08, 1.0],
            muted: [0.05, 0.45, 0.12, 1.0],
            border: [0.05, 0.4, 0.2, 1.0],
            foreground: [0.0, 0.0, 0.93, 1.0],
            muted_fg: [0.0, 0.0, 0.62, 1.0],
            panel: [0.05, 0.5, 0.10, 1.0],
            hover: [0.05, 0.45, 0.17, 1.0],
            ok: OK, warn: WARN, err: ERR, info: INFO,
            theme_id: "sunset",
        },
        ThemePreset {
            name: "Lavender", mode: ThemeMode::Dark,
            accent: [75.0 / 360.0, 0.55, 0.6, 1.0],
            background: [0.75, 0.3, 0.08, 1.0],
            muted: [0.75, 0.25, 0.12, 1.0],
            border: [0.75, 0.2, 0.2, 1.0],
            foreground: [0.0, 0.0, 0.93, 1.0],
            muted_fg: [0.0, 0.0, 0.62, 1.0],
            panel: [0.75, 0.3, 0.10, 1.0],
            hover: [0.75, 0.25, 0.17, 1.0],
            ok: OK, warn: WARN, err: ERR, info: INFO,
            theme_id: "lavender",
        },
        ThemePreset {
            name: "Coffee", mode: ThemeMode::Dark,
            accent: [30.0 / 360.0, 0.6, 0.65, 1.0],
            background: [0.07, 0.5, 0.1, 1.0],
            muted: [0.07, 0.45, 0.15, 1.0],
            border: [0.07, 0.4, 0.22, 1.0],
            foreground: [0.0, 0.0, 0.93, 1.0],
            muted_fg: [0.0, 0.0, 0.62, 1.0],
            panel: [0.07, 0.5, 0.12, 1.0],
            hover: [0.07, 0.45, 0.20, 1.0],
            ok: OK, warn: WARN, err: ERR, info: INFO,
            theme_id: "coffee",
        },
    ]
}

pub fn find_preset_index(theme_id: &str) -> usize {
    theme_presets().iter().position(|p| p.id() == theme_id).unwrap_or(0)
}
