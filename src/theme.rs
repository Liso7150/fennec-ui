//! Theme presets and the extended ground palette for the Fennec gpui-kit
//! shell. Every preset is the web PWA's own theme map (`applyTheme` in
//! `web/app.js`) token for token, not a native departure: the PWA overrides
//! `--bg`, `--panel`, `--surface`, `--text` and `--brand` per theme and keeps
//! `--border`, `--text-muted`, `--hover` and the status colors constant across
//! all of them, so these presets do the same. Each theme carries its own
//! accent; dark keeps the original terracotta and the rest are tuned to their
//! own ground while still holding white button text.
//!
//! The PWA paints four different grounds: `--bg` behind the timeline,
//! `--panel` behind the top bar and the two side panels, `--surface` for cards,
//! and `--hover` for the row under the pointer. gpui's `Colors` has slots for
//! only two of them (`background` and `muted`), so the other two travel in
//! [`Palette`] beside the theme and are read through [`panel`] and [`hover`].

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::*;
use std::cell::RefCell;

/// Status colors as the PWA defines them: `--ok`, `--warn`, `--err`, `--blue`.
/// The PWA keeps them constant across every theme, light included, so every
/// preset shares these exact values.
const OK: [f32; 4] = [0.410, 0.498, 0.578, 1.0];
const WARN: [f32; 4] = [0.093, 0.687, 0.612, 1.0];
const ERR: [f32; 4] = [0.006, 0.821, 0.649, 1.0];
const INFO: [f32; 4] = [0.594, 0.874, 0.657, 1.0];

