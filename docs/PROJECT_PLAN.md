# DesignBot Project Plan

## Overview

DesignBot is a Rust-based 2D graphics generation tool inspired by DrawBot. It enables users to create vector graphics, images, and animations through Rust code, providing a powerful and type-safe alternative to Python-based DrawBot.

### Core Philosophy

- **Rust-first design**: Native Rust API that feels natural to Rust developers
- **Self-contained**: Zero system dependencies - all rendering capabilities built into the CLI
- **Linebender ecosystem**: Leverage high-performance Linebender crates for rendering
- **Desktop-ready architecture**: Designed for future GUI application integration
- **Educational**: Simple, intuitive API suitable for teaching graphics programming

## Target Usage

```bash
# Basic usage
designbot --render design.rs --output design.png

# Additional formats
designbot --render design.rs --output design.pdf
designbot --render design.rs --output design.svg

# Animations
designbot --render animation.rs --output animation.gif
designbot --render animation.rs --output animation.mp4
```

## Design Script Format

Users write Rust code that uses the DesignBot API. Example:

```rust
use designbot::prelude::*;

fn main() {
    canvas(800, 600);

    fill(rgb(242, 140, 168));
    rect(100, 100, 200, 150);

    fill(rgb(100, 200, 255));
    oval(300, 200, 150, 150);

    save("output.png");
}
```

## Core Architecture

### 1. Rendering Pipeline

```
User Script (.rs) -> Compile & Execute -> Scene Builder -> Vello Renderer -> Output (PNG/PDF/SVG)
```

### 2. Key Components

#### a. **CLI Application** (`designbot-cli`)
- Argument parsing (clap)
- Script compilation and execution
- Output format handling
- Error reporting

#### b. **Core Library** (`designbot`)
- Drawing API (DrawBot-inspired interface)
- Scene graph management
- State management (fill, stroke, transforms)
- Path building and manipulation

#### c. **Rendering Backend** (`designbot-render`)
- Vello integration for GPU rendering
- Software fallback for headless environments
- Image encoding (PNG, JPEG, TIFF)
- Vector output (SVG, PDF)

#### d. **Script Runtime** (`designbot-script`)
- Dynamic library loading
- API bindings
- Error handling and sandboxing

### 3. Module Structure

```
designbot/
├── designbot/           # Core library
│   ├── canvas.rs       # Canvas management
│   ├── shapes.rs       # Basic shapes (rect, oval, polygon)
│   ├── path.rs         # Bezier paths
│   ├── text.rs         # Text rendering
│   ├── color.rs        # Color management
│   ├── transform.rs    # Transformations (rotate, scale, translate)
│   ├── gradient.rs     # Gradient support
│   ├── image.rs        # Image placement
│   ├── state.rs        # Graphics state stack
│   └── prelude.rs      # Common imports
│
├── designbot-render/   # Rendering backend
│   ├── vello_backend.rs
│   ├── image_output.rs
│   ├── svg_output.rs
│   └── pdf_output.rs
│
├── designbot-cli/      # CLI application
│   └── main.rs
│
└── examples/           # Example scripts
    ├── basic_shapes.rs
    ├── text_example.rs
    ├── bezier_paths.rs
    └── animation.rs
```

## Technology Stack

### Linebender Ecosystem

1. **Vello** - GPU-accelerated 2D rendering
   - Primary rendering engine
   - High-performance scene rendering
   - PostScript-inspired API

2. **Kurbo** - 2D curves and paths
   - Bezier curve manipulation
   - Path operations
   - Geometric primitives

3. **Peniko** - Styling primitives
   - Fill and stroke styles
   - Color management
   - Brush definitions

4. **Parley** - Rich text layout
   - Font handling
   - Text shaping and layout
   - Unicode support

### Additional Dependencies

- **wgpu** - GPU access (via Vello)
- **image** - Raster image encoding/decoding
- **clap** - CLI argument parsing
- **anyhow** - Error handling
- **thiserror** - Custom error types

### Output Format Libraries

