// Re-export everything from designbot-core and designbot-render for convenience
pub use designbot_core::*;
pub use designbot_render::*;

// Single import prelude for beginners
pub mod prelude {
    pub use designbot_core::{Canvas, Color, TextAlign};
    pub use designbot_render::Renderer;
}
