use crate::color::Color;
use crate::state::StateStack;
use kurbo::{Affine, BezPath, Circle, Ellipse, Line, Point, Rect, Stroke};
use peniko::Brush;
use std::sync::Arc;

/// Pack a (up to) 4-character OpenType axis/feature tag into a big-endian u32,
/// space-padding shorter tags (matching swash's `tag_from_str_lossy`).
fn pack_tag(tag: &str) -> u32 {
    let mut bytes = [b' '; 4];
    for (slot, byte) in bytes.iter_mut().zip(tag.bytes()) {
        *slot = byte;
    }
    u32::from_be_bytes(bytes)
}

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlign {
    /// Left alignment (ignores text direction)
    Left,
    /// Center alignment (ignores text direction)
    Center,
    /// Right alignment (ignores text direction)
    Right,
    /// Start alignment (left for LTR, right for RTL)
    Start,
    /// End alignment (right for LTR, left for RTL)
    End,
    /// Justified alignment (except last line)
    Justified,
}

impl Default for TextAlign {
    fn default() -> Self {
        TextAlign::Left
    }
}

/// Drawing command that can be rendered
#[derive(Debug, Clone)]
pub enum DrawCommand {
    FillShape {
        shape: ShapeType,
        brush: Brush,
        transform: Affine,
    },
    StrokeShape {
        shape: ShapeType,
        brush: Brush,
        stroke: Stroke,
        transform: Affine,
    },
    DrawText {
        text: String,
        x: f64,
        y: f64,
        font_family: Option<String>,
        font_size: f64,
        line_height: Option<f64>,
        letter_spacing: f64,
        align: TextAlign,
        variations: Vec<(u32, f32)>,
        features: Vec<(u32, u16)>,
        brush: Brush,
        stroke_brush: Option<Brush>,
        stroke_width: f64,
        transform: Affine,
    },
    DrawTextBox {
        text: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        font_family: Option<String>,
        font_size: f64,
        line_height: Option<f64>,
        letter_spacing: f64,
        align: TextAlign,
        variations: Vec<(u32, f32)>,
        features: Vec<(u32, u16)>,
        brush: Brush,
        stroke_brush: Option<Brush>,
        stroke_width: f64,
        transform: Affine,
    },
    /// Draw a raster image (straight-alpha RGBA8) at its natural size, honoring
    /// the current transform. `x`/`y` offset the image within that transform;
    /// `alpha` is an extra opacity multiplier.
    DrawImage {
        data: Arc<Vec<u8>>,
        img_width: u32,
        img_height: u32,
        x: f64,
        y: f64,
        alpha: f32,
        transform: Affine,
    },
}

/// Supported shape types
#[derive(Debug, Clone)]
pub enum ShapeType {
    Rect(Rect),
    Circle(Circle),
    Ellipse(Ellipse),
    Line(Line),
    Path(BezPath),
}

/// A finished page (or animation frame): its command list, background, and
/// how long it stays on screen in an animation.
#[derive(Debug, Clone)]
pub struct Page {
    pub commands: Vec<DrawCommand>,
    pub background: Option<Color>,
    /// Frame duration in seconds (DrawBot `frameDuration`); None = default.
    pub duration: Option<f64>,
}

/// Default animation frame duration in seconds (DrawBot's 1/10 s).
pub const DEFAULT_FRAME_DURATION: f64 = 0.1;

/// Main canvas for drawing.
///
/// Coordinates are DrawBot's: the origin is the **bottom-left** corner and y
/// increases **upward**. This is implemented by seeding the graphics state
/// with a y-flip transform (`[1, 0, 0, -1, 0, height]`), so every command's
/// stored transform maps user space to the y-down device space the renderers
/// consume; user `translate`/`rotate`/`scale` calls compose on top of it,
/// which also makes positive `rotate()` counterclockwise, like DrawBot.
pub struct Canvas {
    width: f64,
    height: f64,
    background_color: Option<Color>,
    state: StateStack,
    commands: Vec<DrawCommand>,
    /// Pages completed by `new_page()`; the fields above hold the current page.
    finished_pages: Vec<Page>,
    /// Frame duration for the current (and subsequent) pages.
    frame_duration: Option<f64>,
}

