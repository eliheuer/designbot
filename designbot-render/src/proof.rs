//! Built-in default **print proof** generator.
//!
//! `designbot proof <font> -o proof.pdf` introspects any font (axes, named
//! instances, charset, metrics, features) and emits a multi-page, color-managed
//! PDF proof — no per-repo script required. Designed as a superset by eye of
//! Google Fonts' diffenator2 `proof` view.
//!
//! US Letter landscape (792 × 612 pt), laid out on a 6-column Swiss modular
//! grid (toggle with `--no-grid`; on by default while the proof is in
//! development). Technical data is set in IBM Plex Mono (bundled, OFL).
//! See virtua-grotesk/documentation/proofs/PROOF_SPEC.md for the page plan.

use crate::Renderer;
use designbot_core::{Canvas, Color, DesignBotError, Grid, TextAlign};
use std::path::Path;

// --- bundled monospace for technical chrome (OFL, see assets/) -------------
const MONO_TTF: &[u8] = include_bytes!("../assets/IBMPlexMono-Regular.ttf");
const MONO: &str = "IBM Plex Mono";

// --- page geometry (US Letter landscape, points = px) ----------------------
const W: f64 = 792.0;
const H: f64 = 612.0;
const M: f64 = 54.0; // margin
const COLS: usize = 6; // Swiss modular columns
const GUTTER: f64 = 16.0;
const GRID_ROWS: u32 = 6;

/// Width of a single grid column.
fn col_w() -> f64 {
    (W - 2.0 * M - (COLS as f64 - 1.0) * GUTTER) / COLS as f64
}
/// Left edge of grid column `i` (0-based).
fn col_x(i: usize) -> f64 {
    M + i as f64 * (col_w() + GUTTER)
}
/// Width of a text block spanning `n` grid columns.
fn span_w(n: usize) -> f64 {
    n as f64 * col_w() + (n as f64 - 1.0) * GUTTER
}

fn ink() -> Color {
    Color::rgb(0x23, 0x23, 0x23)
}
fn paper() -> Color {
    Color::rgb(0xff, 0xff, 0xff)
}
fn rule() -> Color {
    Color::rgb(0xcc, 0xcc, 0xcc)
}
fn faint() -> Color {
    Color::rgb(0x80, 0x80, 0x80) // running head, gutter labels
}
fn hair() -> Color {
    Color::rgb(0xa6, 0xa6, 0xa6) // tiny hex labels
}
fn grid_red() -> Color {
    Color::rgb(0xe6, 0x9a, 0x9a) // light red guide grid
}

/// Format a float axis value without a trailing `.0` (400.0 -> "400").
fn num(v: f32) -> String {
    if v.fract().abs() < 1e-4 {
        format!("{}", v as i64)
    } else {
        format!("{:.2}", v)
    }
}

/// A paragraph of neutral prose to exercise the font in running text. Repeated
/// to fill tall columns; the same text across columns makes size / leading /
/// tracking differences directly comparable.
const SAMPLE: &str = "Typography is the craft of arranging letters so that language becomes visible. A typeface earns its keep in running text, where the rhythm of repeated forms, the fit between letters, and the balance of black and white decide whether a page invites reading or resists it. Grotesk designs strip ornament away and let structure carry the voice: even strokes, open counters, and a steady cadence from one word to the next. Set at reading sizes, the plain letters gather into a quiet, legible texture. This proof tests that texture across sizes, leading, and spacing before the design is trusted with real words. ";

fn filled(min_chars: usize) -> String {
    let mut s = String::new();
    while s.len() < min_chars {
        s.push_str(SAMPLE);
    }
    s
}

// --- introspection ---------------------------------------------------------

struct Axis {
    tag: String,
    min: f32,
    default: f32,
    max: f32,
}

struct NamedInstance {
    name: String,
    values: Vec<f32>,
}

struct FontFacts {
    family: String,
    version: String,
    upm: u16,
    glyph_count: u16,
    encoded: usize,
    axes: Vec<Axis>,
    instances: Vec<NamedInstance>,
    features: Vec<String>,
    /// (codepoint, glyph id), sorted by codepoint.
    cmap: Vec<(u32, u16)>,
}

fn tag_str(t: swash::Tag) -> String {
    String::from_utf8_lossy(&t.to_be_bytes()).trim().to_string()
}

