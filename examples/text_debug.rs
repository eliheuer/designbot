use designbot::prelude::*;

fn main() {
    // Create a small canvas
    let mut ctx = Canvas::new(500.0, 200.0);

    // Set white background
    ctx.background(Color::rgb(255, 255, 255));

    // Simple text
    ctx.fill(Color::rgb(0, 0, 0));
    ctx.font_size(48.0);
    ctx.text("ABC", 50.0, 100.0);

    // Render to PNG
    let output_path = "text_debug.png";
    let renderer = Renderer::new(500, 200);
    renderer.render_to_png(&ctx, output_path).unwrap();

    println!("Rendered {}", output_path);
}