- **svg** or **resvg** - SVG generation/rendering
- **printpdf** or **pdf-writer** - PDF generation
- **image** - PNG, JPEG, TIFF encoding

## API Design

### DrawBot Parity

DesignBot aims to provide similar functionality to DrawBot with a Rust-native API:

#### Canvas Management
```rust
canvas(width: u32, height: u32) -> Canvas
new_page() -> Page
width() -> f64
height() -> f64
page_count() -> usize
save(path: &str) -> Result<()>
```

#### Drawing Primitives
```rust
rect(x: f64, y: f64, width: f64, height: f64)
oval(x: f64, y: f64, width: f64, height: f64)
circle(x: f64, y: f64, radius: f64)
line(x1: f64, y1: f64, x2: f64, y2: f64)
polygon(points: &[(f64, f64)], close: bool)
```

#### Path Operations
```rust
new_path() -> Path
move_to(x: f64, y: f64)
line_to(x: f64, y: f64)
curve_to(cp1x: f64, cp1y: f64, cp2x: f64, cp2y: f64, x: f64, y: f64)
close_path()
draw_path()
```

#### Color and Style
```rust
fill(color: Color)
stroke(color: Color)
stroke_width(width: f64)
no_fill()
no_stroke()

// Color constructors
rgb(r: u8, g: u8, b: u8) -> Color
rgba(r: u8, g: u8, b: u8, a: u8) -> Color
hex(hex: &str) -> Color
```

#### Gradients
```rust
linear_gradient(
    start: (f64, f64),
    end: (f64, f64),
    stops: &[(f64, Color)]
) -> Gradient

radial_gradient(
    center: (f64, f64),
    radius: f64,
    stops: &[(f64, Color)]
) -> Gradient
```

#### Text
```rust
font(name: &str, size: f64)
font_size(size: f64)
text(string: &str, x: f64, y: f64)
text_box(string: &str, x: f64, y: f64, width: f64, height: f64)
text_size(string: &str) -> (f64, f64)
```

#### Transformations
```rust
save()           // Push state
restore()        // Pop state
translate(x: f64, y: f64)
rotate(degrees: f64)
scale(factor: f64)
scale_xy(sx: f64, sy: f64)
skew(angle_x: f64, angle_y: f64)
```

#### Image Placement
```rust
place_image(path: &str, x: f64, y: f64, alpha: f64)
```

### Rust-Specific Enhancements

#### Builder Pattern
```rust
Canvas::new(800, 600)
    .fill(rgb(255, 0, 0))
    .rect(10, 10, 100, 100)
    .fill(rgb(0, 0, 255))
    .oval(200, 200, 50, 50)
    .save("output.png")?;
```

#### Type Safety
```rust
pub struct Color { /* ... */ }
pub struct Point { x: f64, y: f64 }
pub struct Size { width: f64, height: f64 }
pub struct Rect { origin: Point, size: Size }
```

#### Error Handling
```rust
pub enum DesignBotError {
    RenderError(String),
    IOError(std::io::Error),
    FontError(String),
    InvalidColor(String),
}

pub type Result<T> = std::result::Result<T, DesignBotError>;
```

## Implementation Phases

### Phase 1: Foundation (Weeks 1-2)
**Goal**: Basic CLI and rendering infrastructure

- [ ] Project structure and workspace setup
- [ ] CLI argument parsing
- [ ] Basic Vello renderer integration
- [ ] PNG output support
- [ ] Simple script execution system
- [ ] Basic canvas management

**Deliverable**: CLI that can render a simple colored rectangle to PNG

### Phase 2: Core Drawing API (Weeks 3-4)
**Goal**: Essential drawing primitives

- [ ] Shape primitives (rect, oval, circle, line, polygon)
- [ ] Fill and stroke colors
- [ ] Path operations (move_to, line_to, curve_to, close_path)
- [ ] Graphics state stack (save/restore)
- [ ] Basic transformations (translate, rotate, scale)

**Deliverable**: Full shape drawing capability with colors and basic transforms

