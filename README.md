# DesignBot

A Rust-based 2D graphics generation tool inspired by [DrawBot](https://www.drawbot.com/). Create vector graphics, images, and animations through Rust code with GPU-accelerated rendering powered by the [Linebender](https://github.com/linebender) ecosystem.

## Features

- 🎨 **DrawBot-inspired API** - Familiar drawing primitives (rect, oval, line, polygon)
- ⚡ **GPU-accelerated** - Fast rendering using Vello
- 🦀 **Rust-native** - Type-safe API with excellent performance
- 🎯 **Self-contained** - Zero system dependencies
- 📦 **Multiple output formats** - PNG (more coming: SVG, PDF, GIF, MP4)

## Installation

```bash
# Install from source
cargo install --path designbot-cli

# Or install from git (once published)
# cargo install --git https://github.com/USER/designbot
```

## Quick Start

### Using the CLI

```bash
# Create a simple design script
designbot --render my_design.rs --output my_design.png
```

### Using Examples (For Development)

Examples output to the current directory:

```bash
cargo run --example basic_shapes
```

This generates `basic_shapes.png` (1000x1000px) in your current directory.

## Usage Examples

### Simple Script (CLI Auto-wraps)

Create a file `my_design.rs`:

```rust
// Just drawing commands - no main function needed
canvas.fill(Color::rgb(255, 100, 100));
canvas.rect(100.0, 100.0, 400.0, 400.0);

canvas.fill(Color::rgb(100, 255, 100));
canvas.oval(200.0, 200.0, 200.0, 200.0);
```

Run with:
```bash
designbot --render my_design.rs --output my_design.png
```

### Full Script (Complete Control)

Create a file `custom_design.rs`:

```rust
use designbot::{Canvas, Color};
use designbot_render::Renderer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut canvas = Canvas::new(800.0, 600.0);

    // Draw shapes
    canvas.fill(Color::rgb(255, 200, 100));
    canvas.rect(100.0, 100.0, 600.0, 400.0);

    // Render
    let renderer = Renderer::new(800, 600);
    renderer.render_to_png(&canvas, "output.png")?;

    Ok(())
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
use designbot::{Canvas, Color};
use designbot_render::Renderer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut canvas = Canvas::new(600.0, 600.0);

    canvas.fill(Color::rgb(100, 200, 255));
    canvas.oval(100.0, 100.0, 400.0, 400.0);

    // Output to current directory
    let renderer = Renderer::new(600, 600);
    renderer.render_to_png(&canvas, "my_example.png")?;

    println!("Rendered my_example.png");
    Ok(())
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
let mut canvas = Canvas::new(800.0, 600.0);
```

### Drawing Primitives
```rust
canvas.rect(x, y, width, height);
canvas.oval(x, y, width, height);
canvas.line(x1, y1, x2, y2);
canvas.polygon(&[(x1, y1), (x2, y2), ...], close);
```

### Colors and Styling
```rust
canvas.fill(Color::rgb(255, 0, 0));
canvas.stroke(Color::black());
canvas.stroke_width(2.0);
canvas.no_fill();
canvas.no_stroke();
```

### Transformations
```rust
canvas.save();           // Push state
canvas.translate(x, y);
canvas.rotate(degrees);
canvas.scale(factor);
canvas.restore();        // Pop state
```

### Rendering
```rust
let renderer = Renderer::new(width, height);
renderer.render_to_png(&canvas, "output.png")?;
```

## Project Structure

```
designbot/
├── designbot/           # Core library (Canvas, Colors, Shapes)
├── designbot-render/    # Rendering backend (Vello integration)
├── designbot-cli/       # CLI application
├── examples/            # Example scripts with outputs
└── docs/               # Documentation and project plan
```

## Technology Stack

- **[Vello](https://github.com/linebender/vello)** - GPU-accelerated 2D rendering
- **[Kurbo](https://github.com/linebender/kurbo)** - 2D curves and paths
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

✅ **Phase 2 In Progress**: Core drawing API
- [x] Shape primitives (rect, oval, line, polygon)
- [x] Fill and stroke colors
- [x] Graphics state stack (save/restore)
- [x] Basic transformations (translate, rotate, scale)
- [ ] Path operations (bezier curves)

🔜 **Coming Soon**:
- Text rendering (Parley integration)
- Gradients
- Image placement
- SVG/PDF output
- Animation support

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
- More examples coming soon!

## License

MIT OR Apache-2.0

## Acknowledgments

Inspired by [DrawBot](https://www.drawbot.com/) by Just van Rossum and Frederik Berlaen.
Built on the amazing [Linebender](https://github.com/linebender) ecosystem.
