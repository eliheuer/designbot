//! Versioned data-only scenes for editor previews. Coordinates are y-up.
use anyhow::{bail, Context, Result};
use designbot_core::{Canvas, Color};
use designbot_render::Renderer;
use serde::Deserialize;
use std::io::Read;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scene {
    version: u32,
    width: u32,
    height: u32,
    paths: Vec<Path>,
    #[serde(default)]
    labels: Vec<Label>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Path {
    d: String,
    #[serde(default = "black")]
    color: [u8; 3],
}
fn black() -> [u8; 3] {
    [0, 0, 0]
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Label {
    text: String,
    x: f64,
    y: f64,
    size: f64,
}

pub fn run(args: &[String]) -> Result<()> {
    if args.len() != 2 || !matches!(args[0].as_str(), "--png" | "--pdf") {
        bail!("usage: designbot render-scene --png|--pdf OUTPUT < scene.json");
    }
    let mut input = Vec::new();
    std::io::stdin()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut input)?;
    if input.len() > 8 * 1024 * 1024 {
        bail!("scene exceeds 8 MiB");
    }
    let scene: Scene = serde_json::from_slice(&input).context("invalid scene")?;
    render(scene, &args[0], &args[1])
}
fn render(scene: Scene, format: &str, output: &str) -> Result<()> {
    if scene.version != 1
        || scene.width == 0
        || scene.height == 0
        || scene.width > 4096
        || scene.height > 4096
        || scene.paths.len() > 4096
        || scene.labels.len() > 512
    {
        bail!("unsupported version or scene dimensions/counts");
    }
    let mut canvas = Canvas::new(scene.width as f64, scene.height as f64);
    canvas.background(Color::rgb(255, 255, 255));
    for item in scene.paths {
        let path = kurbo::BezPath::from_svg(&item.d).context("invalid path")?;
        if path.elements().len() > 65536
            || path.elements().iter().any(|el| {
                use kurbo::PathEl::*;
                let valid = |p: &kurbo::Point| {
                    p.x.is_finite() && p.y.is_finite() && p.x.abs().max(p.y.abs()) <= 1e7
                };
                match el {
                    MoveTo(p) | LineTo(p) => !valid(p),
                    QuadTo(a, b) => !valid(a) || !valid(b),
                    CurveTo(a, b, c) => !valid(a) || !valid(b) || !valid(c),
                    ClosePath => false,
                }
            })
        {
            bail!("invalid or oversized path");
        }
        canvas.fill(Color::rgb(item.color[0], item.color[1], item.color[2]));
        canvas.draw_path(path);
    }
    canvas.fill(Color::rgb(0, 0, 0));
    for label in scene.labels {
        if label.text.len() > 1024
            || ![label.x, label.y, label.size].iter().all(|v| v.is_finite())
            || !(1.0..=256.0).contains(&label.size)
        {
            bail!("invalid label");
        }
        canvas.font_size(label.size);
        canvas.text(&label.text, label.x, label.y);
    }
    let renderer = Renderer::new(scene.width, scene.height);
    if format == "--pdf" {
        renderer.render_to_pdf(&canvas, output)?;
    } else {
        renderer.render_to_png(&canvas, output)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_version_and_unbounded_sizes() {
        for (version, width) in [(2, 32), (1, 5000)] {
            assert!(render(
                Scene {
                    version,
                    width,
                    height: 32,
                    paths: vec![],
                    labels: vec![]
                },
                "--png",
                "unused.png"
            )
            .is_err());
        }
    }
}