impl Canvas {
    /// Create a new canvas with the given dimensions
    pub fn new(width: f64, height: f64) -> Self {
        let mut state = StateStack::new();
        // DrawBot user space (y-up, origin bottom-left) -> device space
        // (y-down, origin top-left). Lives at the bottom of the state stack,
        // which restore() never pops.
        state.current_mut().transform = Affine::new([1.0, 0.0, 0.0, -1.0, 0.0, height]);
        Self {
            width,
            height,
            background_color: Some(Color::white()), // Default white background
            state,
            commands: Vec::new(),
            finished_pages: Vec::new(),
            frame_duration: None,
        }
    }

    /// Finish the current page and start a new one (DrawBot `newPage`).
    /// Graphics state, background color, and frame duration carry over.
    pub fn new_page(&mut self) -> &mut Self {
        let commands = std::mem::take(&mut self.commands);
        self.finished_pages.push(Page {
            commands,
            background: self.background_color,
            duration: self.frame_duration,
        });
        self
    }

    /// Number of pages, counting the current one (DrawBot `pageCount`).
    pub fn page_count(&self) -> usize {
        self.finished_pages.len() + 1
    }

    /// Set the frame duration, in seconds, for the current and subsequent
    /// pages when exporting an animation (DrawBot `frameDuration`).
    pub fn frame_duration(&mut self, seconds: f64) -> &mut Self {
        self.frame_duration = Some(seconds);
        self
    }

    /// Pages completed by `new_page()` (not including the current page).
    pub fn finished_pages(&self) -> &[Page] {
        &self.finished_pages
    }

    /// The current (unfinished) page as a `Page` clone — combined with
    /// `finished_pages()` this is the whole document, in order.
    pub fn current_page(&self) -> Page {
        Page {
            commands: self.commands.clone(),
            background: self.background_color,
            duration: self.frame_duration,
        }
    }

    /// Get canvas width
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Get canvas height
    pub fn height(&self) -> f64 {
        self.height
    }

    /// Get all draw commands
    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    /// Get the background color
    pub fn background_color(&self) -> Option<Color> {
        self.background_color
    }

    /// Set the background color
    pub fn background(&mut self, color: Color) -> &mut Self {
        self.background_color = Some(color);
        self
    }

    /// Set the fill color
    pub fn fill(&mut self, color: Color) -> &mut Self {
        self.state.current_mut().fill_color = Some(color);
        self
    }

    /// Disable fill
    pub fn no_fill(&mut self) -> &mut Self {
        self.state.current_mut().fill_color = None;
        self
    }

    /// Set the stroke color
    pub fn stroke(&mut self, color: Color) -> &mut Self {
        self.state.current_mut().stroke_color = Some(color);
        self
    }

    /// Disable stroke
    pub fn no_stroke(&mut self) -> &mut Self {
        self.state.current_mut().stroke_color = None;
        self
    }

    /// Set stroke width
    pub fn stroke_width(&mut self, width: f64) -> &mut Self {
        self.state.current_mut().stroke_width = width;
        self
    }

    /// Set the stroke end-cap style: `"butt"` (default), `"round"`, or
    /// `"square"` (DrawBot `lineCap`).
    pub fn line_cap(&mut self, cap: &str) -> &mut Self {
        self.state.current_mut().line_cap = match cap {
            "round" => kurbo::Cap::Round,
            "square" => kurbo::Cap::Square,
            _ => kurbo::Cap::Butt,
        };
        self
    }

    /// Set the stroke join style: `"miter"` (default), `"round"`, or
    /// `"bevel"` (DrawBot `lineJoin`).
    pub fn line_join(&mut self, join: &str) -> &mut Self {
        self.state.current_mut().line_join = match join {
            "round" => kurbo::Join::Round,
            "bevel" => kurbo::Join::Bevel,
            _ => kurbo::Join::Miter,
        };
        self
    }

    /// Set the miter limit (DrawBot `miterLimit`).
    pub fn miter_limit(&mut self, limit: f64) -> &mut Self {
        self.state.current_mut().miter_limit = limit;
        self
    }

