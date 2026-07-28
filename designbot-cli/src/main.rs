use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser, Debug)]
#[command(name = "designbot")]
#[command(about = "A Rust-based 2D graphics generation tool", long_about = None)]
struct Args {
    /// Path to the design script (.rs). Positional shorthand for --render;
    /// with no --output the image is written next to it as <script>.png.
    script: Option<String>,

    /// Path to the design script (.rs file). Same as the positional argument.
    #[arg(long)]
    render: Option<String>,

    /// Output file path (PNG, PDF, SVG, …). Defaults to <script>.png.
    #[arg(long)]
    output: Option<String>,

    /// Emit a plain, unmodified PNG. By default PNG output is optimized for
    /// social/web: an sRGB chunk, saturation + grain pre-compensation for
    /// platform JPEG recompression, and the X-lossless alpha trick. Pass
    /// --raw to skip all of that (e.g. for a pixel-exact master).
    #[arg(long)]
    raw: bool,

    /// Extra arguments forwarded to the compiled script (after --)
    #[arg(last = true)]
    script_args: Vec<String>,
}

/// `designbot proof <font> [-o out.pdf]` — the built-in default print proof.
#[derive(Parser, Debug)]
#[command(name = "designbot proof", about = "Generate a print proof PDF for a font")]
struct ProofArgs {
    /// Path to the font to proof (.ttf / .otf, static or variable).
    font: String,

    /// Output PDF path. Defaults to <font>-proof.pdf next to the font.
    #[arg(long, short)]
    output: Option<String>,
}

fn run_proof(argv: &[String]) -> Result<()> {
    let mut full = vec!["designbot-proof".to_string()];
    full.extend_from_slice(argv);
    let args = ProofArgs::parse_from(full);

    let font = PathBuf::from(&args.font);
    let output = args.output.unwrap_or_else(|| {
        let stem = font
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "font".into());
        font.with_file_name(format!("{stem}-proof.pdf"))
            .to_string_lossy()
            .into_owned()
    });

    println!("Proofing {} → {}", args.font, output);
    designbot_render::generate_proof(&font, &output)
        .with_context(|| format!("generating proof for {}", args.font))?;
    println!("✓ Wrote {output}");
    Ok(())
}

fn main() -> Result<()> {
    // `designbot proof …` is handled before the render arg parser (which would
    // otherwise treat "proof" as a positional script path).
    let raw: Vec<String> = std::env::args().collect();
    if raw.get(1).map(|s| s.as_str()) == Some("proof") {
        return run_proof(&raw[2..]);
    }

    let args = Args::parse();

    // Script comes from the positional arg or --render.
    let Some(script_path) = args.render.or(args.script) else {
        print_usage();
        return Ok(());
    };
    // Output defaults to the script path with a .png extension.
    let output_path = args
        .output
        .unwrap_or_else(|| default_output(&script_path));

    render_script(&script_path, &output_path, !args.raw, &args.script_args)
}

/// `path/to/foo.rs` -> `path/to/foo.png`, so `designbot foo.rs` just works.
fn default_output(script_path: &str) -> String {
    Path::new(script_path)
        .with_extension("png")
        .to_string_lossy()
        .into_owned()
}

fn print_usage() {
    eprintln!("DesignBot - A Rust-based 2D graphics generation tool");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  designbot <SCRIPT>            # writes <SCRIPT>.png next to it (social-optimized)");
    eprintln!("  designbot <SCRIPT> --raw     # plain, unmodified PNG");
    eprintln!("  designbot --render <S> --output <O>");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  designbot documentation/specimen-square.rs");
    eprintln!("  cargo run --example basic_shapes");
}