/// The seven `--brand` accents, one per PWA theme: dark #E9865D,
/// light #C2542A, ocean #1F6FE4, forest #2E9458, sunset #DC4A38,
/// lavender #8B5CF6, coffee #A9713F.
const ACCENT_DARK: [f32; 4] = [0.0488, 0.761, 0.639, 1.0];
const ACCENT_LIGHT: [f32; 4] = [0.046, 0.644, 0.463, 1.0];
const ACCENT_OCEAN: [f32; 4] = [0.599, 0.785, 0.508, 1.0];
const ACCENT_FOREST: [f32; 4] = [0.402, 0.526, 0.380, 1.0];
const ACCENT_SUNSET: [f32; 4] = [0.018, 0.701, 0.541, 1.0];
const ACCENT_LAVENDER: [f32; 4] = [0.718, 0.895, 0.663, 1.0];
const ACCENT_COFFEE: [f32; 4] = [0.079, 0.457, 0.455, 1.0];
/// `--border` #2E2E2E, constant across every PWA theme.
const BORDER: [f32; 4] = [0.0, 0.0, 0.180, 1.0];
/// `--text-muted` #A0A0A0, constant across every PWA theme.
const MUTED_FG: [f32; 4] = [0.0, 0.0, 0.627, 1.0];
/// `--hover` #2A2A2A, constant across every PWA theme.
const HOVER: [f32; 4] = [0.0, 0.0, 0.165, 1.0];

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
    // The PWA's `applyTheme` map, token for token: each theme overrides
    // bg/panel/surface/text/brand; border, muted text, hover and the status
    // colors are the same constants in every preset, exactly as the PWA keeps
    // them. HSLA values are the hex grounds converted (neutrals at s=0).
    vec![
        // dark: bg #141414, panel #191919, surface #1F1F1F, text #EDEDED, accent #E9865D.
        ThemePreset {
            name: "Dark", mode: ThemeMode::Dark,
            accent: ACCENT_DARK,
            background: [0.0, 0.0, 0.078, 1.0],
            muted: [0.0, 0.0, 0.122, 1.0],
            border: BORDER,
            foreground: [0.0, 0.0, 0.929, 1.0],
            muted_fg: MUTED_FG,
            panel: [0.0, 0.0, 0.098, 1.0],
            hover: HOVER,
            ok: OK, warn: WARN, err: ERR, info: INFO,
            theme_id: "dark",
        },
        // light: bg #FFFFFF, panel #F5F5F5, surface #E8E8E8, text #1A1A1A, accent #C2542A.
        ThemePreset {
            name: "Light", mode: ThemeMode::Light,
            accent: ACCENT_LIGHT,
            background: [0.0, 0.0, 1.0, 1.0],
            muted: [0.0, 0.0, 0.910, 1.0],
            border: BORDER,
            foreground: [0.0, 0.0, 0.102, 1.0],
            muted_fg: MUTED_FG,
            panel: [0.0, 0.0, 0.961, 1.0],
            hover: HOVER,
            ok: OK, warn: WARN, err: ERR, info: INFO,
            theme_id: "light",
        },
        // ocean: bg #0A1929, panel #0D2137, surface #132F4C, text #E0F2FE, accent #1F6FE4.
        ThemePreset {
            name: "Ocean", mode: ThemeMode::Dark,
            accent: ACCENT_OCEAN,
            background: [0.586, 0.608, 0.100, 1.0],
            muted: [0.585, 0.600, 0.186, 1.0],
            border: BORDER,
            foreground: [0.567, 0.938, 0.937, 1.0],
            muted_fg: MUTED_FG,
            panel: [0.587, 0.618, 0.133, 1.0],
            hover: HOVER,
            ok: OK, warn: WARN, err: ERR, info: INFO,
            theme_id: "ocean",
        },
        // forest: bg #0F1F0F, panel #1A2E1A, surface #253D25, text #E8F5E9, accent #2E9458.
        ThemePreset {
            name: "Forest", mode: ThemeMode::Dark,
            accent: ACCENT_FOREST,
            background: [0.333, 0.348, 0.090, 1.0],
            muted: [0.333, 0.245, 0.192, 1.0],
            border: BORDER,
            foreground: [0.346, 0.394, 0.935, 1.0],
            muted_fg: MUTED_FG,
            panel: [0.333, 0.278, 0.141, 1.0],
            hover: HOVER,
            ok: OK, warn: WARN, err: ERR, info: INFO,
            theme_id: "forest",
        },
        // sunset: bg #2D1B1B, panel #3D2424, surface #4D2D2D, text #FEE2E2, accent #DC4A38.
        ThemePreset {
            name: "Sunset", mode: ThemeMode::Dark,
            accent: ACCENT_SUNSET,
            background: [0.0, 0.250, 0.141, 1.0],
            muted: [0.0, 0.262, 0.239, 1.0],
            border: BORDER,
            foreground: [0.0, 0.933, 0.941, 1.0],
            muted_fg: MUTED_FG,
            panel: [0.0, 0.258, 0.190, 1.0],
            hover: HOVER,
            ok: OK, warn: WARN, err: ERR, info: INFO,
            theme_id: "sunset",
        },
        // lavender: bg #1E1B2E, panel #2A2640, surface #363252, text #EDE9FE, accent #8B5CF6.
        ThemePreset {
            name: "Lavender", mode: ThemeMode::Dark,
            accent: ACCENT_LAVENDER,
            background: [0.693, 0.260, 0.143, 1.0],
            muted: [0.688, 0.242, 0.259, 1.0],
            border: BORDER,
            foreground: [0.698, 0.913, 0.955, 1.0],
            muted_fg: MUTED_FG,
            panel: [0.692, 0.255, 0.200, 1.0],
            hover: HOVER,
            ok: OK, warn: WARN, err: ERR, info: INFO,
            theme_id: "lavender",
        },
        // coffee: bg #1A1410, panel #241C14, surface #2E2418, text #F5E6D3, accent #A9713F.
        ThemePreset {
            name: "Coffee", mode: ThemeMode::Dark,
            accent: ACCENT_COFFEE,
            background: [0.067, 0.238, 0.082, 1.0],
            muted: [0.091, 0.314, 0.137, 1.0],
            border: BORDER,
            foreground: [0.093, 0.630, 0.894, 1.0],
            muted_fg: MUTED_FG,
            panel: [0.083, 0.286, 0.110, 1.0],
            hover: HOVER,
            ok: OK, warn: WARN, err: ERR, info: INFO,
            theme_id: "coffee",
        },
    ]
}

pub fn find_preset_index(theme_id: &str) -> usize {
    theme_presets().iter().position(|p| p.id() == theme_id).unwrap_or(0)
}
