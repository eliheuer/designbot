use thiserror::Error;

#[derive(Error, Debug)]
pub enum DesignBotError {
    #[error("Render error: {0}")]
    RenderError(String),

    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Font error: {0}")]
    FontError(String),

    #[error("Invalid color: {0}")]
    InvalidColor(String),

    #[error("Canvas error: {0}")]
    CanvasError(String),
}

pub type Result<T> = std::result::Result<T, DesignBotError>;