    /// Set a dash pattern as on/off lengths, e.g. `&[8.0, 4.0]`; an empty
    /// slice restores a solid line (DrawBot `lineDash`).
    pub fn line_dash(&mut self, pattern: &[f64]) -> &mut Self {
        self.state.current_mut().dash_pattern = pattern.to_vec();
        self
    }

    /// Draw a rectangle anchored at its bottom-left corner (DrawBot `rect`).
    pub fn rect(&mut self, x: f64, y: f64, width: f64, height: f64) -> &mut Self {
        let rect = Rect::new(x, y, x + width, y + height);
        self.draw_shape(ShapeType::Rect(rect));
        self
    }

    /// Draw an oval (ellipse) inside the rect anchored at its bottom-left
    /// corner (DrawBot `oval`).
    pub fn oval(&mut self, x: f64, y: f64, width: f64, height: f64) -> &mut Self {
        // Calculate center and radii
        let center_x = x + width / 2.0;
        let center_y = y + height / 2.0;
        let radius_x = width / 2.0;
        let radius_y = height / 2.0;

        if (width - height).abs() < 0.001 {
            // It's a circle - use Circle for better performance
            let circle = Circle::new((center_x, center_y), width / 2.0);
            self.draw_shape(ShapeType::Circle(circle));
        } else {
            // Use Kurbo's native Ellipse type
            // Ellipse::new(center, radii, x_rotation)
            let ellipse = Ellipse::new((center_x, center_y), (radius_x, radius_y), 0.0);
            self.draw_shape(ShapeType::Ellipse(ellipse));
        }

        self
    }

