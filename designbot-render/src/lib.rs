mod renderer;

pub use renderer::Renderer;

use designbot::DesignBotError;

pub type Result<T> = std::result::Result<T, DesignBotError>;
