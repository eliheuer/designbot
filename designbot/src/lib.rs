pub mod canvas;
mod color;
mod error;
mod state;

pub use canvas::Canvas;
pub use color::Color;
pub use error::{DesignBotError, Result};

pub mod prelude {
    pub use crate::canvas::Canvas;
    pub use crate::color::Color;
    pub use crate::error::{DesignBotError, Result};
}
