// Both cores define a `Result` alias; the core one wins and that's fine.
#![allow(ambiguous_glob_reexports)]

// Re-export everything from designbot-core and designbot-render for convenience
pub use designbot_core::*;
pub use designbot_render::*;

// Re-export font, curve, and image libraries for use in designbot scripts.
pub use image;
pub use kurbo;
pub use norad;

// Single import prelude for beginners
pub mod prelude {
    pub use designbot_core::motion::*;
    pub use designbot_core::{find_up, Canvas, Color, Format, Grid, TextAlign, Theme};
    pub use designbot_render::Renderer;
    pub use kurbo::BezPath;
}
