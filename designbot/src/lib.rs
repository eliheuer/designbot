pub mod canvas;
mod color;
mod error;
pub mod format;
pub mod grid;
pub mod motion;
mod state;
pub mod theme;

pub use canvas::{Canvas, TextAlign};
pub use color::{gray, rgb, rgba, Color};
pub use error::{DesignBotError, Result};
pub use format::Format;
pub use grid::Grid;
pub use theme::Theme;

// Re-export kurbo so single-file `designbot --render` scripts can build
// BezPaths (glyph outlines) without declaring their own dependency.
pub use kurbo;

use std::path::{Path, PathBuf};

/// Resolve a repo-relative path by walking up from the current directory until
/// it exists, so a script finds its fonts/assets whether it's run from the repo
/// root or from the subfolder the script lives in. Falls back to the path as
/// given (which then fails with a clear "not found" at load time).
///
/// ```no_run
/// # use designbot::prelude::*;
/// # let mut r = Renderer::new(1, 1);
/// r.load_font(find_up("fonts/ttf/VirtuaGrotesk-Regular.ttf")).unwrap();
/// ```
pub fn find_up(relative: impl AsRef<Path>) -> PathBuf {
    let rel = relative.as_ref();
    if let Ok(mut dir) = std::env::current_dir() {
        loop {
            let candidate = dir.join(rel);
            if candidate.exists() {
                return candidate;
            }
            if !dir.pop() {
                break;
            }
        }
    }
    rel.to_path_buf()
}

pub mod prelude {
    pub use crate::canvas::{Canvas, TextAlign};
    pub use crate::color::Color;
    pub use crate::error::{DesignBotError, Result};
    pub use crate::find_up;
    pub use crate::format::Format;
    pub use crate::grid::Grid;
    pub use crate::motion::*;
    pub use crate::theme::Theme;
}
