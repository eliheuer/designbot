/// Color representation for DesignBot
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Create a new color from RGB values (0-255)
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create a new color from RGBA values (0-255)
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create a grayscale color
    pub fn gray(value: u8) -> Self {
        Self::rgb(value, value, value)
    }

    /// Create a black color
    pub fn black() -> Self {
        Self::rgb(0, 0, 0)
    }

    /// Create a white color
    pub fn white() -> Self {
        Self::rgb(255, 255, 255)
    }

    /// Create a transparent color
    pub fn transparent() -> Self {
        Self::rgba(0, 0, 0, 0)
    }

    /// Convert to peniko Color
    pub fn to_peniko(&self) -> peniko::Color {
        peniko::Color::from_rgba8(self.r, self.g, self.b, self.a)
    }

    /// Convert from normalized float values (0.0-1.0)
    pub fn from_floats(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: (r * 255.0) as u8,
            g: (g * 255.0) as u8,
            b: (b * 255.0) as u8,
            a: (a * 255.0) as u8,
        }
    }
}

impl Color {
    /// Build a color from OKLCH (perceptual lightness, chroma, hue-degrees),
    /// reducing chroma at constant lightness + hue when the request falls
    /// outside the sRGB gamut. Equal OKLCH chroma is a far better starting
    /// point for an even-intensity palette than equal HSL saturation or
    /// clamped RGB channels — this is the color engine the design themes and
    /// social palettes are built on.
    pub fn oklch(lightness: f64, chroma: f64, hue_degrees: f64) -> Self {
        fn linear(l: f64, c: f64, h_deg: f64) -> [f64; 3] {
            let h = h_deg.to_radians();
            let (a, b) = (c * h.cos(), c * h.sin());
            let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
            let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
            let s_ = l - 0.089_484_177_5 * a - 1.291_485_548 * b;
            let (l3, m3, s3) = (l_.powi(3), m_.powi(3), s_.powi(3));
            [
                4.076_741_662_1 * l3 - 3.307_711_591_3 * m3 + 0.230_969_929_2 * s3,
                -1.268_438_004_6 * l3 + 2.609_757_401_1 * m3 - 0.341_319_396_5 * s3,
                -0.004_196_086_3 * l3 - 0.703_418_614_7 * m3 + 1.707_614_701 * s3,
            ]
        }
        fn in_gamut(rgb: [f64; 3]) -> bool {
            rgb.iter().all(|c| (0.0..=1.0).contains(c))
        }
        fn encode(c: f64) -> u8 {
            let e = if c <= 0.003_130_8 {
                12.92 * c
            } else {
                1.055 * c.powf(1.0 / 2.4) - 0.055
            };
            (e.clamp(0.0, 1.0) * 255.0).round() as u8
        }
        let mut rgb = linear(lightness, chroma, hue_degrees);
        if !in_gamut(rgb) {
            let (mut lo, mut hi) = (0.0, chroma);
            for _ in 0..24 {
                let mid = (lo + hi) / 2.0;
                let cand = linear(lightness, mid, hue_degrees);
                if in_gamut(cand) {
                    lo = mid;
                    rgb = cand;
                } else {
                    hi = mid;
                }
            }
        }
        Color::rgb(encode(rgb[0]), encode(rgb[1]), encode(rgb[2]))
    }

    /// The same color with a new alpha (0-255).
    pub fn with_alpha(self, a: u8) -> Self {
        Color { a, ..self }
    }
}

/// Convenience function for creating RGB colors
pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::rgb(r, g, b)
}

/// Convenience function for creating RGBA colors
pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::rgba(r, g, b, a)
}

/// Convenience function for creating grayscale colors
pub fn gray(value: u8) -> Color {
    Color::gray(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb() {
        let color = rgb(255, 128, 64);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 64);
        assert_eq!(color.a, 255);
    }

    #[test]
    fn test_rgba() {
        let color = rgba(255, 128, 64, 32);
        assert_eq!(color.a, 32);
    }

    #[test]
    fn test_grayscale() {
        let color = gray(128);
        assert_eq!(color.r, 128);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 128);
    }
}
