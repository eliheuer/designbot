# DesignBot

A Rust-based 2D graphics generation tool inspired by [DrawBot](https://www.drawbot.com/) and [Processing](https://processing.org/). Create vector graphics, images, and animations through Rust code. This should be a fun way for type designers, graphic designers, artists, and students to learn [Rust](https://rust-lang.org/) programming. 

<img width="3840" height="2160" alt="A Virtua Grotesk type specimen drawn with designbot, shown next to the Rust code that draws it" src="https://github.com/user-attachments/assets/0e0f71ad-e87c-46fc-94dc-2b6c32b07944" />

## Features

- 🎨 **DrawBot-inspired API** - Familiar drawing primitives (rect, oval, line, polygon, etc)
- 🎯 **Self-contained** - Zero system dependencies
- 📦 **Multiple output formats** - PNG, GIF, MP4, PDF, and SVG
- 🎬 **Animation** - Multi-page timelines exported to GIF or MP4

## Installation

### Recommended: Install from Git

This is the easiest way to install and ensures you get the latest version:

```bash
cargo install --git https://github.com/eliheuer/designbot designbot-cli
```

### For Development: Install from Local Source

If you're contributing to DesignBot or testing local changes:

```bash
# Clone the repository
git clone https://github.com/eliheuer/designbot
cd designbot

# Install the CLI (will use git dependencies for user scripts)
cargo install --path designbot-cli --force
```

**Important:** After making local changes, you must push them to GitHub's `main` branch before the installed CLI can use them when compiling user scripts.

After installation, the `designbot` command will be available in your terminal (usually in `~/.cargo/bin/designbot`).

### Performance Notes

**First run:** The CLI will compile dependencies (takes ~1-2 minutes)
**Subsequent runs:** Uses a persistent cache at `~/.designbot/cache` (~0.5 seconds)

To clear the cache if needed:
```bash
rm -rf ~/.designbot/cache
```

## Quick Start

### Using the CLI

```bash
# Create a simple design script
designbot --render my_design.rs --output my_design.png
```

### Testing Fonts

Perfect for type designers testing fonts:

```rust
// font_test.rs
use designbot::prelude::*;

fn main() {
    let mut ctx = Canvas::new(1200.0, 800.0);
    let mut renderer = Renderer::new(1200, 800);

    // Load your font
    renderer.load_font("fonts/MyFont-Regular.ttf").unwrap();

    ctx.background(Color::rgb(255, 255, 255));
    ctx.fill(Color::rgb(0, 0, 0));
    ctx.font("MyFont");

    // Test different sizes (origin is bottom-left, y-up, like DrawBot;
    // text() puts the baseline at y)
    ctx.font_size(72.0);
    ctx.text("The quick brown fox", 50.0, 500.0);

    ctx.font_size(48.0);
    ctx.text("jumps over the lazy dog", 50.0, 400.0);

    ctx.font_size(24.0);
    ctx.text("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 50.0, 300.0);
    ctx.text("abcdefghijklmnopqrstuvwxyz", 50.0, 250.0);
    ctx.text("0123456789 !@#$%^&*()", 50.0, 200.0);

    renderer.render_to_png(&ctx, "font_test.png").unwrap();
}
```

Run with:
```bash
designbot --render font_test.rs --output font_test.png
```

### Using Examples (For Development)

Examples output to the current directory:

```bash
# Run from the designbot git repo
cd designbot
cargo run --example basic_shapes
```

This generates `basic_shapes.png` in your current directory.

## Usage Examples

### Simple Script (CLI Auto-wraps)

Create a file `my_design.rs`:

```rust
use designbot::prelude::*;

// Just drawing commands - no main function needed
ctx.fill(Color::rgb(255, 100, 100));
ctx.rect(100.0, 100.0, 400.0, 400.0);

ctx.fill(Color::rgb(100, 255, 100));
ctx.oval(200.0, 200.0, 200.0, 200.0);
```

Run with:
```bash
designbot --render my_design.rs --output my_design.png
```

### Full Script (Complete Control)

Create a file `custom_design.rs`:

```rust
use designbot::prelude::*;

fn main() {
    let mut ctx = Canvas::new(800.0, 600.0);

    // Draw shapes
    ctx.fill(Color::rgb(255, 200, 100));
    ctx.rect(100.0, 100.0, 600.0, 400.0);

    // Render
    let renderer = Renderer::new(800, 600);
    renderer.render_to_png(&ctx, "output.png").unwrap();
}
```

Run with:
```bash
designbot --render custom_design.rs --output custom.png
```

### As a Library (Example Files)

Create examples in `examples/` directory:

```rust
// examples/my_example.rs
use designbot::prelude::*;

fn main() {
    let mut ctx = Canvas::new(600.0, 600.0);

    ctx.fill(Color::rgb(100, 200, 255));
    ctx.oval(100.0, 100.0, 400.0, 400.0);

    // Output to current directory
    let renderer = Renderer::new(600, 600);
    renderer.render_to_png(&ctx, "my_example.png").unwrap();

    println!("Rendered my_example.png");
}
```

Run with:
```bash
cargo run --example my_example
# Generates my_example.png in current directory
```

## API Overview

### Canvas Management
```rust
let mut ctx = Canvas::new(800.0, 600.0);
```

### Drawing Primitives

Coordinates are DrawBot's: the origin is the **bottom-left** corner and y
increases **upward**. `rect`, `oval`, and `image` anchor at their
bottom-left corner.

```rust
ctx.rect(x, y, width, height);
ctx.oval(x, y, width, height);
ctx.line(x1, y1, x2, y2);
ctx.polygon(&[(x1, y1), (x2, y2), ...], close);
```

### Colors and Styling
```rust
ctx.fill(Color::rgb(255, 0, 0));
ctx.stroke(Color::black());
ctx.stroke_width(2.0);
ctx.no_fill();
ctx.no_stroke();
```

### Transformations
```rust
ctx.save();           // Push state
ctx.translate(x, y);
ctx.rotate(degrees);
ctx.scale(factor);
ctx.restore();        // Pop state
```

### Text Rendering
```rust
// Set font and size
ctx.font("Arial");
ctx.font_size(48.0);
ctx.fill(Color::black());

// Draw text with the first line's baseline at (x, y), like DrawBot
ctx.text("Hello World", 100.0, 100.0);

// Word-wrapped text in a box anchored at its bottom-left corner;
// text fills from the top of the box down
ctx.text_box("Long text that wraps...", 100.0, 200.0, 400.0, 200.0);
```

### Custom Font Loading
```rust
// Load custom fonts from files
let mut renderer = Renderer::new(800, 600);
renderer.load_font("fonts/MyFont-Regular.ttf").unwrap();
renderer.load_font("fonts/MyFont-Bold.ttf").unwrap();

// Use the loaded fonts
ctx.font("MyFont");
ctx.text("Custom Font", 100.0, 100.0);

// System fonts still work too
ctx.font("Arial");
ctx.text("System Font", 100.0, 200.0);
```

### Rendering
```rust
let renderer = Renderer::new(width, height);
renderer.render_to_png(&ctx, "output.png").unwrap();
```

### Social media export

Social platforms strip ICC color profiles (assuming sRGB) and re-encode
images to 4:2:0-subsampled JPEG, which halves color resolution: thin
saturated lines keep their brightness but lose their color, and dark
flats band and block up. The social PNG export compensates up front: it
writes an explicit sRGB chunk, pre-boosts saturation 12% around Rec. 709
luma, lays down coarse deterministic film grain that masks banding and
survives recompression, and knocks one corner pixel to 99% alpha, which
makes X/Twitter keep the upload as lossless PNG:

```rust
renderer.render_to_png_social(&ctx, "output.png").unwrap();
```

Or from the CLI, applied to any script's PNG output:

```bash
designbot --render my_design.rs --output my_design.png --social
```

## Project Structure

```
designbot/
├── designbot/           # Core library (Canvas, Colors, Shapes)
├── designbot-render/    # Rendering backend (Vello integration)
├── designbot-cli/       # CLI application
├── examples/            # Example scripts with outputs
└── docs/                # Documentation and project plan
```

## Technology Stack

- **[vello_cpu](https://github.com/linebender/vello)** - CPU-based 2D rendering (faster for reading back to CPU memory)
- **[AnyRender](https://github.com/DioxusLabs/anyrender)** - Portable rendering abstraction across backends
- **[Kurbo](https://github.com/linebender/kurbo)** - 2D curves and paths (including native Ellipse type)
- **[Peniko](https://github.com/linebender/peniko)** - Styling primitives
- **[wgpu](https://wgpu.rs/)** - GPU access
- **[image](https://github.com/image-rs/image)** - Image encoding/decoding

## Current Status

✅ **Phase 1 Complete**: Basic rendering infrastructure
- [x] Project structure and workspace
- [x] CLI with script compilation
- [x] Vello renderer integration
- [x] PNG output
- [x] Canvas management

✅ **Phase 2 Complete**: Core drawing API
- [x] Shape primitives (rect, oval, line, polygon)
- [x] Fill and stroke colors
- [x] Graphics state stack (save/restore)
- [x] Basic transformations (translate, rotate, scale)
- [x] Text rendering with Parley
- [x] Custom font loading
- [ ] Path operations (bezier curves)

🔧 **Current Improvements**:
- Migrating to vello_cpu for better CPU readback performance
- Integrating AnyRender for portable rendering across backends
- Using Kurbo's native Ellipse type for oval rendering

🔜 **Coming Soon**:
- Gradients
- Image placement
- Advanced path operations

See [docs/PROJECT_PLAN.md](docs/PROJECT_PLAN.md) for the full roadmap.

## Development

```bash
# Run tests
cargo test

# Build all packages
cargo build --workspace

# Run clippy
cargo clippy --workspace

# Format code
cargo fmt --all
```

## Examples

Check out the `examples/` directory for more:

- `basic_shapes.rs` - All primitive shapes (rect, oval, line, polygon)
- `basic_text.rs` - Text rendering and custom font loading
- `grid.rs` - Using state transformations for layout
- More examples coming soon!

### Font Setup for Examples

To run the text examples with custom fonts:

1. Create `examples/fonts/` directory (already exists)
2. Download fonts (e.g., [Inter](https://rsms.me/inter/)) and place `.ttf` files in the directory
3. See `examples/fonts/README.md` for detailed setup instructions

The examples will fall back to system fonts if custom fonts aren't found.

## License

Apache-2.0

## Acknowledgments

Inspired by [DrawBot](https://www.drawbot.com/) by Just van Rossum and Frederik Berlaen.
Built on crates from the [Linebender](https://github.com/linebender) ecosystem.
