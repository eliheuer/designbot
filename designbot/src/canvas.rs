use crate::color::Color;
use crate::state::StateStack;
use kurbo::{Affine, BezPath, Circle, Ellipse, Line, Point, Rect, Stroke};
use peniko::Brush;

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
        brush: Brush,
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
        brush: Brush,
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

/// Main canvas for drawing
pub struct Canvas {
    width: f64,
    height: f64,
    background_color: Option<Color>,
    state: StateStack,
    commands: Vec<DrawCommand>,
}

impl Canvas {
    /// Create a new canvas with the given dimensions
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            background_color: Some(Color::white()), // Default white background
            state: StateStack::new(),
            commands: Vec::new(),
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

    /// Draw a rectangle
    pub fn rect(&mut self, x: f64, y: f64, width: f64, height: f64) -> &mut Self {
        let rect = Rect::new(x, y, x + width, y + height);
        self.draw_shape(ShapeType::Rect(rect));
        self
    }

    /// Draw an oval (ellipse)
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
                stroke: Stroke::new(state.stroke_width),
                transform: state.transform,
            });
        }

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
                stroke: Stroke::new(state.stroke_width),
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

    /// Draw text at a specific position
    pub fn text(&mut self, text: &str, x: f64, y: f64) -> &mut Self {
        let state = self.state.current();

        if let Some(fill_color) = state.fill_color {
            self.commands.push(DrawCommand::DrawText {
                text: text.to_string(),
                x,
                y,
                font_family: state.font_family.clone(),
                font_size: state.font_size,
                brush: Brush::Solid(fill_color.to_peniko()),
                transform: state.transform,
            });
        }

        self
    }

    /// Draw text within a bounding box with word wrapping
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
                brush: Brush::Solid(fill_color.to_peniko()),
                transform: state.transform,
            });
        }

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
}
