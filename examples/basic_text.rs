use designbot::prelude::*;

fn main() {
    // Create a 1000x1000 canvas called ctx
    let mut ctx = Canvas::new(1000.0, 1000.0);

    // Set light gray background
    ctx.background(Color::rgb(190, 190, 190));

    // Create renderer and load custom fonts from examples/fonts directory
    let mut renderer = Renderer::new(1000, 1000);

    // Load fonts from the examples/fonts directory
    // Note: You need to download fonts and place them in examples/fonts/
    // See examples/fonts/README.md for recommended fonts and instructions
    if let Err(e) = renderer.load_font("fonts/BezyGrotesk-Regular.ttf") {
        eprintln!("Warning: Could not load Inter-Regular.ttf: {}", e);
        eprintln!("The example will fall back to system fonts.");
        eprintln!("See examples/fonts/README.md for setup instructions.");
    }
    ctx.font("Bezy Grotesk");

    // Example 1: Simple text using custom font
    ctx.fill(Color::rgb(20, 20, 20));
    ctx.font_size(96.0);
    ctx.text("Hello DesignBot", 100.0, 150.0);

    // Example 2: Red Text
    ctx.fill(Color::rgb(200, 50, 50));
    ctx.font_size(128.0);
    ctx.text("Big Red Font", 95.0, 280.0);

    // Example 3: Blue Text
    ctx.fill(Color::rgb(50, 100, 200));
    ctx.font_size(64.0);
    ctx.text("Blue Font", 100.0, 450.0);

    // Example 4: Text box with wrapping
    ctx.fill(Color::rgb(200, 50, 50));
    ctx.font_size(32.0);
    let text_content = "This is a text box demonstration. The text will automatically wrap to fit within the specified width and height boundaries. This feature is similar to DrawBot's textBox function.";
    ctx.text_box(text_content, 100.0, 550.0, 800.0, 400.0);

    // Draw a rectangle to show the text box bounds
    ctx.no_fill();
    ctx.stroke(Color::rgb(50, 50, 50));
    ctx.stroke_width(4.0);
    ctx.rect(100.0, 550.0, 800.0, 400.0);

    // Render to PNG in current directory
    let output_path = "basic_text.png";
    renderer.render_to_png(&ctx, output_path).unwrap();

    println!("Rendered {}", output_path);
}
