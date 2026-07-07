mod pdf;
mod renderer;

pub use pdf::PdfScenePainter;
pub use renderer::Renderer;

use designbot_core::DesignBotError;

pub type Result<T> = std::result::Result<T, DesignBotError>;
