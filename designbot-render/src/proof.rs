//! Built-in default **print proof** generator.
//!
//! `designbot proof <font> -o proof.pdf` introspects any font (axes, named
//! instances, charset, metrics, features) and emits a multi-page, color-managed
//! PDF proof — no per-repo script required. The proof is designed as a superset
//! by eye of Google Fonts' diffenator2 `proof` view, so a designer can confirm
//! everything the machines check.
//!
//! US Letter landscape (792 × 612 pt). This is phase 1: cover, character-set
//! grid, and waterfall. Spacing / figures / accents / kerning / weight /
//! interpolation / text / features pages follow (see
//! virtua-grotesk/documentation/proofs/PROOF_SPEC.md).

use crate::Renderer;
use designbot_core::{Canvas, Color, DesignBotError, TextAlign};
use std::path::Path;

// --- page geometry (US Letter landscape, points = px) ----------------------
const W: f64 = 792.0;
const H: f64 = 612.0;
const M: f64 = 54.0; // margin

fn ink() -> Color {
    Color::rgb(0x23, 0x23, 0x23)
}
fn paper() -> Color {
    Color::rgb(0xff, 0xff, 0xff)
}
fn cover_bg() -> Color {
    Color::rgb(0x92, 0x92, 0x8e)
}
fn rule() -> Color {
    Color::rgb(0xcc, 0xcc, 0xcc)
}
// Real gray values (the PDF backend ignores per-paint alpha, so these must be
// solid colors, not ink().with_alpha(...)).
fn faint() -> Color {
    Color::rgb(0x80, 0x80, 0x80) // running head, gutter labels
}
fn hair() -> Color {
    Color::rgb(0xa6, 0xa6, 0xa6) // tiny hex labels
}
fn hues() -> [Color; 6] {
    [
        Color::oklch(0.66, 0.175, 28.0),
        Color::oklch(0.74, 0.160, 52.0),
        Color::oklch(0.88, 0.160, 92.0),
        Color::oklch(0.67, 0.160, 159.0),
        Color::oklch(0.65, 0.160, 258.0),
        Color::oklch(0.65, 0.160, 302.0),
    ]
}

/// Format a float axis value without a trailing `.0` (400.0 -> "400").
fn num(v: f32) -> String {
    if v.fract().abs() < 1e-4 {
        format!("{}", v as i64)
    } else {
        format!("{:.2}", v)
    }
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

    // Names: prefer typographic (name ID 16) family over legacy (ID 1).
    let (mut family, mut family_legacy, mut version) = (None, None, None);
    for s in font.localized_strings() {
        let id = s.id();
        match id {
            StringId::TypographicFamily if family.is_none() => {
                family = Some(s.to_string())
            }
            StringId::Family if family_legacy.is_none() => {
                family_legacy = Some(s.to_string())
            }
            StringId::Version if version.is_none() => version = Some(s.to_string()),
            _ => {}
        }
    }
    let family = family.or(family_legacy).unwrap_or_else(|| "Unknown".into());
    // Version string is usually "Version 1.000"; keep just the number if present.
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

// --- proof builder ---------------------------------------------------------

struct Proof<'a> {
    ctx: Canvas,
    facts: &'a FontFacts,
    date: String,
    folio: usize,
}

impl<'a> Proof<'a> {
    /// Small self-identifying header on every interior page + a hairline rule.
    fn running_head(&mut self, section: &str) {
        let y = H - 38.0;
        let left = format!("{}  ·  {}", self.facts.family, section);
        let right = format!("{}   ·   {}", self.date, self.folio);
        self.ctx
            .no_stroke()
            .fill(faint())
            .font(&self.facts.family)
            .clear_font_variations()
            .font_size(8.5)
            .tracking(0.2)
            .auto_line_height()
            .text_align(TextAlign::Left);
        self.ctx.text(&left, M, y);
        self.ctx.text_align(TextAlign::Right);
        self.ctx.text(&right, W - M, y);
        self.ctx.stroke(rule()).stroke_width(0.5);
        self.ctx.line(M, y - 7.0, W - M, y - 7.0);
    }

    /// Page title under the running head (semibold).
    fn page_title(&mut self, title: &str) {
        self.ctx
            .no_stroke()
            .fill(ink())
            .font(&self.facts.family)
            .clear_font_variations()
            .font_variation("wght", 600.0)
            .font_size(20.0)
            .tracking(0.0)
            .auto_line_height()
            .text_align(TextAlign::Left);
        self.ctx.text(title, M, H - 74.0);
        self.ctx.clear_font_variations();
    }

    /// Start a fresh interior sheet (white, running head, title).
    fn new_sheet(&mut self, section: &str, title: &str) {
        self.folio += 1;
        self.ctx.new_page();
        self.ctx.background(paper());
        self.running_head(section);
        self.page_title(title);
    }

