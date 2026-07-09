use designbot_core::{Canvas, DesignBotError};
use anyrender::{ImageRenderer, PaintScene, Paint};
use anyrender_vello_cpu::VelloCpuImageRenderer;
use kurbo::Shape;
use color::AlphaColor;
use peniko::Fill;
use parley::{FontContext, LayoutContext, layout::PositionedLayoutItem};
use parley::style::{FontFamily, FontSettings, FontStack, FontVariation, StyleProperty};
use std::borrow::Cow;
use std::cell::RefCell;
use std::path::Path;
use std::sync::Arc;

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
    /// use designbot_render::Renderer;
    /// let mut renderer = Renderer::new(800, 600);
    /// renderer.load_font("fonts/MyFont.ttf").unwrap();
    /// ```
    pub fn load_font(&mut self, path: impl AsRef<Path>) -> Result<(), DesignBotError> {
        let font_data = std::fs::read(path.as_ref())
            .map_err(|e| DesignBotError::IOError(e))?;
        self.custom_fonts.push(font_data);
        Ok(())
    }

    /// Measure the advance width, in pixels, of a single line of text with the
    /// given font/size/axes — the equivalent of DrawBot's `textSize()[0]`. Uses
    /// the same shaping path as rendering, including any registered custom fonts.
    pub fn text_width(
        &self,
        text: &str,
        font_family: Option<&str>,
        font_size: f64,
        variations: &[(u32, f32)],
    ) -> f64 {
        let mut font_cx = FontContext::default();
        for font_data in &self.custom_fonts {
            font_cx.collection.register_fonts(font_data.clone());
        }
        let mut layout_cx: LayoutContext<[u8; 4]> = LayoutContext::new();
        let mut builder = layout_cx.ranged_builder(&mut font_cx, text, 1.0);

        if let Some(family) = font_family {
            let family = FontFamily::Named(Cow::Borrowed(family));
            builder.push_default(StyleProperty::FontStack(FontStack::Single(family)));
        }
        builder.push_default(StyleProperty::FontSize(font_size as f32));
        if !variations.is_empty() {
            let settings: Vec<FontVariation> = variations
                .iter()
                .map(|(tag, value)| FontVariation {
                    tag: *tag,
                    value: *value,
                })
                .collect();
            builder.push_default(StyleProperty::FontVariations(FontSettings::List(
                Cow::Owned(settings),
            )));
        }

        let mut layout = builder.build(text);
        layout.break_all_lines(None);
        layout.width() as f64
    }

    /// Render every page of the canvas (finished pages + the current one) to
    /// raw straight-alpha RGBA8 buffers, reusing one renderer and one font
    /// context across frames. Returns `(pixels, duration_seconds)` per page.
    pub fn render_frames(&self, canvas: &Canvas) -> Vec<(Vec<u8>, f64)> {
        // Create font context once and register custom fonts
        let mut font_cx = FontContext::default();
        for font_data in &self.custom_fonts {
            font_cx.collection.register_fonts(font_data.clone());
        }
        let font_cx = RefCell::new(font_cx);

        let mut renderer = VelloCpuImageRenderer::new(self.width, self.height);

        let mut pages: Vec<designbot_core::canvas::Page> = canvas.finished_pages().to_vec();
        pages.push(canvas.current_page());

        let mut frames = Vec::with_capacity(pages.len());
        for (i, page) in pages.iter().enumerate() {
            if i > 0 {
                // render_to_vec does NOT reset the scene between calls.
                renderer.reset();
            }
            let mut rgba_data = Vec::new();
            renderer.render_to_vec(
                |painter| {
                    if let Some(bg_color) = page.background {
                        let background = kurbo::Rect::new(
                            0.0,
                            0.0,
                            self.width as f64,
                            self.height as f64,
                        );
                        let rgba = bg_color.to_peniko().to_rgba8();
                        let bg_paint =
                            Paint::Solid(AlphaColor::from_rgba8(rgba.r, rgba.g, rgba.b, rgba.a));
                        painter.fill(
                            Fill::NonZero,
                            kurbo::Affine::IDENTITY,
                            &bg_paint,
                            None,
                            &background,
                        );
                    }
                    for command in &page.commands {
                        Self::render_command(painter, command, &font_cx);
                    }
                },
                &mut rgba_data,
            );
            let duration = page
                .duration
                .unwrap_or(designbot_core::canvas::DEFAULT_FRAME_DURATION);
            frames.push((rgba_data, duration));
        }
        frames
    }

    /// Render to PNG. A single-page canvas writes exactly `output_path`; a
    /// multi-page canvas writes numbered siblings (`name_0001.png`, ...).
    pub fn render_to_png(&self, canvas: &Canvas, output_path: &str) -> Result<(), DesignBotError> {
        let frames = self.render_frames(canvas);
        let save = |path: &str, data: &[u8]| {
            image::save_buffer(path, data, self.width, self.height, image::ColorType::Rgba8)
                .map_err(|e| {
                    DesignBotError::IOError(std::io::Error::new(std::io::ErrorKind::Other, e))
                })
        };
        if frames.len() == 1 {
            save(output_path, &frames[0].0)?;
        } else {
            let path = Path::new(output_path);
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let parent = path.parent().unwrap_or_else(|| Path::new(""));
            for (i, (data, _)) in frames.iter().enumerate() {
                let numbered = parent.join(format!("{}_{:04}.png", stem, i + 1));
                save(&numbered.to_string_lossy(), data)?;
            }
        }
        Ok(())
    }

    /// Render to a PNG optimized for posting on social media. Two changes
    /// versus `render_to_png`: the file carries an explicit sRGB chunk
    /// (platforms strip ICC profiles and assume sRGB, so tagging removes the
    /// ambiguity that washes out colors), and the top-left pixel is knocked
    /// to 99% alpha, which makes X/Twitter keep the upload as lossless PNG
    /// instead of re-encoding it to JPEG and smearing fine linework.
    /// Single-image export: a multi-page canvas emits its last page with a
    /// warning, like SVG.
    pub fn render_to_png_social(
        &self,
        canvas: &Canvas,
        output_path: &str,
    ) -> Result<(), DesignBotError> {
        let mut frames = self.render_frames(canvas);
        if frames.len() > 1 {
            eprintln!("warning: social png is single-image; emitting the last page only");
        }
        let (mut data, _) = frames
            .pop()
            .ok_or_else(|| DesignBotError::RenderError("no frames to render".into()))?;
        if data.len() >= 4 {
            data[3] = 253; // ~99% alpha on one pixel: forces PNG passthrough
        }
        let file = std::fs::File::create(output_path).map_err(DesignBotError::IOError)?;
        let mut encoder =
            png::Encoder::new(std::io::BufWriter::new(file), self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_srgb(png::SrgbRenderingIntent::Perceptual);
        let mut writer = encoder
            .write_header()
            .map_err(|e| DesignBotError::RenderError(format!("png encode: {e}")))?;
        writer
            .write_image_data(&data)
            .map_err(|e| DesignBotError::RenderError(format!("png encode: {e}")))?;
        Ok(())
    }

    /// Render all pages to an animated GIF, honoring per-page frame durations
    /// (DrawBot: `newPage` + `frameDuration` + `saveImage("*.gif")`).
    pub fn render_to_gif(&self, canvas: &Canvas, output_path: &str) -> Result<(), DesignBotError> {
        use image::codecs::gif::{GifEncoder, Repeat};
        use image::{Delay, Frame, RgbaImage};

        let frames = self.render_frames(canvas);
        let file = std::fs::File::create(output_path).map_err(DesignBotError::IOError)?;
        let mut encoder = GifEncoder::new(file);
        encoder
            .set_repeat(Repeat::Infinite)
            .map_err(|e| DesignBotError::RenderError(format!("gif encoder: {e}")))?;
        for (data, duration) in frames {
            let img = RgbaImage::from_raw(self.width, self.height, data)
                .ok_or_else(|| DesignBotError::RenderError("frame buffer size mismatch".into()))?;
            let delay =
                Delay::from_saturating_duration(std::time::Duration::from_secs_f64(duration));
            encoder
                .encode_frame(Frame::from_parts(img, 0, 0, delay))
                .map_err(|e| DesignBotError::RenderError(format!("gif frame: {e}")))?;
        }
        Ok(())
    }

    /// Render all pages to an MP4 by piping raw frames to `ffmpeg` (the same
    /// approach DrawBot takes). Uses a constant frame rate derived from the
    /// shortest page duration; longer pages repeat frames. Requires `ffmpeg`
    /// on PATH.
    pub fn render_to_mp4(&self, canvas: &Canvas, output_path: &str) -> Result<(), DesignBotError> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let frames = self.render_frames(canvas);
        let shortest = frames
            .iter()
            .map(|(_, d)| *d)
            .fold(f64::INFINITY, f64::min)
            .max(1.0 / 60.0);
        let fps = (1.0 / shortest).round().clamp(1.0, 60.0);

        let mut child = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "rawvideo",
                "-pixel_format",
                "rgba",
                "-video_size",
                &format!("{}x{}", self.width, self.height),
                "-framerate",
                &format!("{fps}"),
                "-i",
                "-",
                "-pix_fmt",
                "yuv420p",
                // yuv420p needs even dimensions
                "-vf",
                "pad=ceil(iw/2)*2:ceil(ih/2)*2",
                "-movflags",
                "+faststart",
                "-an",
                output_path,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                DesignBotError::RenderError(format!(
                    "could not launch ffmpeg (required for MP4 export — `brew install ffmpeg`): {e}"
                ))
            })?;

        {
            let stdin = child
                .stdin
                .as_mut()
                .ok_or_else(|| DesignBotError::RenderError("ffmpeg stdin unavailable".into()))?;
            for (data, duration) in &frames {
                let repeats = ((duration * fps).round() as usize).max(1);
                for _ in 0..repeats {
                    stdin.write_all(data).map_err(DesignBotError::IOError)?;
                }
            }
        }

        let output = child.wait_with_output().map_err(DesignBotError::IOError)?;
        if !output.status.success() {
            return Err(DesignBotError::RenderError(format!(
                "ffmpeg failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(())
    }

    /// Render all pages to a single **vector** PDF — shapes and text (as
    /// outline paths) stay resolution-independent; one PDF page per canvas
    /// page (DrawBot's multi-page `saveImage("*.pdf")` document model).
    pub fn render_to_pdf(&self, canvas: &Canvas, output_path: &str) -> Result<(), DesignBotError> {
        use pdf_writer::{Pdf, Rect as PdfRect, Ref};

        let mut font_cx = FontContext::default();
        for font_data in &self.custom_fonts {
            font_cx.collection.register_fonts(font_data.clone());
        }
        let font_cx = RefCell::new(font_cx);

        let mut pages: Vec<designbot_core::canvas::Page> = canvas.finished_pages().to_vec();
        pages.push(canvas.current_page());

        let catalog_id = Ref::new(1);
        let pages_id = Ref::new(2);
        let mut next_id = 3;
        let mut rendered: Vec<(Ref, Ref, Vec<u8>)> = Vec::new();
        let mut all_warnings: Vec<&'static str> = Vec::new();

        for page in &pages {
            let page_id = Ref::new(next_id);
            let content_id = Ref::new(next_id + 1);
            next_id += 2;

            let mut painter = crate::pdf::PdfScenePainter::new(self.height as f64);
            if let Some(bg) = page.background {
                let rgba = bg.to_peniko().to_rgba8();
                let paint = Paint::Solid(AlphaColor::from_rgba8(rgba.r, rgba.g, rgba.b, rgba.a));
                let full =
                    kurbo::Rect::new(0.0, 0.0, self.width as f64, self.height as f64);
                painter.fill(Fill::NonZero, kurbo::Affine::IDENTITY, &paint, None, &full);
            }
            for command in &page.commands {
                Self::render_command(&mut painter, command, &font_cx);
            }
            let (bytes, warnings) = painter.finish();
            for w in warnings {
                if !all_warnings.contains(&w) {
                    all_warnings.push(w);
                }
            }
            rendered.push((page_id, content_id, bytes));
        }

        let mut pdf = Pdf::new();
        pdf.catalog(catalog_id).pages(pages_id);
        pdf.pages(pages_id)
            .kids(rendered.iter().map(|(page_id, _, _)| *page_id))
            .count(rendered.len() as i32);
        for (page_id, content_id, bytes) in &rendered {
            {
                let mut page = pdf.page(*page_id);
                page.parent(pages_id)
                    .media_box(PdfRect::new(0.0, 0.0, self.width as f32, self.height as f32))
                    .contents(*content_id);
                page.resources();
            }
            // Outline-heavy content streams compress ~10x; PDF FlateDecode
            // is the zlib format.
            let compressed = {
                use std::io::Write;
                let mut enc = flate2::write::ZlibEncoder::new(
                    Vec::new(),
                    flate2::Compression::default(),
                );
                enc.write_all(bytes).map_err(DesignBotError::IOError)?;
                enc.finish().map_err(DesignBotError::IOError)?
            };
            pdf.stream(*content_id, &compressed)
                .filter(pdf_writer::Filter::FlateDecode);
        }
        for w in all_warnings {
            eprintln!("warning: {w}");
        }
        std::fs::write(output_path, pdf.finish()).map_err(DesignBotError::IOError)?;
        Ok(())
    }

    /// Render the canvas to a single-page SVG document. SVG has no multi-page
    /// concept, so an animated (multi-page) canvas emits only its last page
    /// with a warning; a static canvas emits its one page as true vectors.
    pub fn render_to_svg(
        &self,
        canvas: &Canvas,
        output_path: &str,
    ) -> Result<(), DesignBotError> {
        let mut font_cx = FontContext::default();
        for font_data in &self.custom_fonts {
            font_cx.collection.register_fonts(font_data.clone());
        }
        let font_cx = RefCell::new(font_cx);

        if !canvas.finished_pages().is_empty() {
            eprintln!(
                "warning: svg is single-page; emitting the last page only"
            );
        }
        let page = canvas.current_page();

        let mut painter =
            crate::svg::SvgScenePainter::new(self.width as f64, self.height as f64);
        if let Some(bg) = page.background {
            let rgba = bg.to_peniko().to_rgba8();
            let paint =
                Paint::Solid(AlphaColor::from_rgba8(rgba.r, rgba.g, rgba.b, rgba.a));
            let full = kurbo::Rect::new(0.0, 0.0, self.width as f64, self.height as f64);
            painter.fill(Fill::NonZero, kurbo::Affine::IDENTITY, &paint, None, &full);
        }
        for command in &page.commands {
            Self::render_command(&mut painter, command, &font_cx);
        }
        let (bytes, warnings) = painter.finish();
        for w in warnings {
            eprintln!("warning: {w}");
        }
        std::fs::write(output_path, bytes).map_err(DesignBotError::IOError)?;
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

                // Pass the full stroke through: caps, join, miter, dashes
                let kurbo_stroke = stroke.clone();

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
                align,
                variations,
                brush,
                transform,
            } => {
                Self::render_text(painter, text, *x, *y, None, font_family.as_deref(), *font_size, *align, variations, brush, transform, font_cx);
            }
            DrawCommand::DrawTextBox {
                text,
                x,
                y,
                width,
                height,
                font_family,
                font_size,
                align,
                variations,
                brush,
                transform,
            } => {
                Self::render_text(painter, text, *x, *y, Some((*width, *height)), font_family.as_deref(), *font_size, *align, variations, brush, transform, font_cx);
            }
            DrawCommand::DrawImage {
                data,
                img_width,
                img_height,
                x,
                y,
                alpha,
                transform,
            } => {
                Self::render_image(painter, data, *img_width, *img_height, *x, *y, *alpha, transform);
            }
        }
    }

    /// Draw a raster image (straight-alpha RGBA8) at its natural size under the
    /// given transform, offset by (x, y), with an extra alpha multiplier.
    fn render_image(
        painter: &mut impl PaintScene,
        data: &Arc<Vec<u8>>,
        img_width: u32,
        img_height: u32,
        x: f64,
        y: f64,
        alpha: f32,
        transform: &kurbo::Affine,
    ) {
        use peniko::{Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat};

        // vello_cpu 0.0.4 does not implement opacity on image draws, so bake the
        // alpha multiplier into the image's own alpha channel (this matches
        // DrawBot's `image(..., alpha=)` semantics) and keep the brush opaque.
        let blob = if alpha >= 1.0 {
            Blob::new(data.clone())
        } else {
            let mut faded = (**data).clone();
            for pixel in faded.chunks_exact_mut(4) {
                pixel[3] = (pixel[3] as f32 * alpha).round().clamp(0.0, 255.0) as u8;
            }
            Blob::new(Arc::new(faded))
        };

        let image_data = ImageData {
            data: blob,
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::Alpha,
            width: img_width,
            height: img_height,
        };
        let brush = ImageBrush::new(image_data);

        // Anchor the image's bottom-left corner at (x, y) in DrawBot user
        // space. The local FLIP_Y pairs with the flip already inside the
        // command transform, so raster rows keep their orientation.
        let placement = Self::convert_affine(transform)
            * kurbo::Affine::translate((x, y + img_height as f64))
            * kurbo::Affine::FLIP_Y;
        let bounds = kurbo::Rect::new(0.0, 0.0, img_width as f64, img_height as f64);
        painter.fill(Fill::NonZero, placement, brush.as_ref(), None, &bounds);
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
        align: designbot_core::canvas::TextAlign,
        variations: &[(u32, f32)],
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

        // Apply variable-font axis settings, if any.
        if !variations.is_empty() {
            let settings: Vec<FontVariation> = variations
                .iter()
                .map(|(tag, value)| FontVariation {
                    tag: *tag,
                    value: *value,
                })
                .collect();
            builder.push_default(StyleProperty::FontVariations(FontSettings::List(
                Cow::Owned(settings),
            )));
        }

        // Build the layout
        let mut layout = builder.build(text);

        // Set max width for text_box
        let max_width = bounds.map(|(w, _)| w as f32);
        layout.break_all_lines(max_width);

        // Convert our TextAlign to Parley's Alignment
        // Note: Parley 0.2 has Start, Middle, End, Justified
        // For LTR text: Start=left, Middle=center, End=right
        let parley_align = match align {
            designbot_core::canvas::TextAlign::Left => parley::layout::Alignment::Start,
            designbot_core::canvas::TextAlign::Center => parley::layout::Alignment::Middle,
            designbot_core::canvas::TextAlign::Right => parley::layout::Alignment::End,
            designbot_core::canvas::TextAlign::Start => parley::layout::Alignment::Start,
            designbot_core::canvas::TextAlign::End => parley::layout::Alignment::End,
            designbot_core::canvas::TextAlign::Justified => parley::layout::Alignment::Justified,
        };

        layout.align(max_width, parley_align);

        // For single-line text without a container width, manually adjust x position
        // based on text width and alignment (like DrawBot)
        let x_adjustment = if bounds.is_none() {
            let text_width = layout.width() as f64;
            match align {
                designbot_core::canvas::TextAlign::Center => -text_width / 2.0,
                designbot_core::canvas::TextAlign::Right | designbot_core::canvas::TextAlign::End => -text_width,
                _ => 0.0, // Left, Start, Justified
            }
        } else {
            0.0 // For text_box, use Parley's alignment offset
        };
        let adjusted_x = x + x_adjustment;

        // Convert brush to color
        let color = Self::brush_to_color(brush);

        // Vertical anchoring in DrawBot user space (y-up): text() puts the
        // FIRST baseline at y and stacks later lines downward; text_box()
        // fills downward from the top edge of a box whose bottom-left is at
        // (x, y). Parley's line baselines are measured y-down from the layout
        // top, so convert them into user-space baselines.
        let first_baseline = layout
            .lines()
            .next()
            .map(|l| l.metrics().baseline as f64)
            .unwrap_or(0.0);

        // Render each glyph
        for line in layout.lines() {
            // Get line metrics for proper vertical positioning and alignment offset
            let line_metrics = line.metrics();
            let line_y = match bounds {
                // text_box: first baseline hangs below the box top (y + h)
                Some((_, h)) => y + h - line_metrics.baseline as f64,
                // text: first baseline exactly at y
                None => y - (line_metrics.baseline as f64 - first_baseline),
            };
            let line_x_offset = line_metrics.offset as f64; // Horizontal offset for alignment

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
                            // Apply the run's variable-font axis coordinates so
                            // outlines match the shaped/spaced instance — without
                            // this, variations affect metrics but not the ink.
                            .normalized_coords(run.normalized_coords().iter().copied())
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
                            // Use adjusted x (for single-line alignment), line alignment offset (for text_box), and glyph offset
                            let glyph_x = adjusted_x + line_x_offset + glyph_x_offset as f64;
                            let glyph_y = line_y;

                            // Font glyphs are y-up, exactly like DrawBot user
                            // space; the command transform already carries the
                            // single flip into device space, so glyphs are
                            // placed without one of their own.
                            // Note: swash already scaled the outline to font_size
                            let glyph_transform = kurbo::Affine::translate((glyph_x, glyph_y));
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
