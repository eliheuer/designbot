//! Vector SVG backend: a `PaintScene` implementation that writes an SVG
//! document instead of rasterizing. Like the PDF backend, it works because
//! the renderer paints *everything* through `PaintScene` — text included,
//! which arrives as filled glyph outline paths — so every canvas feature that
//! renders to PNG renders to SVG as true vectors.
//!
//! SVG is y-down top-left like the canvas, so (unlike PDF) no global flip is
//! needed. SVG is single-page: the renderer emits one page per file.
//!
//! v1 limitations (each warns once instead of failing): gradients render as
//! their first stop color, raster images are skipped, layer blend modes are
//! ignored (clip and alpha are honored), box shadows draw unblurred.

use anyrender::{NormalizedCoord, Paint, PaintRef, PaintScene};
use kurbo::{Affine, PathEl, Rect, Shape, Stroke};
use peniko::{BlendMode, Color, Fill, FontData, StyleRef};

const TOLERANCE: f64 = 0.1;

pub struct SvgScenePainter {
    width: f64,
    height: f64,
    defs: String,
    body: String,
    clip_id: usize,
    warnings: Vec<&'static str>,
}

impl SvgScenePainter {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            defs: String::new(),
            body: String::new(),
            clip_id: 0,
            warnings: Vec::new(),
        }
    }

    pub fn finish(self) -> (Vec<u8>, Vec<&'static str>) {
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" \
             height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n",
            w = fnum(self.width),
            h = fnum(self.height),
        );
        if !self.defs.is_empty() {
            out.push_str("<defs>\n");
            out.push_str(&self.defs);
            out.push_str("</defs>\n");
        }
        out.push_str(&self.body);
        out.push_str("</svg>\n");
        (out.into_bytes(), self.warnings)
    }

    fn warn_once(&mut self, msg: &'static str) {
        if !self.warnings.contains(&msg) {
            self.warnings.push(msg);
        }
    }

    /// Solid color from a paint; gradients degrade to their first stop.
    fn paint_rgba(&mut self, paint: PaintRef<'_>) -> Option<(f32, f32, f32, f32)> {
        let color: Color = match paint {
            Paint::Solid(color) => color,
            Paint::Gradient(gradient) => {
                self.warn_once("svg: gradients render as their first stop color (v1)");
                gradient
                    .stops
                    .first()
                    .map(|s| s.color.to_alpha_color::<color::Srgb>())?
            }
            Paint::Image(_) => {
                self.warn_once("svg: raster images are not embedded yet (v1) — skipped");
                return None;
            }
            Paint::Custom(_) => return None,
        };
        let [r, g, b, a] = color.components;
        Some((r, g, b, a))
    }
}

impl PaintScene for SvgScenePainter {
    fn reset(&mut self) {
        *self = Self::new(self.width, self.height);
    }

    fn push_layer(
        &mut self,
        _blend: impl Into<BlendMode>,
        alpha: f32,
        transform: Affine,
        clip: &impl Shape,
    ) {
        self.clip_id += 1;
        let id = self.clip_id;
        // The clip is expressed in the layer's transformed space; the outer
        // <g> carries that transform, and the inner (untransformed) <g>
        // resolves the userSpaceOnUse clip in it. Nested content composes its
        // own transforms on top, matching the PDF backend's CTM stack.
        self.defs.push_str(&format!(
            "<clipPath id=\"clip{id}\" clipPathUnits=\"userSpaceOnUse\">\
             <path d=\"{}\"/></clipPath>\n",
            path_data(clip),
        ));
        let mut open = format!("<g{}", matrix_attr(transform));
        if alpha < 0.999 {
            open.push_str(&format!(" opacity=\"{}\"", fnum(alpha as f64)));
        }
        open.push('>');
        self.body.push_str(&open);
        self.body
            .push_str(&format!("<g clip-path=\"url(#clip{id})\">\n"));
    }

    fn pop_layer(&mut self) {
        self.body.push_str("</g></g>\n");
    }

