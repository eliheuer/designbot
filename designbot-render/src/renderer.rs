use designbot_core::{Canvas, DesignBotError};
use anyrender::{ImageRenderer, PaintScene, Paint};
use anyrender_vello_cpu::VelloCpuImageRenderer;
use kurbo::Shape;
use color::AlphaColor;
use peniko::Fill;

pub struct Renderer {
    width: u32,
    height: u32,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn render_to_png(&self, canvas: &Canvas, output_path: &str) -> Result<(), DesignBotError> {
        // Create AnyRender ImageRenderer with vello_cpu backend
        let mut renderer = VelloCpuImageRenderer::new(self.width, self.height);

        // Render using AnyRender's clean callback API
        let mut rgba_data = Vec::new();
        renderer.render_to_vec(
            |painter| {
                // Draw white background
                let background = kurbo::Rect::new(
                    0.0,
                    0.0,
                    self.width as f64,
                    self.height as f64,
                );
                let white = Paint::Solid(AlphaColor::from_rgba8(255, 255, 255, 255));
                painter.fill(
                    Fill::NonZero,
                    kurbo::Affine::IDENTITY,
                    &white,
                    None,
                    &background,
                );

                // Draw all canvas commands
                for command in canvas.commands() {
                    Self::render_command(painter, command);
                }
            },
            &mut rgba_data,
        );

        // Save to PNG using the image crate
        image::save_buffer(
            output_path,
            &rgba_data,
            self.width,
            self.height,
            image::ColorType::Rgba8,
        )
        .map_err(|e| DesignBotError::IOError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(())
    }

    /// Render a single draw command using the PaintScene API
    fn render_command(painter: &mut impl PaintScene, command: &designbot_core::canvas::DrawCommand) {
        use designbot_core::canvas::{DrawCommand, ShapeType};

        match command {
            DrawCommand::FillShape {
                shape,
                brush,
                transform,
            } => {
                // Convert the shape to kurbo 0.12 BezPath
                let kurbo_path = Self::convert_shape(shape);

                // Convert brush to a color that AnyRender understands
                let color = Self::brush_to_color(brush);

                // Use AnyRender's fill method
                painter.fill(
                    Fill::NonZero,
                    Self::convert_affine(transform),
                    &color,
                    None,
                    &kurbo_path,
                );
            }
            DrawCommand::StrokeShape {
                shape,
                brush,
                stroke,
                transform,
            } => {
                // Convert the shape to kurbo 0.12 BezPath
                let kurbo_path = Self::convert_shape(shape);

                // Convert brush to a color that AnyRender understands
                let color = Self::brush_to_color(brush);

                // Convert stroke to kurbo 0.12
                let kurbo_stroke = kurbo::Stroke::new(stroke.width);

                // Use AnyRender's stroke method
                painter.stroke(
                    &kurbo_stroke,
                    Self::convert_affine(transform),
                    &color,
                    None,
                    &kurbo_path,
                );
            }
        }
    }

    /// Convert peniko::Brush to Paint for AnyRender
    fn brush_to_color(brush: &peniko::Brush) -> Paint {
        match brush {
            peniko::Brush::Solid(color) => {
                // peniko::Color in 0.5 is already AlphaColor<Srgb>
                // We can use it directly or convert via rgba8
                let rgba = color.to_rgba8();
                let alpha_color = AlphaColor::from_rgba8(rgba.r, rgba.g, rgba.b, rgba.a);
                Paint::Solid(alpha_color)
            }
            // For now, just use black for gradients/images
            // TODO: Support gradients when needed
            _ => {
                Paint::Solid(AlphaColor::from_rgba8(0, 0, 0, 255))
            }
        }
    }

    /// Convert kurbo 0.11 Affine to kurbo 0.12 Affine
    fn convert_affine(affine: &kurbo::Affine) -> kurbo::Affine {
        let coeffs = affine.as_coeffs();
        kurbo::Affine::new(coeffs)
    }

    /// Convert our shape types (kurbo 0.11) to kurbo 0.12 BezPath
    fn convert_shape(shape: &designbot_core::canvas::ShapeType) -> kurbo::BezPath {
        use designbot_core::canvas::ShapeType;

        match shape {
            ShapeType::Rect(r) => {
                let rect = kurbo::Rect::new(r.x0, r.y0, r.x1, r.y1);
                rect.to_path(0.1)
            }
            ShapeType::Circle(c) => {
                let circle = kurbo::Circle::new(
                    kurbo::Point::new(c.center.x, c.center.y),
                    c.radius,
                );
                circle.to_path(0.1)
            }
            ShapeType::Ellipse(e) => {
                let ellipse = kurbo::Ellipse::new(
                    kurbo::Point::new(e.center().x, e.center().y),
                    kurbo::Vec2::new(e.radii().x, e.radii().y),
                    e.rotation(),
                );
                ellipse.to_path(0.1)
            }
            ShapeType::Line(l) => {
                let line = kurbo::Line::new(
                    kurbo::Point::new(l.p0.x, l.p0.y),
                    kurbo::Point::new(l.p1.x, l.p1.y),
                );
                line.to_path(0.1)
            }
            ShapeType::Path(p) => {
                // Convert path elements from kurbo 0.11 to 0.12
                let mut new_path = kurbo::BezPath::new();
                for el in p.elements() {
                    match el {
                        kurbo::PathEl::MoveTo(pt) => {
                            new_path.move_to(kurbo::Point::new(pt.x, pt.y));
                        }
                        kurbo::PathEl::LineTo(pt) => {
                            new_path.line_to(kurbo::Point::new(pt.x, pt.y));
                        }
                        kurbo::PathEl::QuadTo(p1, p2) => {
                            new_path.quad_to(
                                kurbo::Point::new(p1.x, p1.y),
                                kurbo::Point::new(p2.x, p2.y),
                            );
                        }
                        kurbo::PathEl::CurveTo(p1, p2, p3) => {
                            new_path.curve_to(
                                kurbo::Point::new(p1.x, p1.y),
                                kurbo::Point::new(p2.x, p2.y),
                                kurbo::Point::new(p3.x, p3.y),
                            );
                        }
                        kurbo::PathEl::ClosePath => {
                            new_path.close_path();
                        }
                    }
                }
                new_path
            }
        }
    }
}
