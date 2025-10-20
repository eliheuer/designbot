use designbot_core::{Canvas, DesignBotError};
use anyrender::{ImageRenderer, PaintScene, Paint};
use anyrender_vello_cpu::VelloCpuImageRenderer;
use kurbo::Shape;
use color::AlphaColor;
use peniko::Fill;
use parley::{FontContext, LayoutContext, layout::PositionedLayoutItem};
use parley::style::{FontFamily, FontStack, StyleProperty};
use std::borrow::Cow;
use std::cell::RefCell;
use std::path::Path;

pub struct Renderer {
    width: u32,
    height: u32,
    custom_fonts: Vec<Vec<u8>>,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            custom_fonts: Vec::new(),
        }
    }

    /// Load a font from a file path and register it for use
    ///
    /// # Example
    /// ```no_run
    /// use designbot::Renderer;
    /// let mut renderer = Renderer::new(800, 600);
    /// renderer.load_font("fonts/MyFont.ttf").unwrap();
    /// ```
    pub fn load_font(&mut self, path: impl AsRef<Path>) -> Result<(), DesignBotError> {
        let font_data = std::fs::read(path.as_ref())
            .map_err(|e| DesignBotError::IOError(e))?;
        self.custom_fonts.push(font_data);
        Ok(())
    }

    pub fn render_to_png(&self, canvas: &Canvas, output_path: &str) -> Result<(), DesignBotError> {
        // Create font context and register custom fonts
        let mut font_cx = FontContext::default();
        for font_data in &self.custom_fonts {
            font_cx.collection.register_fonts(font_data.clone());
        }

        // Wrap in RefCell for interior mutability within the closure
        let font_cx = RefCell::new(font_cx);

        // Create AnyRender ImageRenderer with vello_cpu backend
        let mut renderer = VelloCpuImageRenderer::new(self.width, self.height);

        // Render using AnyRender's clean callback API
        let mut rgba_data = Vec::new();
        renderer.render_to_vec(
            |painter| {
                // Draw background if set
                if let Some(bg_color) = canvas.background_color() {
                    let background = kurbo::Rect::new(
                        0.0,
                        0.0,
                        self.width as f64,
                        self.height as f64,
                    );
                    let rgba = bg_color.to_peniko().to_rgba8();
                    let bg_paint = Paint::Solid(AlphaColor::from_rgba8(rgba.r, rgba.g, rgba.b, rgba.a));
                    painter.fill(
                        Fill::NonZero,
                        kurbo::Affine::IDENTITY,
                        &bg_paint,
                        None,
                        &background,
                    );
                }

                // Draw all canvas commands
                for command in canvas.commands() {
                    Self::render_command(painter, command, &font_cx);
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
    fn render_command(
        painter: &mut impl PaintScene,
        command: &designbot_core::canvas::DrawCommand,
        font_cx: &RefCell<FontContext>,
    ) {
        use designbot_core::canvas::DrawCommand;

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
            DrawCommand::DrawText {
                text,
                x,
                y,
                font_family,
                font_size,
                brush,
                transform,
            } => {
                Self::render_text(painter, text, *x, *y, None, font_family.as_deref(), *font_size, brush, transform, font_cx);
            }
            DrawCommand::DrawTextBox {
                text,
                x,
                y,
                width,
                height,
                font_family,
                font_size,
                brush,
                transform,
            } => {
                Self::render_text(painter, text, *x, *y, Some((*width, *height)), font_family.as_deref(), *font_size, brush, transform, font_cx);
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

    /// Render text using Parley
    fn render_text(
        painter: &mut impl PaintScene,
        text: &str,
        x: f64,
        y: f64,
        bounds: Option<(f64, f64)>, // (width, height) for text_box
        font_family: Option<&str>,
        font_size: f64,
        brush: &peniko::Brush,
        transform: &kurbo::Affine,
        font_cx: &RefCell<FontContext>,
    ) {
        // Get mutable reference to font context
        let mut font_cx_ref = font_cx.borrow_mut();
        let mut layout_cx: LayoutContext<[u8; 4]> = LayoutContext::new();

        // Create a layout builder
        let mut builder = layout_cx.ranged_builder(&mut *font_cx_ref, text, 1.0);

        // Set font family if specified
        if let Some(family) = font_family {
            let font_family = FontFamily::Named(Cow::Borrowed(family));
            builder.push_default(StyleProperty::FontStack(FontStack::Single(font_family)));
        }

        // Set font size
        builder.push_default(StyleProperty::FontSize(font_size as f32));

        // Build the layout
        let mut layout = builder.build(text);

        // Set max width for text_box
        let max_width = bounds.map(|(w, _)| w as f32);
        layout.break_all_lines(max_width);
        layout.align(max_width, parley::layout::Alignment::Start);

        // Convert brush to color
        let color = Self::brush_to_color(brush);

        // Render each glyph
        for line in layout.lines() {
            // Get line metrics for proper vertical positioning
            let line_y = y + line.metrics().baseline as f64;

            for item in line.items() {
                if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                    let run = glyph_run.run();

                    // Track horizontal position as we iterate through glyphs
                    let mut glyph_x_offset = 0.0f32;

                    for glyph in glyph_run.glyphs() {

                        // Use swash to render the glyph outline
                        use swash::scale::ScaleContext;
                        use swash::zeno::Verb;

                        let mut scaler_ctx = ScaleContext::new();

                        // Get the swash font from the parley font
                        // In Parley 0.2, Font stores font data as bytes
                        let parley_font = run.font();
                        let font_data = parley_font.data.as_ref();
                        let swash_font_ref = swash::FontRef::from_index(font_data, parley_font.index as usize)
                            .expect("Failed to create swash FontRef");

                        let mut scaler = scaler_ctx.builder(swash_font_ref)
                            .size(font_size as f32)
                            .hint(true)
                            .build();

                        if let Some(outline) = scaler.scale_outline(glyph.id) {
                            let mut path = kurbo::BezPath::new();

                            // Convert swash path to kurbo path
                            let points = outline.points();
                            let mut point_idx = 0;

                            for verb in outline.verbs() {
                                match verb {
                                    Verb::MoveTo => {
                                        let p = points[point_idx];
                                        path.move_to(kurbo::Point::new(p.x as f64, p.y as f64));
                                        point_idx += 1;
                                    }
                                    Verb::LineTo => {
                                        let p = points[point_idx];
                                        path.line_to(kurbo::Point::new(p.x as f64, p.y as f64));
                                        point_idx += 1;
                                    }
                                    Verb::QuadTo => {
                                        let c = points[point_idx];
                                        let p = points[point_idx + 1];
                                        path.quad_to(
                                            kurbo::Point::new(c.x as f64, c.y as f64),
                                            kurbo::Point::new(p.x as f64, p.y as f64),
                                        );
                                        point_idx += 2;
                                    }
                                    Verb::CurveTo => {
                                        let c1 = points[point_idx];
                                        let c2 = points[point_idx + 1];
                                        let p = points[point_idx + 2];
                                        path.curve_to(
                                            kurbo::Point::new(c1.x as f64, c1.y as f64),
                                            kurbo::Point::new(c2.x as f64, c2.y as f64),
                                            kurbo::Point::new(p.x as f64, p.y as f64),
                                        );
                                        point_idx += 3;
                                    }
                                    Verb::Close => {
                                        path.close_path();
                                    }
                                }
                            }

                            // Calculate glyph position
                            // Use accumulated offset for X position
                            let glyph_x = x + glyph_x_offset as f64;
                            let glyph_y = line_y;

                            // Font glyphs have Y-up coordinate system, but screen is Y-down
                            // We need to flip Y and position the glyph
                            // Note: swash already scaled the outline to font_size
                            let glyph_transform = kurbo::Affine::translate((glyph_x, glyph_y))
                                * kurbo::Affine::FLIP_Y;
                            let final_transform = Self::convert_affine(transform) * glyph_transform;

                            // Fill the glyph
                            painter.fill(
                                Fill::NonZero,
                                final_transform,
                                &color,
                                None,
                                &path,
                            );
                        }

                        // Advance to next glyph position
                        glyph_x_offset += glyph.advance;
                    }
                }
            }
        }
    }
}
