//! Vector PDF backend: a `PaintScene` implementation that writes PDF content
//! streams instead of rasterizing. Because the renderer paints *everything*
//! through `PaintScene` — including text, which arrives as filled glyph
//! outline paths — every canvas feature that renders to PNG renders to PDF
//! as true vectors, one page per canvas page.
//!
//! v1 limitations (each warns once instead of failing): gradients render as
//! their first stop color, raster images are skipped, layer alpha/blend modes
//! are ignored (clip is honored), box shadows draw unblurred.

use anyrender::{NormalizedCoord, Paint, PaintRef, PaintScene};
use kurbo::{Affine, PathEl, Rect, Shape, Stroke};
use peniko::{BlendMode, Color, Fill, FontData, StyleRef};
use pdf_writer::types::{LineCapStyle, LineJoinStyle};
use pdf_writer::Content;

const TOLERANCE: f64 = 0.1;

pub struct PdfScenePainter {
    content: Content,
    height: f64,
    warnings: Vec<&'static str>,
}

impl PdfScenePainter {
    pub fn new(height: f64) -> Self {
        let mut content = Content::new();
        // designbot is y-down top-left; PDF is y-up bottom-left. One global
        // flip up front lets every command use canvas coordinates directly.
        content.transform([1.0, 0.0, 0.0, -1.0, 0.0, height as f32]);
        Self {
            content,
            height,
            warnings: Vec::new(),
        }
    }

    pub fn finish(self) -> (Vec<u8>, Vec<&'static str>) {
        (self.content.finish().into_vec(), self.warnings)
    }

    fn warn_once(&mut self, msg: &'static str) {
        if !self.warnings.contains(&msg) {
            self.warnings.push(msg);
        }
    }

    /// Solid color from a paint; gradients degrade to their first stop.
    fn paint_rgb(&mut self, paint: PaintRef<'_>) -> Option<(f32, f32, f32, f32)> {
        let color: Color = match paint {
            Paint::Solid(color) => color,
            Paint::Gradient(gradient) => {
                self.warn_once("pdf: gradients render as their first stop color (v1)");
                gradient
                    .stops
                    .first()
                    .map(|s| s.color.to_alpha_color::<color::Srgb>())?
            }
            Paint::Image(_) => {
                self.warn_once("pdf: raster images are not embedded yet (v1) — skipped");
                return None;
            }
            Paint::Custom(_) => return None,
        };
        let [r, g, b, a] = color.components;
        if a < 0.999 {
            self.warn_once("pdf: per-paint alpha is ignored (v1)");
        }
        Some((r, g, b, a))
    }

    fn apply_transform(&mut self, transform: Affine) {
        if transform != Affine::IDENTITY {
            let c = transform.as_coeffs();
            self.content.transform([
                c[0] as f32,
                c[1] as f32,
                c[2] as f32,
                c[3] as f32,
                c[4] as f32,
                c[5] as f32,
            ]);
        }
    }

    /// Emit a shape's path elements as PDF path construction ops.
    fn emit_shape(&mut self, shape: &impl Shape) {
        let mut start = kurbo::Point::ZERO;
        let mut cur = kurbo::Point::ZERO;
        for el in shape.path_elements(TOLERANCE) {
            match el {
                PathEl::MoveTo(p) => {
                    self.content.move_to(p.x as f32, p.y as f32);
                    start = p;
                    cur = p;
                }
                PathEl::LineTo(p) => {
                    self.content.line_to(p.x as f32, p.y as f32);
                    cur = p;
                }
                PathEl::QuadTo(q, p) => {
                    // PDF has no quadratic op; elevate to cubic.
                    let c1 = cur + (q - cur) * (2.0 / 3.0);
                    let c2 = p + (q - p) * (2.0 / 3.0);
                    self.content.cubic_to(
                        c1.x as f32,
                        c1.y as f32,
                        c2.x as f32,
                        c2.y as f32,
                        p.x as f32,
                        p.y as f32,
                    );
                    cur = p;
                }
                PathEl::CurveTo(c1, c2, p) => {
                    self.content.cubic_to(
                        c1.x as f32,
                        c1.y as f32,
                        c2.x as f32,
                        c2.y as f32,
                        p.x as f32,
                        p.y as f32,
                    );
                    cur = p;
                }
                PathEl::ClosePath => {
                    self.content.close_path();
                    cur = start;
                }
            }
        }
    }
}