fn introspect(data: &[u8]) -> Result<FontFacts, DesignBotError> {
    use swash::{FontRef, StringId};

    let font = FontRef::from_index(data, 0)
        .ok_or_else(|| DesignBotError::FontError("could not parse font".into()))?;

    let metrics = font.metrics(&[]);

    let (mut family, mut family_legacy, mut version) = (None, None, None);
    for s in font.localized_strings() {
        match s.id() {
            StringId::TypographicFamily if family.is_none() => family = Some(s.to_string()),
            StringId::Family if family_legacy.is_none() => family_legacy = Some(s.to_string()),
            StringId::Version if version.is_none() => version = Some(s.to_string()),
            _ => {}
        }
    }
    let family = family.or(family_legacy).unwrap_or_else(|| "Unknown".into());
    let version = version
        .map(|v| v.trim().trim_start_matches("Version").trim().to_string())
        .unwrap_or_default();

    let axes: Vec<Axis> = font
        .variations()
        .map(|v| Axis {
            tag: tag_str(v.tag()),
            min: v.min_value(),
            default: v.default_value(),
            max: v.max_value(),
        })
        .collect();

    let instances: Vec<NamedInstance> = font
        .instances()
        .map(|i| NamedInstance {
            name: i.name(None).map(|n| n.to_string()).unwrap_or_default(),
            values: i.values().collect(),
        })
        .collect();

    let mut features: Vec<String> = Vec::new();
    for f in font.features() {
        let t = tag_str(f.tag());
        if !features.contains(&t) {
            features.push(t);
        }
    }
    features.sort();

    let mut cmap: Vec<(u32, u16)> = Vec::new();
    font.charmap().enumerate(|cp, gid| {
        if gid != 0 {
            cmap.push((cp, gid));
        }
    });
    cmap.sort_by_key(|&(cp, _)| cp);
    cmap.dedup_by_key(|&mut (cp, _)| cp);
    let encoded = cmap.len();

    Ok(FontFacts {
        family,
        version,
        upm: metrics.units_per_em,
        glyph_count: metrics.glyph_count,
        encoded,
        axes,
        instances,
        features,
        cmap,
    })
}

/// Best-effort short git hash of the repo containing `path`; empty on failure.
fn git_hash(path: &Path) -> String {
    let dir = path.parent().unwrap_or(Path::new("."));
    std::process::Command::new("git")
        .args(["-C"])
        .arg(dir)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Best-effort ISO date (YYYY-MM-DD) via the system `date`; empty on failure.
fn today() -> String {
    std::process::Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
}

// --- proof builder ---------------------------------------------------------

struct Proof<'a> {
    ctx: Canvas,
    facts: &'a FontFacts,
    date: String,
    git: String,
    folio: usize,
    grid: bool,
}

impl<'a> Proof<'a> {
    fn fam(&self) -> String {
        self.facts.family.clone()
    }

    /// Light-red Swiss modular grid overlay (guide while designing the proof).
    fn grid_overlay(&mut self) {
        Grid::modular(COLS as u32, GRID_ROWS)
            .margin(M)
            .gutter(GUTTER)
            .color(grid_red())
            .stroke_width(0.5)
            .draw(&mut self.ctx, W, H);
    }

    /// Small self-identifying header on every interior page + a hairline rule.
    fn running_head(&mut self, section: &str) {
        let y = H - 38.0;
        let fam = self.fam();
        let left = format!("{}  ·  {}", fam, section);
        let right = format!("{}   ·   {}", self.date, self.folio);
        self.ctx
            .no_stroke()
            .fill(faint())
            .font(MONO)
            .clear_font_variations()
            .font_size(8.0)
            .tracking(0.2)
            .auto_line_height()
            .text_align(TextAlign::Left);
        self.ctx.text(&left, M, y);
        self.ctx.text_align(TextAlign::Right);
        self.ctx.text(&right, W - M, y);
        self.ctx.stroke(rule()).stroke_width(0.5);
        self.ctx.line(M, y - 7.0, W - M, y - 7.0);
    }

    /// Page title under the running head (proofed font, regular weight — the
    /// proof only sets Bold where it is deliberately showing Bold).
    fn page_title(&mut self, title: &str) {
        self.ctx
            .no_stroke()
            .fill(ink())
            .font(&self.facts.family)
            .clear_font_variations()
            .font_variation("wght", 400.0)
            .font_size(20.0)
            .tracking(0.0)
            .auto_line_height()
            .text_align(TextAlign::Left);
        self.ctx.text(title, M, H - 74.0);
        self.ctx.clear_font_variations();
    }

