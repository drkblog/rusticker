use clap::Parser;
use background_remover::{remove_background, ModelType, OutputFormat};
use std::path::PathBuf;

/// Stickerize CLI application - Background removal tool for rusticker
#[derive(Parser, Debug)]
#[command(
    name = "stickerize",
    disable_version_flag = true,
    about = "Erase the background of an image and save it as a transparent PNG",
    long_about = None
)]
struct Cli {
    /// Path to the input image file (PNG, JPEG, or WEBP)
    #[arg(long, required_unless_present = "version")]
    input: Option<PathBuf>,

    /// Output transparent PNG file path
    #[arg(short, long, required_unless_present = "version")]
    output: Option<PathBuf>,

    /// Model to use for background removal (models are downloaded to ~/.rusticker/models/)
    #[arg(long, value_enum, default_value = "birefnet")]
    model: ModelType,

    /// Force overwrite of the output file if it already exists
    #[arg(long, default_value_t = false)]
    force: bool,

    /// Show verbose logs on stdout
    #[arg(short = 'v', long = "verbose", default_value_t = false)]
    verbose: bool,

    /// Use CUDA GPU acceleration for inference
    #[arg(long, default_value_t = false)]
    cuda: bool,

    /// Do not output any logs to stdout
    #[arg(short = 'q', long = "quiet", default_value_t = false)]
    quiet: bool,

    /// Print version information
    #[arg(short = 'V', long = "version", action = clap::ArgAction::SetTrue)]
    version: bool,

    /// Border thickness to add around the image in pixels (if present)
    #[arg(long)]
    border: Option<u32>,

    /// Border color in hexadecimal format (e.g. '#22AA5E' or '22AA5E', case insensitive)
    #[arg(long, default_value = "#FFFFFF")]
    border_color: String,

    /// Enable antialiasing for the outer part of the border outline (true/false)
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    antialiasing: bool,

    /// Output image format
    #[arg(long, value_enum, default_value = "png")]
    format: OutputFormat,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    if cli.version {
        println!("stickerize {}", env!("CARGO_PKG_VERSION"));
        println!("Background removal tool build with Rust by drkbugs");
        println!();
        println!("Supported Models:");
        println!("  - birefnet: https://github.com/danielgatis/rembg/releases/download/v0.0.0/BiRefNet-general-bb_swin_v1_tiny-epoch_232.onnx");
        println!("  - u2netp:   https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2netp.onnx");
        println!("  - rmbg:     https://huggingface.co/briaai/RMBG-1.4/resolve/main/onnx/model.onnx");
        return Ok(());
    }

    let input = cli.input.ok_or("Missing input path")?;
    let output = cli.output.ok_or("Missing output path")?;

    if cli.verbose && !cli.quiet {
        println!("[VERBOSE] Starting background removal on {:?}", input);
    }
    
    remove_background(
        input,
        output,
        cli.model,
        cli.force,
        cli.verbose,
        cli.cuda,
        cli.quiet,
        cli.border,
        Some(cli.border_color),
        cli.antialiasing,
        cli.format,
    )?;
    
    if cli.verbose && !cli.quiet {
        println!("[VERBOSE] Background removal finished successfully.");
    }
    Ok(())
}
