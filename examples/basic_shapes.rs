use designbot::{Canvas, Color};
use designbot_render::Renderer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a 1000x1000 canvas (same size as DrawBot examples)
    let mut canvas = Canvas::new(100.0, 100.0);

    // Example 1: Rectangle
    // DrawBot: rect(100, 100, 800, 800)
    canvas.rect(100.0, 100.0, 100.0, 100.0);

    // Example 2: Oval
    // DrawBot: oval(100, 100, 800, 800)
    canvas.no_fill(); // Clear fill from previous shape
    canvas.stroke(Color::black());
    canvas.stroke_width(2.0);
    canvas.oval(100.0, 100.0, 800.0, 800.0);

    // Example 3: Line
    // DrawBot: stroke(0); line((100, 100), (900, 900))
    canvas.stroke(Color::black());
    canvas.stroke_width(2.0);
    canvas.line(100.0, 100.0, 900.0, 900.0);

    // Example 4: Polygon
    // DrawBot: polygon((100, 100), (100, 900), (900, 900), (200, 800), close=True)
    canvas.fill(Color::rgb(200, 200, 200));
    canvas.stroke(Color::black());
    canvas.stroke_width(2.0);
    canvas.polygon(
        &[
            (100.0, 100.0),
            (100.0, 900.0),
            (900.0, 900.0),
            (200.0, 800.0),
        ],
        true, // close
    );

    // Render to PNG in current directory
    let output_path = "basic_shapes.png";
    let renderer = Renderer::new(1000, 1000);
    renderer.render_to_png(&canvas, output_path)?;

    println!("Rendered {}", output_path);

    Ok(())
}