    /// Start a fresh interior sheet: white, grid overlay, running head, title.
    fn new_sheet(&mut self, section: &str, title: &str) {
        self.folio += 1;
        self.ctx.new_page();
        self.ctx.background(paper());
        if self.grid {
            self.grid_overlay();
        }
        self.running_head(section);
        self.page_title(title);
    }

    /// A monospace field: tiny gray caps label + stacked value lines.
    fn field(&mut self, x: f64, top: f64, label: &str, values: &[String]) {
        self.ctx
            .no_stroke()
            .font(MONO)
            .clear_font_variations()
            .text_align(TextAlign::Left)
            .auto_line_height()
            .tracking(0.6)
            .fill(faint())
            .font_size(6.5);
        self.ctx.text(&label.to_uppercase(), x, top);
        self.ctx.fill(ink()).tracking(0.0).font_size(8.5);
        let mut y = top - 15.0;
        for v in values {
            self.ctx.text(v, x, y);
            y -= 12.5;
        }
    }

    // ---- pages ----

    fn cover(&mut self) {
        self.ctx.background(paper());
        if self.grid {
            self.grid_overlay();
        }

        // Family name — regular weight, a little tracking, large.
        self.ctx
            .no_stroke()
            .fill(ink())
            .font(&self.facts.family)
            .clear_font_variations()
            .font_variation("wght", 400.0)
            .font_size(78.0)
            .tracking(1.5)
            .auto_line_height()
            .text_align(TextAlign::Left);
        self.ctx.text(&self.facts.family, M, H - 150.0);

        // Technical data — monospace, in Swiss grid columns.
        let top = 250.0;
        let axes: Vec<String> = self
            .facts
            .axes
            .iter()
            .map(|a| format!("{} {}-{}", a.tag, num(a.min), num(a.max)))
            .collect();
        let instances: Vec<String> = self
            .facts
            .instances
            .iter()
            .map(|i| {
                let v = i.values.iter().map(|x| num(*x)).collect::<Vec<_>>().join("/");
                if v.is_empty() {
                    i.name.clone()
                } else {
                    format!("{} {}", i.name, v)
                }
            })
            .collect();
        let character = vec![
            format!("{} glyphs", self.facts.glyph_count),
            format!("{} encoded", self.facts.encoded),
            format!("{} upm", self.facts.upm),
        ];
        // features wrapped 2 per line
        let features: Vec<String> = self
            .facts
            .features
            .chunks(2)
            .map(|c| c.join(" "))
            .collect();
        let mut meta = Vec::new();
        if !self.facts.version.is_empty() {
            meta.push(format!("version {}", self.facts.version));
        }
        if !self.git.is_empty() {
            meta.push(format!("commit {}", self.git));
        }
        if !self.date.is_empty() {
            meta.push(format!("generated {}", self.date));
        }

        self.field(col_x(0), top, "Axes", &axes);
        self.field(col_x(1), top, "Instances", &instances);
        self.field(col_x(2), top, "Character", &character);
        self.field(col_x(3), top, "Features", &features);
        self.field(col_x(4), top, "Build", &meta);
    }

    fn char_set(&mut self) {
        self.new_sheet("Character Set", "Character Set");
        let cell_w = 46.0;
        let cell_h = 58.0;
        let cols = (((W - 2.0 * M) / cell_w).floor() as usize).max(1);
        let top = H - 104.0;
        let bottom = M;
        let glyph_size = cell_h * 0.58;

        let mut col = 0usize;
        let mut row_top = top;
        let cmap = self.facts.cmap.clone();
        for (cp, _gid) in cmap {
            if cp < 0x20 {
                continue;
            }
            let Some(ch) = char::from_u32(cp) else { continue };
            if col == 0 && row_top - cell_h < bottom {
                self.new_sheet("Character Set", "Character Set (cont.)");
                row_top = top;
            }
            let x = M + col as f64 * cell_w;
            let cx = x + cell_w / 2.0;
            let base = row_top - cell_h + cell_h * 0.34;

            self.ctx.no_fill().stroke(rule()).stroke_width(0.4);
            self.ctx.rect(x, row_top - cell_h, cell_w, cell_h);

            self.ctx
                .no_stroke()
                .fill(ink())
                .font(&self.facts.family)
                .clear_font_variations()
                .font_variation("wght", 400.0)
                .font_size(glyph_size)
                .tracking(0.0)
                .auto_line_height()
                .text_align(TextAlign::Center);
            self.ctx.text(&ch.to_string(), cx, base);

            self.ctx
                .no_stroke()
                .font(MONO)
                .clear_font_variations()
                .fill(hair())
                .font_size(5.5)
                .text_align(TextAlign::Center);
            self.ctx.text(&format!("{:04X}", cp), cx, row_top - cell_h + 4.0);

            col += 1;
            if col >= cols {
                col = 0;
                row_top -= cell_h;
            }
        }
    }

