use clap::Parser;
use background_remover::{remove_background, ModelType};
use std::path::PathBuf;

/// Stickerize CLI application - Background removal tool for rusticker
#[derive(Parser, Debug)]
#[command(
    name = "stickerize",
    version,
    about = "Erase the background of an image and save it as a transparent PNG",
    long_about = None
)]
struct Cli {
    /// Path to the input image file (PNG, JPEG, or WEBP)
    #[arg(long)]
    input: PathBuf,

    /// Output transparent PNG file path
    #[arg(short, long)]
    output: PathBuf,

    /// Model to use for background removal (models are downloaded to ~/.rusticker/models/)
    #[arg(long, value_enum, default_value = "u2netp")]
    model: ModelType,

    /// Force overwrite of the output file if it already exists
    #[arg(long, default_value_t = false)]
    force: bool,

    /// Show verbose logs on stdout
    #[arg(short = 'v', long = "verbose", default_value_t = false)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    if cli.verbose {
        println!("[VERBOSE] Starting background removal on {:?}", cli.input);
    }
    
    remove_background(cli.input, cli.output, cli.model, cli.force, cli.verbose)?;
    
    if cli.verbose {
        println!("[VERBOSE] Background removal finished successfully.");
    }
    Ok(())
}