    /// Draw a line
    pub fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) -> &mut Self {
        let line = Line::new((x1, y1), (x2, y2));
        // Lines are always stroked, never filled
        let state = self.state.current();

        if let Some(stroke_color) = state.stroke_color {
            self.commands.push(DrawCommand::StrokeShape {
                shape: ShapeType::Line(line),
                brush: Brush::Solid(stroke_color.to_peniko()),
                stroke: state.make_stroke(),
                transform: state.transform,
            });
        }

        self
    }

    /// Draw a bezier path
    pub fn draw_path(&mut self, path: BezPath) -> &mut Self {
        self.draw_shape(ShapeType::Path(path));
        self
    }

    /// Draw a polygon
    pub fn polygon(&mut self, points: &[(f64, f64)], close: bool) -> &mut Self {
        if points.is_empty() {
            return self;
        }

        let mut path = BezPath::new();
        path.move_to(Point::new(points[0].0, points[0].1));

        for point in points.iter().skip(1) {
            path.line_to(Point::new(point.0, point.1));
        }

        if close {
            path.close_path();
        }

        self.draw_shape(ShapeType::Path(path));
        self
    }

    /// Internal method to draw a shape with current state
    fn draw_shape(&mut self, shape: ShapeType) {
        let state = self.state.current();

        // Add fill command if fill color is set
        if let Some(fill_color) = state.fill_color {
            self.commands.push(DrawCommand::FillShape {
                shape: shape.clone(),
                brush: Brush::Solid(fill_color.to_peniko()),
                transform: state.transform,
            });
        }

        // Add stroke command if stroke color is set
        if let Some(stroke_color) = state.stroke_color {
            self.commands.push(DrawCommand::StrokeShape {
                shape,
                brush: Brush::Solid(stroke_color.to_peniko()),
                stroke: state.make_stroke(),
                transform: state.transform,
            });
        }
    }

    /// Save the current graphics state
    pub fn save(&mut self) -> &mut Self {
        self.state.save();
        self
    }

    /// Restore the previous graphics state
    pub fn restore(&mut self) -> &mut Self {
        self.state.restore();
        self
    }

    /// Translate the coordinate system
    pub fn translate(&mut self, x: f64, y: f64) -> &mut Self {
        let current_transform = self.state.current().transform;
        self.state.current_mut().transform = current_transform * Affine::translate((x, y));
        self
    }

    /// Rotate the coordinate system (in degrees)
    pub fn rotate(&mut self, degrees: f64) -> &mut Self {
        let radians = degrees.to_radians();
        let current_transform = self.state.current().transform;
        self.state.current_mut().transform = current_transform * Affine::rotate(radians);
        self
    }

    /// Scale the coordinate system
    pub fn scale(&mut self, factor: f64) -> &mut Self {
        let current_transform = self.state.current().transform;
        self.state.current_mut().transform = current_transform * Affine::scale(factor);
        self
    }

    /// Scale with different factors for x and y
    pub fn scale_xy(&mut self, sx: f64, sy: f64) -> &mut Self {
        let current_transform = self.state.current().transform;
        self.state.current_mut().transform =
            current_transform * Affine::scale_non_uniform(sx, sy);
        self
    }

    /// Set the font family
    pub fn font(&mut self, family: &str) -> &mut Self {
        self.state.current_mut().font_family = Some(family.to_string());
        self
    }

    /// Set the font size
    pub fn font_size(&mut self, size: f64) -> &mut Self {
        self.state.current_mut().font_size = size;
        self
    }

    /// Set the line height (baseline-to-baseline) in points, like DrawBot
    /// `lineHeight`. Applies to subsequent `text`/`text_box`. Set it to a
    /// powers-of-two value to keep multi-line baselines on the grid.
    pub fn line_height(&mut self, value: f64) -> &mut Self {
        self.state.current_mut().line_height = Some(value);
        self
    }

    /// Reset the line height to the font's natural metrics.
    pub fn auto_line_height(&mut self) -> &mut Self {
        self.state.current_mut().line_height = None;
        self
    }

    /// Set letter spacing / tracking in points, like DrawBot `tracking`.
    pub fn tracking(&mut self, value: f64) -> &mut Self {
        self.state.current_mut().letter_spacing = value;
        self
    }

    /// Set the text alignment
    pub fn text_align(&mut self, align: TextAlign) -> &mut Self {
        self.state.current_mut().text_align = align;
        self
    }

    /// Draw text with the first line's **baseline** at (x, y), like DrawBot's
    /// `text()`. Subsequent lines of a multi-line string stack downward.
    pub fn text(&mut self, text: &str, x: f64, y: f64) -> &mut Self {
        let state = self.state.current();

        if let Some(fill_color) = state.fill_color {
            self.commands.push(DrawCommand::DrawText {
                text: text.to_string(),
                x,
                y,
                font_family: state.font_family.clone(),
                font_size: state.font_size,
                line_height: state.line_height,
                letter_spacing: state.letter_spacing,
                align: state.text_align,
                variations: state.font_variations.clone(),
                features: state.font_features.clone(),
                brush: Brush::Solid(fill_color.to_peniko()),
                stroke_brush: state.stroke_color.map(|c| Brush::Solid(c.to_peniko())),
                stroke_width: state.stroke_width,
                transform: state.transform,
            });
        }

        self
    }

    /// Draw text with word wrapping inside a box anchored at its
    /// bottom-left corner; text fills from the top of the box down
    /// (DrawBot `textBox`).
    pub fn text_box(&mut self, text: &str, x: f64, y: f64, width: f64, height: f64) -> &mut Self {
        let state = self.state.current();

        if let Some(fill_color) = state.fill_color {
            self.commands.push(DrawCommand::DrawTextBox {
                text: text.to_string(),
                x,
                y,
                width,
                height,
                font_family: state.font_family.clone(),
                font_size: state.font_size,
                line_height: state.line_height,
                letter_spacing: state.letter_spacing,
                align: state.text_align,
                variations: state.font_variations.clone(),
                features: state.font_features.clone(),
                brush: Brush::Solid(fill_color.to_peniko()),
                stroke_brush: state.stroke_color.map(|c| Brush::Solid(c.to_peniko())),
                stroke_width: state.stroke_width,
                transform: state.transform,
            });
        }

        self
    }

    /// Set a variable-font axis (e.g. `"wght"`, `700.0`). Repeated calls set
    /// additional axes; a later call for the same axis overrides the earlier
    /// value. Applies to subsequent `text`/`text_box` calls.
    pub fn font_variation(&mut self, axis: &str, value: f32) -> &mut Self {
        let tag = pack_tag(axis);
        let variations = &mut self.state.current_mut().font_variations;
        if let Some(existing) = variations.iter_mut().find(|(t, _)| *t == tag) {
            existing.1 = value;
        } else {
            variations.push((tag, value));
        }
        self
    }

    /// Clear all variable-font axis settings.
    pub fn clear_font_variations(&mut self) -> &mut Self {
        self.state.current_mut().font_variations.clear();
        self
    }

    /// Set an OpenType feature (e.g. `"kern"`, `0` to disable; `"tnum"`, `1` to
    /// enable). Repeated calls set additional features; a later call for the
    /// same tag overrides the earlier value. Applies to subsequent
    /// `text`/`text_box` calls.
    pub fn font_feature(&mut self, tag: &str, value: u16) -> &mut Self {
        let tag = pack_tag(tag);
        let features = &mut self.state.current_mut().font_features;
        if let Some(existing) = features.iter_mut().find(|(t, _)| *t == tag) {
            existing.1 = value;
        } else {
            features.push((tag, value));
        }
        self
    }

    /// Clear all OpenType feature settings.
    pub fn clear_font_features(&mut self) -> &mut Self {
        self.state.current_mut().font_features.clear();
        self
    }

    /// Draw a raster image from straight-alpha RGBA8 pixels at its natural
    /// size, anchored at its bottom-left corner (DrawBot `image`) and honoring
    /// the current transform. `data` must be `img_width * img_height * 4`
    /// bytes; `alpha` is an extra opacity multiplier in `[0, 1]`.
    pub fn image_rgba(
        &mut self,
        data: Vec<u8>,
        img_width: u32,
        img_height: u32,
        x: f64,
        y: f64,
        alpha: f32,
    ) -> &mut Self {
        let transform = self.state.current().transform;
        self.commands.push(DrawCommand::DrawImage {
            data: Arc::new(data),
            img_width,
            img_height,
            x,
            y,
            alpha,
            transform,
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_creation() {
        let canvas = Canvas::new(800.0, 600.0);
        assert_eq!(canvas.width(), 800.0);
        assert_eq!(canvas.height(), 600.0);
    }

    #[test]
    fn test_rect_command() {
        let mut canvas = Canvas::new(800.0, 600.0);
        canvas.fill(Color::rgb(255, 0, 0)).rect(10.0, 10.0, 100.0, 100.0);

        assert_eq!(canvas.commands().len(), 1);
    }

    #[test]
    fn test_state_stack() {
        let mut canvas = Canvas::new(800.0, 600.0);

        canvas.fill(Color::rgb(255, 0, 0));
        canvas.save();
        canvas.fill(Color::rgb(0, 255, 0));
        canvas.restore();

        // After restore, should be back to red
        assert_eq!(
            canvas.state.current().fill_color,
            Some(Color::rgb(255, 0, 0))
        );
    }

    #[test]
    fn test_pages() {
        let mut canvas = Canvas::new(100.0, 100.0);
        assert_eq!(canvas.page_count(), 1);

        canvas.rect(0.0, 0.0, 10.0, 10.0);
        canvas.frame_duration(0.05);
        canvas.new_page();

        // Current page is fresh; the finished page kept its commands.
        assert_eq!(canvas.page_count(), 2);
        assert!(canvas.commands().is_empty());
        assert_eq!(canvas.finished_pages().len(), 1);
        assert_eq!(canvas.finished_pages()[0].commands.len(), 1);
        assert_eq!(canvas.finished_pages()[0].duration, Some(0.05));

        // Frame duration persists onto the next page.
        canvas.rect(0.0, 0.0, 10.0, 10.0);
        assert_eq!(canvas.current_page().duration, Some(0.05));
        assert_eq!(canvas.current_page().commands.len(), 1);
    }

    #[test]
    fn test_stroke_style_state() {
        let mut canvas = Canvas::new(100.0, 100.0);
        canvas
            .stroke(Color::black())
            .stroke_width(4.0)
            .line_cap("round")
            .line_join("bevel")
            .line_dash(&[8.0, 4.0]);
        let stroke = canvas.state.current().make_stroke();
        assert_eq!(stroke.width, 4.0);
        assert_eq!(stroke.start_cap, kurbo::Cap::Round);
        assert_eq!(stroke.join, kurbo::Join::Bevel);
        assert_eq!(stroke.dash_pattern.as_slice(), &[8.0, 4.0]);
    }
}