### Phase 3: Advanced Graphics (Weeks 5-6)
**Goal**: Professional graphics features

- [ ] Gradient support (linear and radial)
- [ ] Image placement
- [ ] Transparency and blending modes
- [ ] Clipping paths
- [ ] Advanced path operations

**Deliverable**: Complex graphics with gradients, images, and clipping

### Phase 4: Text Rendering (Week 7)
**Goal**: Rich text support

- [ ] Font loading and management (via Parley)
- [ ] Text rendering (text, text_box)
- [ ] Font sizing and metrics
- [ ] Text alignment options

**Deliverable**: Text rendering with font support

### Phase 5: Output Formats (Week 8)
**Goal**: Multiple export formats

- [ ] SVG output
- [ ] PDF output
- [ ] JPEG/TIFF support
- [ ] Multi-page document support

**Deliverable**: Export to SVG, PDF, and multiple raster formats

### Phase 6: Animation Support (Week 9)
**Goal**: Frame-based animations

- [ ] Frame management
- [ ] GIF output
- [ ] MP4/video output
- [ ] Animation helpers (frame interpolation)

**Deliverable**: Basic animation capabilities

### Phase 7: Polish & Documentation (Week 10)
**Goal**: Production-ready release

- [ ] Comprehensive examples
- [ ] API documentation
- [ ] Error messages and debugging
- [ ] Performance optimization
- [ ] Testing suite

**Deliverable**: v0.1.0 release

## Future Enhancements

### Desktop Application
- Native GUI application (using Xilem or other Rust GUI framework)
- Interactive code editor
- Live preview
- Debugging tools

### Advanced Features
- 3D transformations
- Filters and effects (blur, shadow, etc.)
- Layer management
- Export presets
- Color palette management
- Variable/dynamic graphics

### Performance
- Parallel rendering
- Caching and optimization
- Headless server mode
- Cloud rendering API

## Technical Challenges

### 1. Script Execution Model

**Challenge**: How to safely execute user-provided Rust code

**Options**:
- **Dynamic Library Loading**: Compile user script to `.so`/`.dylib`/`.dll` and load at runtime
- **Proc Macro DSL**: Provide a DSL that compiles to safe API calls
- **Embedded Scripting**: Use Rhai or similar for script language

**Recommended**: Dynamic library loading with well-defined API boundary

### 2. Headless Rendering

**Challenge**: Vello requires wgpu/GPU access

**Solution**:
- wgpu supports software adapters
- Fallback to CPU rendering if GPU unavailable
- Consider software-only rendering backend for CI/CD

### 3. PDF/SVG Generation

**Challenge**: Vello renders to GPU textures, not vector formats

**Solution**:
- Maintain scene graph representation
- Implement separate SVG/PDF exporters that traverse scene graph
- Use Vello for raster output, direct path-to-vector for PDF/SVG

### 4. Font Handling

**Challenge**: Cross-platform font discovery and loading

**Solution**:
- Use Parley's font enumeration
- Bundle common fonts with CLI
- Support custom font directories
- Font fallback system

## Success Metrics

- **Functional parity**: 80% of DrawBot API covered
- **Performance**: Render 1000x1000px canvas in <100ms
- **Usability**: Clear error messages and documentation
- **Portability**: Works on Linux, macOS, Windows
- **Community**: Example gallery, active issue discussions

## Open Questions

1. Should we support a simpler DSL instead of full Rust syntax?
2. How to handle animated GIFs with Vello's GPU rendering?
3. Bundle fonts or require system fonts?
4. Support WebAssembly compilation for web usage?
5. Include a template/scaffolding command?

## References

- [DrawBot Documentation](https://www.drawbot.com/)
- [DrawBot GitHub](https://github.com/typemytype/drawbot)
- [Vello](https://github.com/linebender/vello)
- [Kurbo](https://github.com/linebender/kurbo)
- [Parley](https://github.com/linebender/parley)
- [Peniko](https://github.com/linebender/peniko)
