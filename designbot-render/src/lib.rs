mod pdf;
mod renderer;
mod svg;

pub use pdf::PdfScenePainter;
pub use renderer::Renderer;
pub use svg::SvgScenePainter;

use designbot_core::DesignBotError;

pub type Result<T> = std::result::Result<T, DesignBotError>;
