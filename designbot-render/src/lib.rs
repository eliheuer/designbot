mod renderer;

pub use renderer::Renderer;

use designbot_core::DesignBotError;

pub type Result<T> = std::result::Result<T, DesignBotError>;
