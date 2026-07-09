use designbot::prelude::*;

fn main() {
    // Create a 1200x1600 canvas
    let mut ctx = Canvas::new(1200.0, 1600.0);

    // Set white background
    ctx.background(Color::rgb(255, 255, 255));

    // Create renderer and load custom fonts
    let mut renderer = Renderer::new(1200, 1600);

    // Load fonts from the examples/fonts directory
    if let Err(e) = renderer.load_font("fonts/BezyGrotesk-Regular.ttf") {
        eprintln!("Warning: Could not load BezyGrotesk-Regular.ttf: {}", e);
        eprintln!("The example will fall back to system fonts.");
        eprintln!("See examples/fonts/README.md for setup instructions.");
    }
    ctx.font("Bezy Grotesk");

    // Title
    ctx.fill(Color::rgb(20, 20, 20));
    ctx.font_size(72.0);
    ctx.text_align(TextAlign::Center);
    ctx.text("Text Alignment Examples", 600.0, 1480.0);

    // Reset to default settings for examples
    ctx.font_size(48.0);
    ctx.fill(Color::rgb(40, 40, 40));

    // Helper function to draw a vertical guide line and label
    let draw_guide_line = |ctx: &mut Canvas, x: f64, y: f64, label: &str| {
        // Draw vertical guide line
        ctx.save();
        ctx.stroke(Color::rgb(200, 100, 100));
        ctx.stroke_width(2.0);
        ctx.line(x, y - 30.0, x, y + 70.0);
        ctx.restore();

        // Draw label above the guide line
        ctx.save();
        ctx.fill(Color::rgb(200, 100, 100));
        ctx.font_size(16.0);
        ctx.text_align(TextAlign::Center);
        ctx.text(label, x, y + 80.0);
        ctx.restore();
    };

    // Example 1: Left Alignment
    let y1 = 1330.0;
    draw_guide_line(&mut ctx, 100.0, y1, "anchor");
    ctx.text_align(TextAlign::Left);
    ctx.text("Left Aligned Text", 100.0, y1);

    // Example 2: Center Alignment
    let y2 = 1220.0;
    draw_guide_line(&mut ctx, 600.0, y2, "anchor");
    ctx.text_align(TextAlign::Center);
    ctx.text("Center Aligned Text", 600.0, y2);

    // Example 3: Right Alignment
    let y3 = 1110.0;
    draw_guide_line(&mut ctx, 1100.0, y3, "anchor");
    ctx.text_align(TextAlign::Right);
    ctx.text("Right Aligned Text", 1100.0, y3);

    // Example 4: Text Box with different alignments
    ctx.font_size(32.0);
    let box_x = 100.0;
    let box_y = 800.0;
    let box_width = 1000.0;
    let box_height = 200.0;
    let text_content = "This is a text box with word wrapping. Text boxes support all alignment options including left, center, right, and justified alignment.";

    // Draw box outline
    ctx.save();
    ctx.no_fill();
    ctx.stroke(Color::rgb(150, 150, 150));
    ctx.stroke_width(2.0);
    ctx.rect(box_x, box_y, box_width, box_height);
    ctx.restore();

    // Left aligned text box
    ctx.fill(Color::rgb(40, 40, 40));
    ctx.text_align(TextAlign::Left);
    ctx.text_box(text_content, box_x, box_y, box_width, box_height);

    // Label
    ctx.font_size(20.0);
    ctx.text_align(TextAlign::Left);
    ctx.text("Left Aligned Text Box", box_x, box_y + box_height + 14.0);

    // Example 5: Center aligned text box
    let box_y2 = 545.0;
    ctx.save();
    ctx.no_fill();
    ctx.stroke(Color::rgb(150, 150, 150));
    ctx.stroke_width(2.0);
    ctx.rect(box_x, box_y2, box_width, box_height);
    ctx.restore();

    ctx.font_size(32.0);
    ctx.fill(Color::rgb(40, 40, 40));
    ctx.text_align(TextAlign::Center);
    ctx.text_box(text_content, box_x, box_y2, box_width, box_height);

    // Label
    ctx.font_size(20.0);
    ctx.text_align(TextAlign::Left);
    ctx.text("Center Aligned Text Box", box_x, box_y2 + box_height + 14.0);

    // Example 6: Right aligned text box
    let box_y3 = 290.0;
    ctx.save();
    ctx.no_fill();
    ctx.stroke(Color::rgb(150, 150, 150));
    ctx.stroke_width(2.0);
    ctx.rect(box_x, box_y3, box_width, box_height);
    ctx.restore();

    ctx.font_size(32.0);
    ctx.fill(Color::rgb(40, 40, 40));
    ctx.text_align(TextAlign::Right);
    ctx.text_box(text_content, box_x, box_y3, box_width, box_height);

    // Label
    ctx.font_size(20.0);
    ctx.text_align(TextAlign::Left);
    ctx.text("Right Aligned Text Box", box_x, box_y3 + box_height + 14.0);

    // Example 7: Justified text box
    let box_y4 = 35.0;
    ctx.save();
    ctx.no_fill();
    ctx.stroke(Color::rgb(150, 150, 150));
    ctx.stroke_width(2.0);
    ctx.rect(box_x, box_y4, box_width, box_height);
    ctx.restore();

    ctx.font_size(32.0);
    ctx.fill(Color::rgb(40, 40, 40));
    ctx.text_align(TextAlign::Justified);
    ctx.text_box(text_content, box_x, box_y4, box_width, box_height);

    // Label
    ctx.font_size(20.0);
    ctx.text_align(TextAlign::Left);
    ctx.text("Justified Text Box", box_x, box_y4 + box_height + 14.0);

    // Render to PNG in current directory
    let output_path = "text_alignment.png";
    renderer.render_to_png(&ctx, output_path).unwrap();

    println!("Rendered {}", output_path);
}
