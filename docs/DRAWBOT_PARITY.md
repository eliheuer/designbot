# DrawBot Parity Roadmap

2026-07-07. designbot's goal: a **full DrawBot replacement**
(https://www.drawbot.com) in Rust — same creative-coding ergonomics, one
binary, no Python. This doc is the gap map and the execution checklist.
It supersedes the format/animation phases of `PROJECT_PLAN.md` (which it
agrees with) and reuses that doc's API names where they were already sketched.

**Policy: Linebender first.** Everything possible comes from the crates
already in the tree — kurbo, peniko, parley, swash, vello_cpu, anyrender.
Accepted non-Linebender exceptions: the `image` crate for PNG/GIF encoding
(already a dep) and an `ffmpeg` subprocess for MP4 (same approach DrawBot
takes). SVG export rides on kurbo's `BezPath::to_svg()`. If Linebender grows
an equivalent (e.g. a PDF/scene serializer), we switch.

## The big discovery (2026-07-07 survey)

The installed stack **already implements** most of what's missing — it just
isn't exposed by `Canvas`, and the renderer drops some of it on the floor:

| capability | status in stack | blocker in designbot |
|---|---|---|
| clip paths, blend modes, opacity layers | `PaintScene::push_layer/pop_layer`, real in vello_cpu backend | no Canvas API |
| gradients (linear/radial/sweep) | `peniko::Gradient` flows through `PaintRef` | `brush_to_color` collapses every non-solid brush to black (renderer.rs ~278–293) |
| stroke caps/joins/dashes/miter | `kurbo::Stroke` passes through whole | renderer rebuilds `Stroke::new(width)` discarding the rest (renderer.rs ~183); no Canvas API |
| box shadow | `draw_box_shadow` → `fill_blurred_rounded_rect` | no Canvas API |
| hinted/stroked/COLR text | `draw_glyphs` glyph runs | renderer manually outlines per-glyph via swash (also rebuilds `ScaleContext` per glyph — slow) |
| masks (image/alpha clipping) | `vello_cpu::Mask`, `push_mask_layer` | not exposed by anyrender wrapper; direct use needed |

So most parity items are: add `GraphicsState` fields + `DrawCommand` data +
`Canvas` methods, and delete the two lossy renderer shortcuts.

## Gap matrix (DrawBot API → designbot)

Legend: ✅ have · 🔶 partial · ❌ missing · — won't do (for now)

**Canvas & pages**: size ✅ (Canvas::new) · width/height ✅ · newPage ❌ →
`new_page()` · pageCount ❌ · frameDuration ❌ → `frame_duration(secs)` ·
saveImage ❌ (PNG only) → `--output` by extension: png ✅ / gif / mp4 / svg /
pdf (later).

**Shapes & paths**: rect/oval/line/polygon ✅ · newPath/moveTo/lineTo/curveTo/
closePath/drawPath ✅ via `kurbo::BezPath` + `draw_path` (idiomatic Rust
stands in for DrawBot's implicit path state) · arc/arcTo 🔶 (kurbo has
`Arc`; add helpers) · BezierPath object with primitives (rect/oval/text) and
boolean ops ❌ (kurbo shapes → BezPath covers primitives; booleans need
skia-pathops-like crate or Linebender `kurbo` additions — defer).

**Color/fill/stroke**: fill/stroke/none ✅ · strokeWidth ✅ · lineCap ❌ ·
lineJoin ❌ · lineDash ❌ · miterLimit ❌ (all four = expose the
`kurbo::Stroke` already carried) · linearGradient ❌ · radialGradient ❌
(peniko has both, + sweep as a bonus) · shadow ❌ (box-shadow primitive
exists; path shadows via blur layer later) · opacity/blendMode ❌
(push_layer) · clipPath ❌ (push_layer clip) · cmyk* — (print color
management out of scope for now) · hex/hsb color helpers ❌ (tiny).

**Transformations**: save/restore ✅ · translate/rotate/scale ✅ · skew ❌ ·
transform(6-tuple affine) ❌ (kurbo::Affine, trivial).

**Text**: font/fontSize ✅ · text/textBox ✅ · align ✅ · fontVariations ✅
(fixed 2026-07-07: normalized coords now reach the swash scaler, so
variations apply to OUTLINES, not just shaping/metrics) ·
listFontVariations ❌ · listNamedInstances ❌ (swash exposes both:
`FontRef::variations()` / `::instances()`) ·
textSize ❌ on Canvas (width-only exists on Renderer) · lineHeight ❌ ·
tracking ❌ · openTypeFeatures ❌ (all three are parley `StyleProperty`s,
just not wired) · FormattedString ❌ (parley ranged styles exist; needs API
design) · text stroke ❌ (backend `stroke_glyphs` exists) · hyphenation/
underline/strikethrough ❌ (parley has them).

**Images**: image_rgba ✅ (raw) · image(path) ❌ (decode via `image` crate →
image_rgba) · imageSize/imagePixelColor ❌ (trivial with `image`) ·
ImageObject filters — (defer).

**Misc**: randomSeed/random helpers ❌ (tiny; `fastrand` or std hash — decide)
· Variable() UI — (native-app concept; skip).

## Execution phases

### Phase A — Animation (the headline) — DONE 2026-07-07
- [x] `Page` model in Canvas: `new_page()`, `page_count()`,
      `frame_duration(secs)`; `commands()` stays = current page (back-compat)
- [x] Renderer: one `FontContext` + one `VelloCpuImageRenderer` reused across
      frames with `reset()` between (NOT auto-reset — survey §3);
      `render_frames() -> Vec<(RGBA buffer, duration)>`
- [x] `render_to_gif` (image crate `GifEncoder`, per-page delay, infinite loop)
- [x] `render_to_mp4` (ffmpeg rawvideo pipe, yuv420p, even-dimension pad)
- [x] Multi-page PNG: `name.png` → `name_0001.png` … per page
- [x] CLI: pick `render_to_{png,gif,mp4}` from `--output` extension —
      the SAME script exports any format (DrawBot saveImage-by-extension)
- [x] `examples/animation.rs`, registered in root Cargo.toml (48-frame
      orbit; verified: GIF 48 frames/40ms/loop, MP4 24fps yuv420p)
- [x] Unit tests: page snapshots, duration bookkeeping, stroke-style state

### Phase B — Stop dropping what the stack gives us (quick, high value)
- [x] Stroke styles: `line_cap/line_join/line_dash/miter_limit` on Canvas →
      full `kurbo::Stroke` through renderer (2026-07-07; DrawBot defaults:
      butt cap, miter join, miter limit 10)
- [ ] Gradients: `linear_gradient/radial_gradient` (PROJECT_PLAN names) →
      `peniko::Gradient` brush; delete `brush_to_color` black-collapse
- [ ] `clip_path(BezPath)` + `opacity(a)` + `blend_mode(m)` via
      push_layer/pop_layer tied to save/restore
- [ ] `skew(ax, ay)` + `transform([a,b,c,d,e,f])`
- [ ] `shadow(offset, blur, color)` (box-shadow primitive first; general
      path shadow via blurred layer later)

### Phase C — Text parity

Gaps confirmed in production by the virtua-grotesk proof port (2026-07-07),
which had to parse SFNT tables by hand and reverse-engineer parley's
baseline rounding — fixing these removes that fragility for every port:

- [ ] **Baseline-anchored `text()`** (or a documented baseline-offset
      accessor) — designbot y is top-of-line; drawbot ports need baselines
- [ ] `text_size()` on Canvas returning (w, h); fix Renderer width-only —
      and CACHE the FontContext (rebuilding per call is too slow for
      per-glyph loops)
- [ ] `line_height` / leading control (parley uses 1.0×size; drawbot uses
      the font default ~1.29×) — caused visible paragraph divergence
- [ ] Font metadata access: family name(s), cmap coverage, hhea metrics —
      scripts currently parse SFNT themselves
- [ ] `list_font_variations()` / `list_named_instances()` (axis + instance
      discovery via swash) and a named-instance setter
- [ ] `line_height`, `tracking`, `open_type_features` (parley StyleProperties)
- [ ] Switch glyph drawing to anyrender `draw_glyphs` (hinting, stroked text,
      COLR; kills the per-glyph `ScaleContext` rebuild). Also fixes two
      CORRECTNESS bugs found by the virtua-grotesk specimen port (2026-07-08):
      (a) glyph runs within a line all start at x=0 — the renderer drops each
      run's line offset, so multi-run lines (bidi, font fallback) collapse
      into overlap; (b) per-glyph x/y offsets are ignored — GPOS mark
      attachment is lost (Arabic marks)
- [ ] FormattedString-style ranged styling API (parley ranged_builder)

### Phase D — I/O + formats
- [ ] `image(path, x, y, alpha)` (decode → image_rgba) + `image_size`
- [ ] SVG export (kurbo `to_svg()` per command; text as paths first)
- [x] PDF export — DONE 2026-07-07: **vector** PDF via a `PaintScene`
      implementation writing PDF content streams (pdf-writer). Shapes, text
      (as outline paths), clip, full stroke styles; multi-page native; CLI
      routes `.pdf`. v1 gaps (warn, not fail): gradients→first stop, raster
      images skipped, layer alpha/blend ignored, shadows unblurred
- [x] FlateDecode compression of PDF content streams (2026-07-08; ~10×)
- [ ] PDF font embedding + text operators (instead of flattened outlines) —
      the remaining file-size gap vs skia PDFs (proof 811KB vs 90KB)
- [x] Per-script compile cache (concurrent renders don't clobber; 2026-07-07)

### Phase E — Ergonomics
- [ ] hex/hsb color helpers; `random_seed`/`random` helpers
- [ ] arc/arcTo path helpers; register `grid.rs` example
- [ ] README + docs refresh; DrawBot→designbot migration cheatsheet

## Compatibility notes

- designbot is **y-down, origin top-left** (Processing-style); DrawBot is
  y-up bottom-left. This is a deliberate divergence — document it in the
  cheatsheet rather than emulate.
- Rust scripts, not Python: the API mirrors DrawBot *semantics*, not its
  dynamic typing (e.g. `fill(None)` → `no_fill()`).
