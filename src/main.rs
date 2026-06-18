use clap::{Parser, Subcommand};
use rusticker::{bake_grid, compose_grid, FigureType};
use std::path::PathBuf;

/// Rusticker CLI application
#[derive(Parser, Debug)]
#[command(
    name = "rusticker",
    version,
    disable_version_flag = true,
    about = "A Rust command-line application that demonstrates argument parsing and PDF generation",
    long_about = None
)]
struct Cli {
    /// Resolution of the application in DPI (dots per inch)
    #[arg(long, global = true, default_value_t = 300)]
    dpi: u32,

    /// Force overwrite of the output file if it already exists
    #[arg(long, global = true)]
    force: bool,

    /// Print version information
    #[arg(short = 'V', long = "version", action = clap::ArgAction::Version)]
    version: Option<bool>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Bake figures into a PDF grid on an A4 page
    Bake {
        /// Type of figure to bake (square or circle)
        #[arg(long, value_enum)]
        figure: FigureType,

        /// Size of the figure in pixels (side for square, diameter for circle)
        #[arg(long)]
        size: u32,

        /// Minimum space in millimeters between a figure and the others surrounding it
        #[arg(long, default_value_t = 2.0)]
        min_space: f64,

        /// Stroke thickness of the figure outline in millimeters
        #[arg(long, default_value_t = 1.0)]
        stroke_thickness: f64,

        /// Output file path for the PDF
        #[arg(short, long, default_value = "baked.pdf")]
        output: PathBuf,
    },
    /// Compose figures and repeat an input image into a PDF grid on an A4 page
    Compose {
        /// Type of figure to bake (square or circle)
        #[arg(long, value_enum)]
        figure: FigureType,

        /// Path to the input image file (PNG or JPEG)
        #[arg(long)]
        input: PathBuf,

        /// Size of the figure in pixels (side for square, diameter for circle)
        #[arg(long)]
        size: u32,

        /// Minimum space in millimeters between a figure and the others surrounding it
        #[arg(long, default_value_t = 2.0)]
        min_space: f64,

        /// Stroke thickness of the figure outline in millimeters
        #[arg(long, default_value_t = 1.0)]
        stroke_thickness: f64,

        /// Output file path for the PDF
        #[arg(short, long, default_value = "composed.pdf")]
        output: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let dpi = cli.dpi;
    let force = cli.force;

    // Validate DPI values
    if dpi != 100 && dpi != 200 && dpi != 300 && dpi != 600 {
        return Err("DPI must be one of: 100, 200, 300, 600".into());
    }

    match cli.command {
        Commands::Bake {
            figure,
            size,
            min_space,
            stroke_thickness,
            output,
        } => {
            if output.exists() && !force {
                return Err(format!(
                    "Output file '{}' already exists. Use --force to overwrite.",
                    output.display()
                )
                .into());
            }
            bake_grid(figure, size, dpi, min_space, stroke_thickness, output)?;
        }
        Commands::Compose {
            figure,
            input,
            size,
            min_space,
            stroke_thickness,
            output,
        } => {
            if output.exists() && !force {
                return Err(format!(
                    "Output file '{}' already exists. Use --force to overwrite.",
                    output.display()
                )
                .into());
            }
            compose_grid(figure, input, size, dpi, min_space, stroke_thickness, output)?;
        }
    }

    Ok(())
}
