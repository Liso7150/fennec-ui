//! The PWA's own line icons, rendered natively. `web/app.js` keeps its icon
//! set as inline SVG path data (`UI_ICON_PATHS`) drawn at 24x24 with a 1.8
//! round stroke; the bytes here are those same paths, embedded so the native
//! shell paints the identical monochrome glyph instead of a color emoji.
//!
//! gpui's `svg()` element resolves its path through the window's
//! [`AssetSource`], so [`FennecAssets`] serves these icons under
//! `fennec-icons/<name>.svg` and delegates everything else to gpui-kit's own
//! embedded assets. The renderer turns the SVG into an alpha mask and tints
//! it with the element's text color, which is how [`ui_icon`] paints each
//! glyph in the theme's accent or muted foreground like the PWA does.

use std::borrow::Cow;

use gpui_kit::{px, AssetSource, Hsla, IntoElement, SharedString, Styled};

/// The PWA's `UI_ICON_PATHS` entries (and the inspector's `context-icon`
/// paths from `web/index.html`), verbatim. Each is the inner markup of a
/// 24x24 viewBox; the stroke styling `.ui-icon` applies in CSS is applied by
/// [`icon_svg`] when the bytes are served.
const ICONS: &[(&str, &str)] = &[
    ("settings", "<path d=\"M12 3v2m0 14v2M3 12h2m14 0h2M5.64 5.64l1.42 1.42m9.88 9.88 1.42 1.42m0-12.72-1.42 1.42M7.06 16.94l-1.42 1.42\"/><circle cx=\"12\" cy=\"12\" r=\"3.5\"/>"),
    ("panel", "<rect x=\"4\" y=\"4\" width=\"16\" height=\"16\" rx=\"2\"/><path d=\"M15 4v16\"/>"),
    ("list", "<path d=\"M6 6h12M6 12h12M6 18h12\"/>"),
    ("plus", "<path d=\"M12 5v14M5 12h14\"/>"),
    ("clock", "<circle cx=\"12\" cy=\"12\" r=\"8\"/><path d=\"M12 8v4l3 2\"/>"),
    ("search", "<circle cx=\"10.5\" cy=\"10.5\" r=\"5.5\"/><path d=\"m15 15 4 4\"/>"),
    ("paperclip", "<path d=\"m8 12 5.5-5.5a3 3 0 0 1 4.24 4.24l-6.36 6.36a4 4 0 0 1-5.66-5.66l6.01-6.01\"/>"),
    ("mic", "<rect x=\"9\" y=\"4\" width=\"6\" height=\"11\" rx=\"3\"/><path d=\"M6 11a6 6 0 0 0 12 0M12 17v3M9 20h6\"/>"),
    ("stop", "<rect x=\"7\" y=\"7\" width=\"10\" height=\"10\" rx=\"1.5\"/>"),
    ("github", "<path d=\"M12 3a9 9 0 0 0-2.84 17.54c.45.08.62-.2.62-.43v-1.52c-2.52.55-3.05-1.06-3.05-1.06-.41-1.05-1-1.33-1-1.33-.82-.56.06-.55.06-.55.91.06 1.39.94 1.39.94.8 1.39 2.1.99 2.61.76.08-.59.31-.99.57-1.22-2.01-.23-4.12-1-4.12-4.46 0-.99.35-1.8.93-2.44-.09-.23-.4-1.15.09-2.4 0 0 .76-.24 2.48.93a8.6 8.6 0 0 1 4.52 0c1.72-1.17 2.48-.93 2.48-.93.49 1.25.18 2.17.09 2.4.58.64.93 1.45.93 2.44 0 3.47-2.11 4.23-4.12 4.46.32.28.61.83.61 1.68v2.49c0 .24.16.52.62.43A9 9 0 0 0 12 3Z\"/>"),
    ("workspace", "<path d=\"M4 7.5A2.5 2.5 0 0 1 6.5 5h4l2 2H18a2 2 0 0 1 2 2v7.5a2.5 2.5 0 0 1-2.5 2.5h-11A2.5 2.5 0 0 1 4 16.5z\"/>"),
    ("file", "<path d=\"M6 3h8l4 4v14H6z\"/><path d=\"M14 3v5h5M9 13h6M9 17h6\"/>"),
    ("image", "<rect x=\"4\" y=\"5\" width=\"16\" height=\"14\" rx=\"2\"/><circle cx=\"9\" cy=\"10\" r=\"1.5\"/><path d=\"m5 17 4-4 3 3 2-2 5 4\"/>"),
    ("queue", "<path d=\"M6 6h12M6 12h12M6 18h8\"/><circle cx=\"4\" cy=\"6\" r=\".7\" fill=\"#000\"/><circle cx=\"4\" cy=\"12\" r=\".7\" fill=\"#000\"/><circle cx=\"4\" cy=\"18\" r=\".7\" fill=\"#000\"/>"),
    ("subagents", "<path d=\"M8 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm8-1a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5ZM3.5 19a4.5 4.5 0 0 1 9 0M14 18a4 4 0 0 1 6.5-3.1\"/>"),
    ("tool", "<path d=\"m14 6 4 4M5 19l6.5-6.5M13 7a4 4 0 0 0 5 5l-7 7H6v-5l7-7Z\"/>"),
    ("volume", "<path d=\"M5 10v4h3l4 3V7l-4 3H5Z\"/><path d=\"M16 9a4 4 0 0 1 0 6M18.5 6.5a7.5 7.5 0 0 1 0 11\"/>"),
];

/// The full SVG document for one icon, styled the way `.ui-icon` styles it:
/// no fill, a 1.8 round stroke. The stroke color is irrelevant to the final
/// paint — gpui keeps only the alpha mask and tints it — but it must be
/// opaque for the mask to carry the stroke at all.
fn icon_svg(name: &str) -> Option<String> {
    let (_, inner) = ICONS.iter().find(|(n, _)| *n == name)?;
    Some(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"#000\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{inner}</svg>"
    ))
}

/// The window's asset source: the PWA icons embedded above, everything else
/// from gpui-kit's own embedded assets (its components load their chevrons
/// and checks through the same source).
pub struct FennecAssets;

impl AssetSource for FennecAssets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        if let Some(name) = path.strip_prefix("fennec-icons/").and_then(|p| p.strip_suffix(".svg")) {
            if let Some(svg) = icon_svg(name) {
                return Ok(Some(Cow::Owned(svg.into_bytes())));
            }
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<SharedString>> {
        gpui_kit::assets::Assets.list(path)
    }
}

/// One PWA line icon at `size` px, tinted `color` — the native equivalent of
/// the PWA's `iconSvg(name)` inside a `.ui-icon` span. Pass `t.accent` or
/// `t.muted_foreground` the way the PWA tints its icons.
pub fn ui_icon(name: &str, size: f32, color: Hsla) -> impl IntoElement {
    gpui_kit::svg()
        .path(SharedString::from(format!("fennec-icons/{name}.svg")))
        .w(px(size))
        .h(px(size))
        .flex_shrink_0()
        .text_color(color)
}

#[cfg(test)]
mod tests {
    use super::{icon_svg, ICONS};

    #[test]
    fn every_icon_serves_a_well_formed_svg() {
        for (name, _) in ICONS {
            let svg = icon_svg(name).expect("icon must resolve");
            assert!(svg.starts_with("<svg xmlns="), "{name}");
            assert!(svg.contains("viewBox=\"0 0 24 24\""), "{name}");
            assert!(svg.ends_with("</svg>"), "{name}");
        }
        assert!(icon_svg("no-such-icon").is_none());
    }
}