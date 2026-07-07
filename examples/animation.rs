// Multi-page animation: an orbiting square on a dashed track.
//
//   cargo run --example animation            -> animation.gif
//   designbot --render examples/animation.rs --output orbit.gif
//   designbot --render examples/animation.rs --output orbit.mp4  (needs ffmpeg)
//
// Every page created with `new_page()` becomes one frame; `frame_duration`
// sets the per-frame delay (DrawBot's newPage/frameDuration model).

use designbot::prelude::*;

fn main() {
    let (w, h) = (480.0, 480.0);
    let mut ctx = Canvas::new(w, h);
    ctx.frame_duration(1.0 / 24.0);

    let frames = 48;
    for i in 0..frames {
        if i > 0 {
            ctx.new_page();
        }
        let t = i as f64 / frames as f64;
        ctx.background(Color::rgb(32, 32, 32));

        // Dashed orbit track (line_dash / stroke styling)
        ctx.no_fill();
        ctx.stroke(Color::rgb(24, 184, 111));
        ctx.stroke_width(2.0);
        ctx.line_dash(&[8.0, 6.0]);
        ctx.oval(w / 2.0 - 140.0, h / 2.0 - 140.0, 280.0, 280.0);
        ctx.line_dash(&[]);

        // Orbiting, spinning square
        let angle = t * std::f64::consts::TAU;
        let (cx, cy) = (w / 2.0 + 140.0 * angle.cos(), h / 2.0 + 140.0 * angle.sin());
        ctx.save();
        ctx.translate(cx, cy);
        ctx.rotate(t * 720.0);
        ctx.fill(Color::rgb(220, 220, 220));
        ctx.no_stroke();
        ctx.rect(-32.0, -32.0, 64.0, 64.0);
        ctx.restore();
    }

    let renderer = Renderer::new(480, 480);
    renderer.render_to_gif(&ctx, "animation.gif").unwrap();
    println!("Rendered animation.gif ({} pages)", ctx.page_count());
}
