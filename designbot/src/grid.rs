//! Design grids — a toggle-able overlay for laying out a composition.
//!
//! Two flavors, both built with the same builder:
//!
//! * [`Grid::unit`] — a basic uniform grid: a border, lines every `unit`, an
//!   optional finer sub-grid, and center lines. (The DrawBot `GRID_VIEW` idea.)
//! * [`Grid::modular`] — a Swiss / Müller-Brockmann modular grid: the content
//!   area divided into `columns` × `rows` module boxes separated by a `gutter`.
//!
//! Draw it right after the background (so it sits behind the art) and gate it
//! on a `const SHOW_GRID: bool` you flip while designing:
//!
//! ```no_run
//! # use designbot::prelude::*;
//! # let (mut ctx, t, w, h) = (Canvas::new(100.0, 100.0), Theme::dark(), 100.0, 100.0);
//! const SHOW_GRID: bool = true;
//! if SHOW_GRID {
//!     Grid::unit(64.0).margin(120.0).subdivisions(2).color(t.grid).draw(&mut ctx, w, h);
//! }
//! ```

use crate::{Canvas, Color};

#[derive(Clone, Copy, Debug)]
enum Kind {
    /// Uniform grid: a line every `unit`, with `subdivisions` finer lines.
    Unit { unit: f64, subdivisions: u32 },
    /// Modular grid: `columns` x `rows` module boxes with a `gutter` between.
    Modular { columns: u32, rows: u32, gutter: f64 },
}

/// A configurable, toggle-able design grid. Build with [`Grid::unit`] or
/// [`Grid::modular`], tune with the chainable setters, then [`Grid::draw`].
#[derive(Clone, Copy, Debug)]
pub struct Grid {
    kind: Kind,
    margin: f64,
    color: Color,
    subcolor: Color,
    stroke: f64,
    center: bool,
    border: bool,
}

const DEFAULT_LINE: Color = Color { r: 128, g: 128, b: 128, a: 90 };

impl Grid {
    /// A basic uniform grid with a line every `unit` px.
    pub fn unit(unit: f64) -> Self {
        Grid {
            kind: Kind::Unit { unit, subdivisions: 0 },
            margin: 0.0,
            color: DEFAULT_LINE,
            subcolor: DEFAULT_LINE.with_alpha(DEFAULT_LINE.a / 2),
            stroke: 1.0,
            center: true,
            border: true,
        }
    }

    /// A UPM-aware powers-of-two grid, mapped to the font's own coordinate
    /// system: 1 font unit = 1 canvas pixel. `units_per_em` is the font's UPM
    /// (e.g. 1024). Defaults to the **8-unit structural grid** (the faint
    /// sub-lines) beneath a heavier reference line every `UPM / 8` units, so it
    /// tiles any powers-of-two canvas exactly (2048, 1024, … are all multiples
    /// of both 8 and UPM/8). Change the structural step with
    /// [`Grid::structural`], or drop the reference with `.subdivisions(0)`.
    ///
    /// ```no_run
    /// # use designbot::prelude::*;
    /// # let (mut c, t) = (Canvas::new(2048.0, 1024.0), Theme::dark());
    /// Grid::upm(1024.0).color(t.grid).border(false).draw(&mut c, 2048.0, 1024.0);
    /// ```
    pub fn upm(units_per_em: f64) -> Self {
        let reference = units_per_em / 8.0; // e.g. 128 for a 1024 UPM
        let subdivisions = (reference / 8.0).round().max(1.0) as u32; // 8-unit
        Grid::unit(reference).subdivisions(subdivisions).border(false)
    }

    /// Set the structural (finest) grid step in font units for a [`Grid::upm`]
    /// grid — 8 by default. E.g. `.structural(16)` for a 16-unit grid.
    pub fn structural(mut self, units: f64) -> Self {
        if let Kind::Unit { unit, .. } = self.kind {
            let n = (unit / units).round().max(1.0) as u32;
            self.kind = Kind::Unit { unit, subdivisions: n };
        }
        self
    }

