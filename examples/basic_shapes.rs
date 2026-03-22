use designbot::prelude::*;

fn main() {
    // Create a 1000x1000 canvas
    let mut ctx = Canvas::new(1000.0, 1000.0);

    // Set dark gray background
    ctx.background(Color::rgb(10, 10, 10));

    // Example 1: Rectangle
    ctx.fill(Color::rgb(100, 100, 100));
    ctx.stroke(Color::rgb(220, 220, 220));
    ctx.stroke_width(2.0);
    ctx.rect(100.0, 100.0, 100.0, 100.0);

    // Example 2: Oval
    ctx.no_fill();
    ctx.stroke(Color::rgb(220, 220, 220));
    ctx.stroke_width(2.0);
    ctx.oval(100.0, 100.0, 800.0, 800.0);

    // Example 3: Line
    ctx.stroke(Color::rgb(220, 220, 220));
    ctx.stroke_width(2.0);
    ctx.line(100.0, 100.0, 900.0, 900.0);

    // Example 4: Polygon
    ctx.fill(Color::rgb(100, 100, 100));
    ctx.stroke(Color::rgb(220, 220, 220));
    ctx.stroke_width(2.0);
    ctx.polygon(
        &[
            (10.0, 10.0),
            (100.0, 900.0),
            (900.0, 500.0),
            (200.0, 800.0),
        ],
        true, // close
    );

    // Render to PNG in current directory
    let output_path = "basic_shapes.png";
    let renderer = Renderer::new(1000, 1000);
    renderer.render_to_png(&ctx, output_path).unwrap();

    println!("Rendered {}", output_path);
}
