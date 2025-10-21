# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

DesignBot is a Rust-based 2D graphics generation tool inspired by DrawBot and Processing. It provides a simple API for creating vector graphics, images, and animations through Rust code, targeting type designers, graphic designers, artists, and students learning Rust.

## Development Commands

### Building and Testing
```bash
# Build all workspace packages
cargo build --workspace

# Run tests
cargo test

# Run clippy for linting
cargo clippy --workspace

# Format code
cargo fmt --all
```

### Running Examples
Examples output PNG files to the current directory:
```bash
# Run any example from the workspace root
cargo run --example basic_shapes
cargo run --example basic_text
cargo run --example grid

# This generates <example_name>.png in the current directory
```

### CLI Development
```bash
# Build the CLI
cargo build -p designbot-cli

# Test CLI rendering (from workspace root)
cargo run -p designbot-cli -- --render path/to/script.rs --output output.png
```

### CLI Performance and Caching

The CLI uses a persistent cache directory at `~/.designbot/cache` to speed up compilation:

- **First run:** ~80 seconds (compiles all dependencies)
- **Subsequent runs:** ~0.6 seconds (uses cargo's incremental compilation)

The cache directory contains a Cargo project that's reused across all script executions. This provides a 100x+ speedup for repeated use.

To clear the cache:
```bash
rm -rf ~/.designbot/cache
```

## Architecture

### Workspace Structure

The project uses a Cargo workspace with three core packages:

1. **`designbot-core` (at `designbot/`)** - Core drawing API
   - Canvas management and drawing commands
   - Color system
   - Graphics state stack (save/restore, transformations)
   - Shape primitives (rect, oval, line, polygon)
   - Text rendering API

2. **`designbot-render` (at `designbot-render/`)** - Rendering backend
   - Vello CPU renderer integration via AnyRender abstraction
   - PNG output using the `image` crate
   - Text rendering using Parley for layout and Swash for glyph outlines
   - Handles conversion between kurbo versions (0.11 in designbot-core, 0.12 in renderer)

3. **`designbot-cli` (at `designbot-cli/`)** - CLI application
   - Compiles user scripts dynamically
   - Auto-wraps simple scripts (no main function) with canvas and renderer setup
   - Creates temporary Cargo projects to compile user code
   - Supports both development mode (local paths) and installed mode (git deps)

4. **Root package** - Convenience wrapper
   - Re-exports designbot-core and designbot-render
   - Contains examples

### Key Design Patterns

#### Command Pattern for Drawing
The Canvas struct doesn't render immediately. Instead, it accumulates `DrawCommand` enums that are executed later by the Renderer. This enables:
- Multiple output backends (PNG, SVG, PDF future)
- Deferred rendering
- Command inspection/debugging

```rust
pub enum DrawCommand {
    FillShape { shape, brush, transform },
    StrokeShape { shape, brush, stroke, transform },
    DrawText { text, x, y, font_family, font_size, brush, transform },
    DrawTextBox { text, x, y, width, height, font_family, font_size, brush, transform },
}
```

#### Graphics State Stack
Uses a state stack pattern (similar to HTML Canvas or Cairo) for managing fill color, stroke color, stroke width, transformations, and font settings. `save()` pushes current state, `restore()` pops it.

#### Builder Pattern
Canvas methods return `&mut Self` to enable method chaining:
```rust
ctx.fill(Color::rgb(255, 0, 0))
   .rect(10.0, 10.0, 100.0, 100.0)
   .stroke(Color::rgb(0, 0, 255))
   .oval(50.0, 50.0, 50.0, 50.0);
```

### Rendering Pipeline

```
User Script (.rs)
    ↓
Canvas (builds DrawCommand list)
    ↓
Renderer (AnyRender + vello_cpu backend)
    ↓
Image crate (PNG encoding)
    ↓
Output file
```

### Technology Stack

- **vello_cpu** (0.0.4) - CPU-based 2D rendering, optimized for reading back to CPU memory
- **AnyRender** (0.6.1) - Portable rendering abstraction, provides clean callback API
- **Kurbo** (0.12) - 2D curves and geometric primitives (Rect, Circle, Ellipse, BezPath)
- **Peniko** (0.5) - Styling primitives (Brush, Color, Stroke)
- **Parley** (0.2) - Rich text layout and font handling
- **Swash** (0.1) - Font glyph scaling and outline extraction

### Kurbo Version Split

The codebase currently uses two versions of Kurbo:
- **designbot-core**: kurbo 0.11 (used in Canvas API)
- **designbot-render**: kurbo 0.12 (required by vello_cpu and AnyRender)

The renderer includes conversion logic to translate between these versions. This is a temporary state during migration.

## CLI Script Execution Model

The CLI supports two modes:

### Simple Scripts (Auto-wrapped)
Scripts without a `main()` function or `render_to_png` call are automatically wrapped:
```rust
use designbot::prelude::*;

ctx.fill(Color::rgb(255, 100, 100));
ctx.rect(100.0, 100.0, 400.0, 400.0);
```

The CLI wraps this with canvas creation and rendering setup.

### Full Scripts (User-controlled)
Scripts with their own `main()` function have full control:
```rust
use designbot::prelude::*;

fn main() {
    let mut ctx = Canvas::new(800.0, 600.0);
    ctx.fill(Color::rgb(255, 0, 0));
    ctx.rect(10.0, 10.0, 100.0, 100.0);

    let renderer = Renderer::new(800, 600);
    renderer.render_to_png(&ctx, "output.png").unwrap();
}
```

The CLI compiles user scripts by:
1. Creating a temporary Cargo project in `/tmp/designbot-<pid>/`
2. Adding designbot dependencies (local paths in dev, git in production)
3. Running `cargo run --release --quiet`
4. Cleaning up temp directory on exit

## Text Rendering

Text rendering uses Parley for layout and Swash for glyph outline extraction:

1. **Layout**: Parley breaks text into lines, positions glyphs, handles font fallback
2. **Glyph outlines**: Swash scales font outlines to requested size
3. **Path conversion**: Swash paths (MoveTo, LineTo, QuadTo, CurveTo) → Kurbo BezPath
4. **Coordinate system**: Glyphs are Y-up, screen is Y-down, so apply `Affine::FLIP_Y`

### Text API

Two text functions:
- `text(text, x, y)` - Single-line text at position
- `text_box(text, x, y, width, height)` - Multi-line text with word wrapping

### Font Loading

Custom fonts can be loaded from files using the Renderer:

```rust
let mut renderer = Renderer::new(800, 600);

// Load custom fonts before rendering
renderer.load_font("fonts/MyFont-Regular.ttf").unwrap();
renderer.load_font("fonts/MyFont-Bold.ttf").unwrap();

// Use the fonts in your canvas
ctx.font("MyFont");
ctx.text("Custom Font", 100.0, 100.0);
```

**Implementation details:**
- Font data is stored in `Renderer.custom_fonts` as `Vec<Vec<u8>>`
- During `render_to_png`, a `FontContext` is created and custom fonts are registered via `font_cx.collection.register_fonts()`
- The FontContext is shared across all text rendering calls via `RefCell` for interior mutability
- System fonts remain available and are loaded by `FontContext::default()`
- Font loading is optional - system fonts will be used as fallback

## Development Patterns

### Adding New Shape Primitives

1. Add variant to `ShapeType` enum in `designbot/src/canvas.rs`
2. Add public method to `Canvas` that constructs the shape and calls `draw_shape()`
3. Add conversion logic in `designbot-render/src/renderer.rs::convert_shape()`

### Adding New DrawCommands

1. Add variant to `DrawCommand` enum in `designbot/src/canvas.rs`
2. Add Canvas method that pushes the command
3. Handle the command in `Renderer::render_command()`

### Coordinate System

- Origin (0, 0) is top-left
- X increases to the right
- Y increases downward
- Transformations (translate, rotate, scale) apply to subsequent drawing commands
- All angles are in degrees (converted to radians internally)

## Examples Setup

### Font Files for Examples

Examples use fonts from `examples/fonts/` directory to ensure consistent rendering across systems. The directory contains a README with setup instructions.

**To run examples with custom fonts:**
1. Download font files (e.g., Inter from https://rsms.me/inter/)
2. Place `.ttf` files in `examples/fonts/`
3. Examples will gracefully fall back to system fonts if files aren't found

**Example pattern:**
```rust
let mut renderer = Renderer::new(800, 600);

// Load fonts with error handling
if let Err(e) = renderer.load_font("examples/fonts/Inter-Regular.ttf") {
    eprintln!("Warning: {}", e);
    eprintln!("Falling back to system fonts");
}

// Use the fonts
ctx.font("Inter");
ctx.text("Hello", 100.0, 100.0);
```

## Current Limitations

- **Output formats**: Only PNG supported (SVG, PDF planned)
- **Gradients**: Not yet implemented
- **Image placement**: Not yet implemented
- **Bezier paths**: Not yet exposed in public API
- **Clipping**: Not yet implemented
- **Animations**: Not yet supported
- **Font formats**: Only TrueType (.ttf) and OpenType (.otf) supported
