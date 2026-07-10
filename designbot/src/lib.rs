pub mod canvas;
mod color;
mod error;
pub mod motion;
mod state;

pub use canvas::{Canvas, TextAlign};
pub use color::Color;
pub use error::{DesignBotError, Result};

// Re-export kurbo so single-file `designbot --render` scripts can build
// BezPaths (glyph outlines) without declaring their own dependency.
pub use kurbo;

pub mod prelude {
    pub use crate::canvas::{Canvas, TextAlign};
    pub use crate::color::Color;
    pub use crate::error::{DesignBotError, Result};
    pub use crate::motion::*;
}
