use designbot::{Canvas, Color};
use designbot_render::Renderer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut canvas = Canvas::new(600.0, 600.0);

    // Draw a colorful pattern
    canvas.fill(Color::rgb(255, 100, 100));
    canvas.rect(50.0, 50.0, 200.0, 200.0);

    canvas.fill(Color::rgb(100, 255, 100));
    canvas.oval(250.0, 50.0, 200.0, 200.0);

    canvas.fill(Color::rgb(100, 100, 255));
    canvas.rect(150.0, 250.0, 200.0, 200.0);

    // Render to PNG
    let renderer = Renderer::new(600, 600);
    renderer.render_to_png(&canvas, "test_output.png")?;

    println!("Rendered test_output.png");

    Ok(())
}
