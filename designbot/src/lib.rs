pub mod canvas;
mod color;
mod error;
pub mod format;
pub mod grid;
pub mod motion;
mod state;
pub mod theme;

pub use canvas::{Canvas, TextAlign};
pub use color::Color;
pub use error::{DesignBotError, Result};
pub use format::Format;
pub use grid::Grid;
pub use theme::Theme;

// Re-export kurbo so single-file `designbot --render` scripts can build
// BezPaths (glyph outlines) without declaring their own dependency.
pub use kurbo;

pub mod prelude {
    pub use crate::canvas::{Canvas, TextAlign};
    pub use crate::color::Color;
    pub use crate::error::{DesignBotError, Result};
    pub use crate::format::Format;
    pub use crate::grid::Grid;
    pub use crate::motion::*;
    pub use crate::theme::Theme;
}
