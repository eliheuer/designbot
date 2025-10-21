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
    /// Path to the design script (.rs file)
    #[arg(long)]
    render: Option<String>,

    /// Output file path (PNG, PDF, SVG, etc.)
    #[arg(long)]
    output: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if let (Some(script_path), Some(output_path)) = (args.render, args.output) {
        render_script(&script_path, &output_path)?;
    } else {
        print_usage();
    }

    Ok(())
}

fn print_usage() {
    eprintln!("DesignBot - A Rust-based 2D graphics generation tool");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  designbot --render <SCRIPT> --output <OUTPUT>");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  designbot --render design.rs --output output.png");
    eprintln!("  cargo run --example basic_shapes");
}

fn render_script(script_path: &str, output_path: &str) -> Result<()> {
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

    // Use persistent cache directory for faster subsequent runs
    let cache_dir = if let Some(home) = dirs::home_dir() {
        home.join(".designbot").join("cache")
    } else {
        std::env::temp_dir().join("designbot-cache")
    };

    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("Failed to create cache directory: {}", cache_dir.display()))?;

    // Read the user's script
    let user_script = fs::read_to_string(&script_abs)
        .with_context(|| format!("Failed to read script: {}", script_abs.display()))?;

    // Determine if script already has output handling
    let needs_wrapper = !user_script.contains("render_to_png");

    // Create wrapper script if needed, or replace output path if it exists
    let final_script = if needs_wrapper {
        create_wrapper_script(&user_script, &output_abs)?
    } else {
        // Replace the output path in the user's render_to_png call
        replace_output_path(&user_script, &output_abs)?
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

    // Compile in cache directory, but run from user's directory
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--quiet")
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
        .status()
        .context("Failed to run compiled script")?;

    if !status.success() {
        anyhow::bail!("Script compilation or execution failed");
    }

    println!("✓ Rendered successfully: {}", output_abs.display());

    Ok(())
}

fn create_wrapper_script(user_script: &str, output_path: &Path) -> Result<String> {
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
    renderer.render_to_png(&ctx, "{output}").unwrap();
}}
"#,
            user_script = user_script,
            output = output_path.display().to_string().replace('\\', "\\\\")
        )
    };

    Ok(wrapper)
}

fn replace_output_path(user_script: &str, output_path: &Path) -> Result<String> {
    // Use regex to replace the output path in render_to_png calls
    let re = regex::Regex::new(r#"render_to_png\s*\([^,]+,\s*"[^"]*"\)"#)
        .context("Failed to create regex")?;

    let output_str = output_path.display().to_string().replace('\\', "\\\\");
    let replacement = format!(r#"render_to_png($1, "{}")"#, output_str);

    // Replace render_to_png calls
    let re2 = regex::Regex::new(r#"render_to_png\s*\(([^,]+),\s*"[^"]*"\)"#)
        .context("Failed to create regex")?;

    let result = re2.replace_all(user_script, |caps: &regex::Captures| {
        format!(r#"render_to_png({}, "{}")"#, &caps[1], output_str)
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
    let candidates = vec![
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

    let candidates = vec![
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

