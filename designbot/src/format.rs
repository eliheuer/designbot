//! Canonical image formats — the sizes that matter for social + web.
//!
//! Pixel dimensions and a sensible house margin per aspect ratio, so a script
//! says `Format::Square` instead of memorizing `2048.0`. Pair with the CLI's
//! `--social` flag, which tags sRGB and keeps the PNG lossless on X.

/// A named canvas format at a fixed aspect ratio.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// 1:1 — X / LinkedIn / Instagram card (and square animation master).
    Square,
    /// 4:5 — Instagram feed / carousel slide.
    Portrait,
    /// 1.91:1 — the X / LinkedIn link-card ratio.
    Landscape,
    /// 9:16 — Reels / Stories / TikTok.
    Vertical,
}

impl Format {
    /// (width, height) in pixels.
    pub fn size(self) -> (f64, f64) {
        match self {
            Format::Square => (2048.0, 2048.0),
            Format::Portrait => (1080.0, 1350.0),
            Format::Landscape => (2520.0, 1320.0),
            Format::Vertical => (1080.0, 1920.0),
        }
    }
    pub fn w(self) -> f64 {
        self.size().0
    }
    pub fn h(self) -> f64 {
        self.size().1
    }
    /// A comfortable default outer margin for this canvas.
    pub fn margin(self) -> f64 {
        match self {
            Format::Square => 180.0,
            Format::Portrait => 96.0,
            Format::Landscape => 120.0,
            Format::Vertical => 96.0,
        }
    }
    /// Short slug for filenames / CLI args.
    pub fn slug(self) -> &'static str {
        match self {
            Format::Square => "square",
            Format::Portrait => "portrait",
            Format::Landscape => "landscape",
            Format::Vertical => "vertical",
        }
    }
    /// Parse a slug (`"square"`, `"sq"`, `"portrait"`, `"wide"`, `"reel"`, …).
    pub fn from_slug(s: &str) -> Option<Format> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "square" | "sq" => Format::Square,
            "portrait" | "feed" | "carousel" => Format::Portrait,
            "landscape" | "wide" | "og" => Format::Landscape,
            "vertical" | "reel" | "story" => Format::Vertical,
            _ => return None,
        })
    }
    /// All four, for "render every format" loops.
    pub fn all() -> [Format; 4] {
        [Format::Square, Format::Portrait, Format::Landscape, Format::Vertical]
    }
}
