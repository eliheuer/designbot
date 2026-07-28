//! Color themes — reusable design systems.
//!
//! A [`Theme`] is a complete, named set of roles: neutrals (ground, ink,
//! furniture, rules, figure pen) plus the shared semantic hues, all built on
//! the OKLCH engine in [`Color::oklch`](crate::Color::oklch) so they read at
//! even intensity on any ground. Scripts reference `t.ground`, `t.ink`, … so
//! swapping `Theme::dark()` → `Theme::light()` re-skins a whole composition.
//!
//! A font project can use these defaults or define its own `Theme` value.

use crate::Color;

/// A complete, swappable color scheme.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub name: &'static str,
    // neutrals — the theme's personality
    /// Page background.
    pub ground: Color,
    /// Primary type: specimen fill, display glyphs.
    pub ink: Color,
    /// Monospace corner furniture (labels, captions).
    pub furniture: Color,
    /// Hairline rules.
    pub rule: Color,
    /// Drawing pen for technical figures (glyph outline, dimension lines).
    pub pen: Color,
    /// Subtle line color for design [`Grid`](crate::grid::Grid) overlays.
    pub grid: Color,
    // shared semantic hues
    pub green: Color,
    pub red: Color,
    pub yellow: Color,
    pub orange: Color,
    pub blue: Color,
    pub purple: Color,
    /// The single brand accent a minimalist card may use.
    pub accent: Color,
}

/// The shared hue set (OKLCH), identical across themes so figures keep their
/// meaning when the neutrals flip.
fn hues() -> (Color, Color, Color, Color, Color, Color) {
    (
        Color::oklch(0.67, 0.160, 159.0), // green
        Color::oklch(0.66, 0.175, 28.0),  // red
        Color::oklch(0.88, 0.160, 92.0),  // yellow
        Color::oklch(0.74, 0.160, 52.0),  // orange
        Color::oklch(0.65, 0.160, 258.0), // blue
        Color::oklch(0.65, 0.160, 302.0), // purple
    )
}

impl Theme {
    /// Dark ground, light type — the Font.Garden house default.
    pub fn dark() -> Self {
        let (green, red, yellow, orange, blue, purple) = hues();
        Theme {
            name: "dark",
            ground: Color::rgb(0x28, 0x28, 0x28),
            ink: Color::rgb(0xbe, 0xbe, 0xbe),
            furniture: Color::rgb(0x92, 0x92, 0x8e),
            rule: Color::rgb(0x3a, 0x3a, 0x3a),
            pen: Color::rgb(0xe6, 0xe6, 0xe6),
            grid: Color::rgba(0xff, 0xff, 0xff, 0x22),
            green, red, yellow, orange, blue, purple,
            accent: yellow,
        }
    }

    /// Light ground, dark type — the same cards, flipped.
    pub fn light() -> Self {
        let (green, red, yellow, orange, blue, purple) = hues();
        Theme {
            name: "light",
            ground: Color::rgb(0xef, 0xee, 0xea),
            ink: Color::rgb(0x24, 0x24, 0x22),
            furniture: Color::rgb(0x6a, 0x6a, 0x66),
            rule: Color::rgb(0xd6, 0xd5, 0xd0),
            pen: Color::rgb(0x10, 0x10, 0x10),
            grid: Color::rgba(0x00, 0x00, 0x00, 0x1e),
            green, red, yellow, orange, blue, purple,
            accent: red,
        }
    }

    /// Near-black ground, bright ink — high-contrast variant.
    pub fn black() -> Self {
        Theme {
            name: "black",
            ground: Color::rgb(0x0a, 0x0a, 0x0a),
            ink: Color::rgb(0xe6, 0xe6, 0xe6),
            furniture: Color::rgb(0x78, 0x78, 0x78),
            rule: Color::rgb(0x28, 0x28, 0x28),
            pen: Color::rgb(0xe6, 0xe6, 0xe6),
            ..Theme::dark()
        }
    }

    /// Look up a theme by name (`"dark"`, `"light"`, `"black"`). Unknown → dark.
    pub fn by_name(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "light" => Theme::light(),
            "black" => Theme::black(),
            _ => Theme::dark(),
        }
    }
}