fn render_script(
    script_path: &str,
    output_path: &str,
    social: bool,
    script_args: &[String],
) -> Result<()> {
    let script_path = Path::new(script_path);
    let output_path = Path::new(output_path);

    // Validate script exists
    if !script_path.exists() {
        anyhow::bail!("Script file not found: {}", script_path.display());
    }

    // Get absolute paths
    let script_abs = fs::canonicalize(script_path)
        .with_context(|| format!("Failed to resolve script path: {}", script_path.display()))?;
    let output_abs = if output_path.is_absolute() {
        output_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(output_path)
    };

    println!("Rendering {} -> {}", script_abs.display(), output_abs.display());

    // Use a persistent per-script cache so subsequent runs are fast and
    // concurrent renders of different scripts don't clobber each other.
    let script_key = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        script_abs.hash(&mut h);
        let stem = script_abs
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "script".into());
        format!("{}-{:016x}", stem, h.finish())
    };
    let cache_dir = if let Some(home) = dirs::home_dir() {
        home.join(".designbot").join("cache").join(&script_key)
    } else {
        std::env::temp_dir().join("designbot-cache").join(&script_key)
    };

    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("Failed to create cache directory: {}", cache_dir.display()))?;

    // Read the user's script
    let user_script = fs::read_to_string(&script_abs)
        .with_context(|| format!("Failed to read script: {}", script_abs.display()))?;

    // Determine if script already has output handling
    let needs_wrapper = !user_script.contains("render_to_");

    // Create wrapper script if needed, or replace output path if it exists
    let final_script = if needs_wrapper {
        create_wrapper_script(&user_script, &output_abs, social)?
    } else {
        // Replace the output path in the user's render_to_png call
        replace_output_path(&user_script, &output_abs, social)?
    };

    // Create/update Cargo project in cache
    create_cache_project(&cache_dir, &final_script, &output_abs)?;

    // Get the directory where the user invoked the command
    let user_cwd = std::env::current_dir()
        .context("Failed to get current working directory")?;

    // Check if this is the first run
    let binary_path = cache_dir.join("target").join("release").join("designbot-script");
    let is_first_run = !binary_path.exists();

    // Show informative message on first run
    if is_first_run {
        println!();
        println!("First run detected - setting up DesignBot environment...");
        println!();
        println!("This will:");
        println!("  • Download Rust dependencies (~50MB)");
        println!("  • Compile the rendering engine");
        println!("  • Create a cache at {}", cache_dir.display());
        println!();
        println!("This is a one-time setup (~80 seconds).");
        println!("Future runs will be instant (<1 second)!");
        println!();
    }

    // Create progress spinner during compilation
    let spinner = if is_first_run {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_message("Downloading dependencies and compiling...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        Some(pb)
    } else {
        None
    };

    // Compile in cache directory, but run from user's directory.
    // Cap lints to `allow` so building designbot (a path dependency here) and
    // the user's script never spew warnings into the render output — only real
    // errors surface.
    let rustflags = match std::env::var("RUSTFLAGS") {
        Ok(existing) if !existing.is_empty() => format!("{existing} --cap-lints=allow"),
        _ => "--cap-lints=allow".to_string(),
    };
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--quiet")
        .env("RUSTFLAGS", rustflags)
        .current_dir(&cache_dir)
        .status()
        .context("Failed to compile")?;

    // Stop the spinner
    if let Some(spinner) = spinner {
        spinner.finish_with_message("✓ Setup complete!");
        println!();
        println!("Cache created at: {}", cache_dir.display());
        println!("You can remove this directory anytime to clear the cache.");
        println!();
    }

    if !status.success() {
        anyhow::bail!("Script compilation failed");
    }

    // Run the compiled binary from the user's working directory
    let binary_path = cache_dir.join("target").join("release").join("designbot-script");
    let status = Command::new(&binary_path)
        .current_dir(&user_cwd)
        .args(script_args)
        .status()
        .context("Failed to run compiled script")?;

    if !status.success() {
        anyhow::bail!("Script compilation or execution failed");
    }

    println!("✓ Rendered successfully: {}", output_abs.display());

    Ok(())
}

/// Pick the Renderer method from the output file extension (DrawBot's
/// saveImage-by-extension behavior): .gif -> animated GIF, .mp4/.mov -> video
/// via ffmpeg, .pdf -> vector PDF, .svg -> vector SVG, anything else -> PNG.
fn render_method_for(output_path: &Path, social: bool) -> &'static str {
    match output_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("gif") => "render_to_gif",
        Some("mp4") | Some("mov") => "render_to_mp4",
        Some("pdf") => "render_to_pdf",
        Some("svg") => "render_to_svg",
        _ if social => "render_to_png_social",
        _ => "render_to_png",
    }
}

fn create_wrapper_script(user_script: &str, output_path: &Path, social: bool) -> Result<String> {
    // Check if user script has a main function
    let has_main = user_script.contains("fn main");

    let wrapper = if has_main {
        // User has their own main, assume they handle rendering
        user_script.to_string()
    } else {
        // Wrap user code in a main function
        format!(
            r#"
use designbot::prelude::*;

fn main() {{
    let mut ctx = Canvas::new(800.0, 600.0);
    let mut renderer = Renderer::new(800, 600);

    // User code
    {user_script}

    // Render
    renderer.{method}(&ctx, "{output}").unwrap();
}}
"#,
            user_script = user_script,
            method = render_method_for(output_path, social),
            output = output_path.display().to_string().replace('\\', "\\\\")
        )
    };

    Ok(wrapper)
}

