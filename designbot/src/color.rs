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
        peniko::Color::rgba8(self.r, self.g, self.b, self.a)
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