    fn stroke<'a>(
        &mut self,
        style: &Stroke,
        transform: Affine,
        brush: impl Into<PaintRef<'a>>,
        _brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        let Some((r, g, b, a)) = self.paint_rgba(brush.into()) else {
            return;
        };
        let mut attrs = format!(
            " fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"",
            color_hex(r, g, b),
            fnum(style.width),
        );
        if a < 0.999 {
            attrs.push_str(&format!(" stroke-opacity=\"{}\"", fnum(a as f64)));
        }
        attrs.push_str(match style.start_cap {
            kurbo::Cap::Butt => "",
            kurbo::Cap::Round => " stroke-linecap=\"round\"",
            kurbo::Cap::Square => " stroke-linecap=\"square\"",
        });
        attrs.push_str(match style.join {
            kurbo::Join::Miter => "",
            kurbo::Join::Round => " stroke-linejoin=\"round\"",
            kurbo::Join::Bevel => " stroke-linejoin=\"bevel\"",
        });
        if matches!(style.join, kurbo::Join::Miter) {
            attrs.push_str(&format!(
                " stroke-miterlimit=\"{}\"",
                fnum(style.miter_limit)
            ));
        }
        if !style.dash_pattern.is_empty() {
            let dashes: Vec<String> =
                style.dash_pattern.iter().map(|d| fnum(*d)).collect();
            attrs.push_str(&format!(
                " stroke-dasharray=\"{}\" stroke-dashoffset=\"{}\"",
                dashes.join(","),
                fnum(style.dash_offset),
            ));
        }
        self.body.push_str(&format!(
            "<path d=\"{}\"{}{}/>\n",
            path_data(shape),
            attrs,
            matrix_attr(transform),
        ));
    }

    fn fill<'a>(
        &mut self,
        style: Fill,
        transform: Affine,
        brush: impl Into<PaintRef<'a>>,
        _brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        let Some((r, g, b, a)) = self.paint_rgba(brush.into()) else {
            return;
        };
        let mut attrs = format!(" fill=\"{}\"", color_hex(r, g, b));
        if a < 0.999 {
            attrs.push_str(&format!(" fill-opacity=\"{}\"", fnum(a as f64)));
        }
        if matches!(style, Fill::EvenOdd) {
            attrs.push_str(" fill-rule=\"evenodd\"");
        }
        self.body.push_str(&format!(
            "<path d=\"{}\"{}{}/>\n",
            path_data(shape),
            attrs,
            matrix_attr(transform),
        ));
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
        self.warn_once("svg: draw_glyphs is not implemented — text missing");
    }

    fn draw_box_shadow(
        &mut self,
        transform: Affine,
        rect: Rect,
        brush: Color,
        radius: f64,
        _std_dev: f64,
    ) {
        self.warn_once("svg: box shadows draw unblurred (v1)");
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

/// Format a coordinate compactly: up to 3 decimals, trailing zeros trimmed.
fn fnum(v: f64) -> String {
    let s = format!("{v:.3}");
    // `{:.3}` always emits a decimal point, so trimming zeros then the dot is
    // safe (never eats significant trailing zeros of an integer).
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn color_hex(r: f32, g: f32, b: f32) -> String {
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", byte(r), byte(g), byte(b))
}

fn matrix_attr(transform: Affine) -> String {
    if transform == Affine::IDENTITY {
        return String::new();
    }
    let c = transform.as_coeffs();
    format!(
        " transform=\"matrix({} {} {} {} {} {})\"",
        fnum(c[0]),
        fnum(c[1]),
        fnum(c[2]),
        fnum(c[3]),
        fnum(c[4]),
        fnum(c[5]),
    )
}

/// Build an SVG path `d` string from any kurbo shape.
fn path_data(shape: &impl Shape) -> String {
    let mut d = String::new();
    for el in shape.path_elements(TOLERANCE) {
        match el {
            PathEl::MoveTo(p) => {
                d.push_str(&format!("M{} {} ", fnum(p.x), fnum(p.y)))
            }
            PathEl::LineTo(p) => {
                d.push_str(&format!("L{} {} ", fnum(p.x), fnum(p.y)))
            }
            PathEl::QuadTo(c, p) => d.push_str(&format!(
                "Q{} {} {} {} ",
                fnum(c.x),
                fnum(c.y),
                fnum(p.x),
                fnum(p.y),
            )),
            PathEl::CurveTo(c1, c2, p) => d.push_str(&format!(
                "C{} {} {} {} {} {} ",
                fnum(c1.x),
                fnum(c1.y),
                fnum(c2.x),
                fnum(c2.y),
                fnum(p.x),
                fnum(p.y),
            )),
            PathEl::ClosePath => d.push_str("Z "),
        }
    }
    d.trim_end().to_string()
}
