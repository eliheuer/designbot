# DesignBot

A Rust-based 2D graphics generation tool inspired by [DrawBot](https://www.drawbot.com/) and [Processing](https://processing.org/). Create vector graphics, images, and animations through Rust code. This should be a fun way for type designers, graphic designers, artists, and students to learn [Rust](https://rust-lang.org/) programming. 

<img width="3840" height="2160" alt="Image" src="https://github.com/user-attachments/assets/1c84c6ea-8603-439b-a3d8-6b9b209c9448" />

## Features

- 🎨 **DrawBot-inspired API** - Familiar drawing primitives (rect, oval, line, polygon, etc)
- 🎯 **Self-contained** - Zero system dependencies
- 📦 **Multiple output formats** - PNG (more coming: SVG, PDF, GIF, MP4)

## Installation

```bash
# Install from source
cargo install --path designbot-cli

# Or install from git (once published)
# cargo install --git https://github.com/eliheuer/designbot
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

### Rendering
```rust
let renderer = Renderer::new(width, height);
renderer.render_to_png(&ctx, "output.png").unwrap();
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

✅ **Phase 2 In Progress**: Core drawing API
- [x] Shape primitives (rect, oval, line, polygon)
- [x] Fill and stroke colors
- [x] Graphics state stack (save/restore)
- [x] Basic transformations (translate, rotate, scale)
- [ ] Path operations (bezier curves)

🔧 **Current Improvements**:
- Migrating to vello_cpu for better CPU readback performance
- Integrating AnyRender for portable rendering across backends
- Using Kurbo's native Ellipse type for oval rendering

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

Apache-2.0

## Acknowledgments

Inspired by [DrawBot](https://www.drawbot.com/) by Just van Rossum and Frederik Berlaen.
Built on crates from the [Linebender](https://github.com/linebender) ecosystem.