    // ---- pages ----

    fn cover(&mut self) {
        self.ctx.background(cover_bg());

        // Family name, large + bold.
        self.ctx
            .no_stroke()
            .fill(ink())
            .font(&self.facts.family)
            .clear_font_variations()
            .font_variation("wght", 700.0)
            .font_size(92.0)
            .tracking(-2.0)
            .auto_line_height()
            .text_align(TextAlign::Left);
        self.ctx.text(&self.facts.family, M, H - 190.0);

        // Subtitle.
        self.ctx
            .clear_font_variations()
            .font_variation("wght", 400.0)
            .font_size(22.0)
            .tracking(0.5);
        self.ctx.text("Print Proof", M, H - 226.0);

        // Six brand dots.
        for (i, c) in hues().iter().enumerate() {
            self.ctx.fill(*c).no_stroke();
            self.ctx.oval(M + i as f64 * 30.0, H - 268.0, 18.0, 18.0);
        }

        // Metadata block, bottom-left.
        let axes = self
            .facts
            .axes
            .iter()
            .map(|a| format!("{} {}–{} ({})", a.tag, num(a.min), num(a.max), num(a.default)))
            .collect::<Vec<_>>()
            .join("    ");
        let instances = self
            .facts
            .instances
            .iter()
            .map(|i| {
                let v = i
                    .values
                    .iter()
                    .map(|x| num(*x))
                    .collect::<Vec<_>>()
                    .join("/");
                if v.is_empty() {
                    i.name.clone()
                } else {
                    format!("{} {}", i.name, v)
                }
            })
            .collect::<Vec<_>>()
            .join(",  ");

        let mut lines: Vec<String> = Vec::new();
        if !axes.is_empty() {
            lines.push(format!("Axes        {axes}"));
        }
        if !instances.is_empty() {
            lines.push(format!("Instances   {instances}"));
        }
        lines.push(format!(
            "Character   {} glyphs · {} encoded · {} upm",
            self.facts.glyph_count, self.facts.encoded, self.facts.upm
        ));
        if !self.facts.features.is_empty() {
            lines.push(format!("Features    {}", self.facts.features.join(", ")));
        }
        if !self.facts.version.is_empty() {
            lines.push(format!("Version     {}", self.facts.version));
        }
        lines.push(format!("Generated   {}", self.date));

        self.ctx
            .fill(ink())
            .clear_font_variations()
            .font_variation("wght", 400.0)
            .font_size(12.0)
            .tracking(0.0)
            .auto_line_height()
            .text_align(TextAlign::Left);
        let mut y = M + (lines.len() as f64 - 1.0) * 18.0;
        for l in &lines {
            self.ctx.text(l, M, y);
            y -= 18.0;
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
            // size label in the gutter, baseline-aligned
            self.ctx.fill(faint()).font_size(8.0);
            self.ctx.text(&format!("{}", s as i64), M, y);
            // sample
            self.ctx.fill(ink()).font_size(s);
            self.ctx.text(sample, M + 34.0, y);
            y -= s * 1.26 + 5.0;
        }
    }

    fn glyph_grid(&mut self) {
        self.new_sheet("Character Set", "Character Set");
        let cell_w = 46.0;
        let cell_h = 58.0;
        let cols = (((W - 2.0 * M) / cell_w).floor() as usize).max(1);
        let top = H - 104.0;
        let bottom = M;
        let glyph_size = cell_h * 0.58;

        let mut col = 0usize;
        let mut row_top = top;
        // clone to avoid borrowing self.facts across &mut self page breaks
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

            // faint cell frame
            self.ctx.no_fill().stroke(rule()).stroke_width(0.4);
            self.ctx.rect(x, row_top - cell_h, cell_w, cell_h);

            // glyph
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

            // hex label
            self.ctx.fill(hair()).font_size(5.5);
            self.ctx.text(&format!("{:04X}", cp), cx, row_top - cell_h + 4.0);

            col += 1;
            if col >= cols {
                col = 0;
                row_top -= cell_h;
            }
        }
    }
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

/// Generate the default print proof for `font_path`, writing a PDF to
/// `output_path`. Introspects the font and lays out the proof pages; the PDF is
/// sRGB color-managed by the renderer's PDF backend.
pub fn generate_proof(font_path: &Path, output_path: &str) -> Result<(), DesignBotError> {
    let data = std::fs::read(font_path).map_err(DesignBotError::IOError)?;
    let facts = introspect(&data)?;

    let mut r = Renderer::new(W as u32, H as u32);
    r.load_font(font_path)?;

    let mut proof = Proof {
        ctx: Canvas::new(W, H),
        facts: &facts,
        date: today(),
        folio: 1,
    };
    proof.cover();
    proof.glyph_grid();
    proof.waterfall();

    r.render_to_pdf(&proof.ctx, output_path)?;
    Ok(())
}