    fn waterfall(&mut self) {
        self.new_sheet("Waterfall", "Waterfall");
        let sizes = [72.0, 54.0, 42.0, 32.0, 24.0, 18.0, 14.0, 12.0, 10.0, 9.0, 8.0];
        let sample = "Hamburgefonstiv";
        let mut y = H - 152.0;
        self.ctx
            .font(&self.facts.family)
            .clear_font_variations()
            .font_variation("wght", 400.0)
            .tracking(0.0)
            .auto_line_height()
            .text_align(TextAlign::Left)
            .no_stroke();
        for &s in &sizes {
            if y - s < M {
                break;
            }
            self.ctx.font(MONO).fill(faint()).font_size(8.0);
            self.ctx.text(&format!("{}", s as i64), M, y);
            self.ctx.font(&self.facts.family).fill(ink()).font_size(s);
            self.ctx.text(sample, M + 34.0, y);
            y -= s * 1.26 + 5.0;
        }
    }

    /// A single column of running SAMPLE text at a given size/leading/tracking,
    /// spanning `span` grid columns from column `start`, with a mono caption.
    fn text_column(&mut self, start: usize, span: usize, size: f64, leading: f64, track: f64) {
        let x = col_x(start);
        let w = span_w(span);
        let top = H - 100.0;
        let bh = top - M;

        // mono caption above the column
        let cap = format!(
            "{}/{}  ·  tracking {:+.1}",
            num(size as f32),
            num(leading as f32),
            track
        );
        self.ctx
            .no_stroke()
            .font(MONO)
            .clear_font_variations()
            .fill(faint())
            .font_size(6.5)
            .tracking(0.4)
            .auto_line_height()
            .text_align(TextAlign::Left);
        self.ctx.text(&cap.to_uppercase(), x, top + 12.0);

        // the text block
        self.ctx
            .fill(ink())
            .font(&self.facts.family)
            .clear_font_variations()
            .font_variation("wght", 400.0)
            .font_size(size)
            .line_height(leading)
            .tracking(track)
            .text_align(TextAlign::Left);
        self.ctx.text_box(&filled(1400), x, M, w, bh);
    }

    /// Three columns at ascending reading sizes with matched leading.
    fn text_sizes(&mut self) {
        self.new_sheet("Text · Reading Sizes", "Text — Reading Sizes");
        self.text_column(0, 2, 8.5, 12.0, 0.0);
        self.text_column(2, 2, 10.0, 14.5, 0.0);
        self.text_column(4, 2, 12.5, 17.5, 0.0);
    }

    /// Same size, three leadings — the effect of line spacing on color.
    fn text_leading(&mut self) {
        self.new_sheet("Text · Leading", "Text — Leading");
        self.text_column(0, 2, 10.0, 12.0, 0.0);
        self.text_column(2, 2, 10.0, 14.5, 0.0);
        self.text_column(4, 2, 10.0, 17.5, 0.0);
    }

    /// Same size/leading, three tracking values — tighter to looser.
    fn text_tracking(&mut self) {
        self.new_sheet("Text · Tracking", "Text — Tracking");
        self.text_column(0, 2, 10.5, 15.0, -0.4);
        self.text_column(2, 2, 10.5, 15.0, 0.0);
        self.text_column(4, 2, 10.5, 15.0, 0.6);
    }
}

/// Generate the default print proof for `font_path`, writing a PDF to
/// `output_path`. `grid` overlays the Swiss guide grid on every page.
pub fn generate_proof(
    font_path: &Path,
    output_path: &str,
    grid: bool,
) -> Result<(), DesignBotError> {
    let data = std::fs::read(font_path).map_err(DesignBotError::IOError)?;
    let facts = introspect(&data)?;

    let mut r = Renderer::new(W as u32, H as u32);
    r.load_font(font_path)?;
    r.load_font_data(MONO_TTF.to_vec());

    let mut proof = Proof {
        ctx: Canvas::new(W, H),
        facts: &facts,
        date: today(),
        git: git_hash(font_path),
        folio: 1,
        grid,
    };
    proof.cover();
    proof.char_set();
    proof.waterfall();
    proof.text_sizes();
    proof.text_leading();
    proof.text_tracking();

    r.render_to_pdf(&proof.ctx, output_path)?;
    Ok(())
}