impl PaintScene for PdfScenePainter {
    fn reset(&mut self) {
        let height = self.height;
        *self = Self::new(height);
    }

    fn push_layer(
        &mut self,
        _blend: impl Into<BlendMode>,
        alpha: f32,
        transform: Affine,
        clip: &impl Shape,
    ) {
        if alpha < 0.999 {
            self.warn_once("pdf: layer alpha is ignored (v1)");
        }
        self.content.save_state();
        self.apply_transform(transform);
        self.emit_shape(clip);
        self.content.clip_nonzero();
        self.content.end_path();
    }

    fn pop_layer(&mut self) {
        self.content.restore_state();
    }

    fn stroke<'a>(
        &mut self,
        style: &Stroke,
        transform: Affine,
        brush: impl Into<PaintRef<'a>>,
        _brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        let Some((r, g, b, _)) = self.paint_rgb(brush.into()) else {
            return;
        };
        self.content.save_state();
        self.apply_transform(transform);
        self.content.set_stroke_rgb(r, g, b);
        self.content.set_line_width(style.width as f32);
        self.content.set_line_cap(match style.start_cap {
            kurbo::Cap::Butt => LineCapStyle::ButtCap,
            kurbo::Cap::Round => LineCapStyle::RoundCap,
            kurbo::Cap::Square => LineCapStyle::ProjectingSquareCap,
        });
        self.content.set_line_join(match style.join {
            kurbo::Join::Miter => LineJoinStyle::MiterJoin,
            kurbo::Join::Round => LineJoinStyle::RoundJoin,
            kurbo::Join::Bevel => LineJoinStyle::BevelJoin,
        });
        self.content.set_miter_limit(style.miter_limit as f32);
        if !style.dash_pattern.is_empty() {
            self.content.set_dash_pattern(
                style.dash_pattern.iter().map(|d| *d as f32),
                style.dash_offset as f32,
            );
        }
        self.emit_shape(shape);
        self.content.stroke();
        self.content.restore_state();
    }

    fn fill<'a>(
        &mut self,
        style: Fill,
        transform: Affine,
        brush: impl Into<PaintRef<'a>>,
        _brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        let Some((r, g, b, _)) = self.paint_rgb(brush.into()) else {
            return;
        };
        self.content.save_state();
        self.apply_transform(transform);
        self.content.set_fill_rgb(r, g, b);
        self.emit_shape(shape);
        match style {
            Fill::NonZero => self.content.fill_nonzero(),
            Fill::EvenOdd => self.content.fill_even_odd(),
        };
        self.content.restore_state();
    }

    fn draw_glyphs<'a, 's: 'a>(
        &'s mut self,
        _font: &'a FontData,
        _font_size: f32,
        _hint: bool,
        _normalized_coords: &'a [NormalizedCoord],
        _style: impl Into<StyleRef<'a>>,
        _brush: impl Into<PaintRef<'a>>,
        _brush_alpha: f32,
        _transform: Affine,
        _glyph_transform: Option<Affine>,
        _glyphs: impl Iterator<Item = anyrender::Glyph>,
    ) {
        // The designbot renderer draws text as filled outline paths, so this
        // path is unused today. If it ever is used, make it visible.
        self.warn_once("pdf: draw_glyphs is not implemented — text missing");
    }

    fn draw_box_shadow(
        &mut self,
        transform: Affine,
        rect: Rect,
        brush: Color,
        radius: f64,
        _std_dev: f64,
    ) {
        self.warn_once("pdf: box shadows draw unblurred (v1)");
        let rounded = rect.to_rounded_rect(radius);
        self.fill(
            Fill::NonZero,
            transform,
            Paint::Solid(brush),
            None,
            &rounded,
        );
    }
}