    /// A Swiss / Müller-Brockmann modular grid: `columns` x `rows` boxes.
    pub fn modular(columns: u32, rows: u32) -> Self {
        Grid {
            kind: Kind::Modular { columns: columns.max(1), rows: rows.max(1), gutter: 24.0 },
            margin: 100.0,
            color: DEFAULT_LINE,
            subcolor: DEFAULT_LINE.with_alpha(DEFAULT_LINE.a / 2),
            stroke: 1.0,
            center: false,
            border: false,
        }
    }

    /// Outer margin (the grid is drawn inside it).
    pub fn margin(mut self, margin: f64) -> Self {
        self.margin = margin;
        self
    }

    /// Finer lines per unit cell (unit grids only). `0` or `1` disables them.
    pub fn subdivisions(mut self, n: u32) -> Self {
        if let Kind::Unit { unit, .. } = self.kind {
            self.kind = Kind::Unit { unit, subdivisions: n };
        }
        self
    }

    /// Gap between modules (modular grids only).
    pub fn gutter(mut self, gutter: f64) -> Self {
        if let Kind::Modular { columns, rows, .. } = self.kind {
            self.kind = Kind::Modular { columns, rows, gutter };
        }
        self
    }

    /// Main line color. The sub-grid color is derived at half alpha unless you
    /// also call [`Grid::subcolor`].
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self.subcolor = color.with_alpha(color.a / 2);
        self
    }

    /// Override the sub-grid / finer-line color explicitly.
    pub fn subcolor(mut self, color: Color) -> Self {
        self.subcolor = color;
        self
    }

    pub fn stroke_width(mut self, w: f64) -> Self {
        self.stroke = w;
        self
    }

    /// Draw the two center lines (default: on for unit, off for modular).
    pub fn center_lines(mut self, on: bool) -> Self {
        self.center = on;
        self
    }

    /// Draw the outer border rectangle (default: on for unit, off for modular).
    pub fn border(mut self, on: bool) -> Self {
        self.border = on;
        self
    }

    /// Draw the grid onto `ctx` for a `w` x `h` canvas.
    pub fn draw(&self, ctx: &mut Canvas, w: f64, h: f64) {
        let (x0, y0, x1, y1) = (self.margin, self.margin, w - self.margin, h - self.margin);
        match self.kind {
            Kind::Unit { unit, subdivisions } => {
                if subdivisions > 1 && unit > 0.0 {
                    let sub = unit / subdivisions as f64;
                    ctx.no_fill().stroke(self.subcolor).stroke_width(self.stroke);
                    lines(ctx, x0, y0, x1, y1, sub);
                }
                if unit > 0.0 {
                    ctx.no_fill().stroke(self.color).stroke_width(self.stroke);
                    lines(ctx, x0, y0, x1, y1, unit);
                }
            }
            Kind::Modular { columns, rows, gutter } => {
                let cw = (x1 - x0 - (columns - 1) as f64 * gutter) / columns as f64;
                let rh = (y1 - y0 - (rows - 1) as f64 * gutter) / rows as f64;
                ctx.no_fill().stroke(self.color).stroke_width(self.stroke);
                for c in 0..columns {
                    for r in 0..rows {
                        let cx = x0 + c as f64 * (cw + gutter);
                        let ry = y0 + r as f64 * (rh + gutter);
                        ctx.rect(cx, ry, cw, rh);
                    }
                }
            }
        }
        if self.center {
            // Keep the center lines inside the margin, like the rest of the grid.
            ctx.no_fill().stroke(self.color).stroke_width(self.stroke);
            ctx.line(w / 2.0, y0, w / 2.0, y1);
            ctx.line(x0, h / 2.0, x1, h / 2.0);
        }
        if self.border {
            ctx.no_fill().stroke(self.color).stroke_width(self.stroke);
            ctx.rect(x0, y0, x1 - x0, y1 - y0);
        }
    }
}

/// Evenly spaced vertical + horizontal lines across [x0,x1] x [y0,y1] at `step`.
fn lines(ctx: &mut Canvas, x0: f64, y0: f64, x1: f64, y1: f64, step: f64) {
    let mut x = x0;
    while x <= x1 + 0.01 {
        ctx.line(x, y0, x, y1);
        x += step;
    }
    let mut y = y0;
    while y <= y1 + 0.01 {
        ctx.line(x0, y, x1, y);
        y += step;
    }
}
