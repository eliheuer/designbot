use designbot::prelude::*;

fn main() {
    // Configuration
    const WIDTH: f64 = 2000.0;
    const HEIGHT: f64 = 2000.0;
    const MARGIN: f64 = 100.0;
    const GRID_SIZE: usize = 10; // Number of grid cells (10x10)

    // Calculate grid dimensions
    let grid_width = WIDTH - (MARGIN * 2.0);
    let grid_height = HEIGHT - (MARGIN * 2.0);
    let cell_width = grid_width / GRID_SIZE as f64;
    let cell_height = grid_height / GRID_SIZE as f64;

    // Create canvas with black background
    let mut ctx = Canvas::new(WIDTH, HEIGHT);
    ctx.background(Color::black());

    // Set colors
    let white = Color::white();
    let light_gray = Color::rgb(50, 50, 50);

    // Draw vertical lines
    ctx.stroke(white).stroke_width(2.0);
    for i in 0..=GRID_SIZE {
        let x = MARGIN + (i as f64 * cell_width);
        ctx.line(x, MARGIN, x, HEIGHT - MARGIN);
    }

    // Draw horizontal lines
    for i in 0..=GRID_SIZE {
        let y = MARGIN + (i as f64 * cell_height);
        ctx.line(MARGIN, y, WIDTH - MARGIN, y);
    }

    // Draw a circle
    let oval_width = cell_width * 2.0;
    let oval_height = cell_height * 2.0;
    let oval_x = MARGIN + (cell_width * 4.0);
    let oval_y = MARGIN + (cell_height * 4.0);
    ctx.stroke(white).stroke_width(2.0).no_fill();
    ctx.oval(oval_x, oval_y, oval_width, oval_height);

    // Render
    let renderer = Renderer::new(WIDTH as u32, HEIGHT as u32);
    renderer
        .render_to_png(&ctx, "grid.png")
        .expect("Failed to render");
    println!("Rendered grid.png");
}