fn replace_output_path(user_script: &str, output_path: &Path, social: bool) -> Result<String> {
    let output_str = output_path.display().to_string().replace('\\', "\\\\");

    // Retarget the script's render_to_* call to the requested output path AND
    // format (extension picks the method, DrawBot saveImage-style).
    let method = render_method_for(output_path, social);
    let re = regex::Regex::new(r#"render_to_\w+\s*\(([^,]+),\s*"[^"]*"\)"#)
        .context("Failed to create regex")?;

    let result = re.replace_all(user_script, |caps: &regex::Captures| {
        format!(r#"{}({}, "{}")"#, method, &caps[1], output_str)
    });

    Ok(result.to_string())
}

fn create_cache_project(cache_dir: &Path, script: &str, _output_path: &Path) -> Result<()> {
    // Create Cargo.toml - try local paths first, fall back to git
    let cargo_toml = if let (Ok(designbot_path), Ok(render_path)) =
        (get_designbot_path(), get_designbot_render_path()) {
        // Development mode - use local paths
        format!(
            r#"[package]
name = "designbot-script"
version = "0.1.0"
edition = "2021"

[dependencies]
designbot = {{ path = "{designbot_path}" }}
designbot-render = {{ path = "{render_path}" }}
"#,
            designbot_path = designbot_path.display(),
            render_path = render_path.display()
        )
    } else {
        // Installed mode - use git dependencies
        format!(
            r#"[package]
name = "designbot-script"
version = "0.1.0"
edition = "2021"

[dependencies]
designbot = {{ git = "https://github.com/eliheuer/designbot", branch = "main" }}
designbot-render = {{ git = "https://github.com/eliheuer/designbot", branch = "main" }}
"#
        )
    };

    fs::write(cache_dir.join("Cargo.toml"), cargo_toml)
        .context("Failed to write Cargo.toml")?;

    // Create src directory and main.rs
    let src_dir = cache_dir.join("src");
    fs::create_dir_all(&src_dir).context("Failed to create src directory")?;
    fs::write(src_dir.join("main.rs"), script).context("Failed to write main.rs")?;

    Ok(())
}

fn get_designbot_path() -> Result<PathBuf> {
    // Try to find the workspace root (which has the main designbot package)
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().context("Failed to get exe directory")?;

    // Try multiple locations
    let current_dir = PathBuf::from(".");
    let parent_dir = PathBuf::from("..");
    // Workspace root baked in at compile time, so installed binaries
    // (cargo install --path) find their source checkout from any cwd.
    let manifest_workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent();
    let candidates = vec![
        manifest_workspace,
        // When running from cargo in workspace (target/debug or target/release) - go up to workspace root
        exe_dir.parent().and_then(|p| p.parent()),
        // When running from current directory
        Some(current_dir.as_path()),
        // When in a subdirectory
        Some(parent_dir.as_path()),
    ];

    for candidate in candidates.into_iter().flatten() {
        // Check if this is the workspace root by looking for the main Cargo.toml
        let cargo_toml = candidate.join("Cargo.toml");
        if cargo_toml.exists() {
            // Check if it contains the designbot package
            if let Ok(contents) = fs::read_to_string(&cargo_toml) {
                if contents.contains("name = \"designbot\"") || contents.contains("members") {
                    return fs::canonicalize(&candidate)
                        .with_context(|| format!("Failed to canonicalize path: {}", candidate.display()));
                }
            }
        }
    }

    anyhow::bail!("Could not find designbot library. Make sure you're running from the workspace or the library is published.")
}

fn get_designbot_render_path() -> Result<PathBuf> {
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().context("Failed to get exe directory")?;

    let manifest_workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent();
    let candidates = vec![
        manifest_workspace.map(|p| p.join("designbot-render")),
        exe_dir.parent().and_then(|p| p.parent()).map(|p| p.join("designbot-render")),
        Some(PathBuf::from("designbot-render")),
        Some(PathBuf::from("../designbot-render")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() && candidate.join("Cargo.toml").exists() {
            // Return absolute path
            return fs::canonicalize(&candidate)
                .with_context(|| format!("Failed to canonicalize path: {}", candidate.display()));
        }
    }

    anyhow::bail!("Could not find designbot-render library. Make sure you're running from the workspace or the library is published.")
}

