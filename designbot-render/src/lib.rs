mod pdf;
mod proof;
mod renderer;
mod svg;

pub use pdf::PdfScenePainter;
pub use proof::generate_proof;
pub use renderer::Renderer;
pub use svg::SvgScenePainter;

use designbot_core::DesignBotError;

pub type Result<T> = std::result::Result<T, DesignBotError>;
