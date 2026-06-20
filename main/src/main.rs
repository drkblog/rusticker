use clap::{Parser, Subcommand};
use rusticker::{bake_grid, compose_grid, FigureType, MaskAlgorithmType};
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

    /// Show verbose logs on stdout
    #[arg(short = 'v', long = "verbose", global = true)]
    verbose: bool,

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

        /// Diameter of the circle in pixels (required for circle)
        #[arg(long)]
        diameter: Option<u32>,

        /// Side length of the square in pixels (required for square)
        #[arg(long)]
        side: Option<u32>,

        /// Width of the rectangle in pixels (required for rectangle)
        #[arg(long)]
        width: Option<u32>,

        /// Height of the rectangle in pixels (required for rectangle)
        #[arg(long)]
        height: Option<u32>,

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

        /// Diameter of the circle in pixels (optional for circle)
        #[arg(long)]
        diameter: Option<u32>,

        /// Side length of the square in pixels (optional for square)
        #[arg(long)]
        side: Option<u32>,

        /// Width of the rectangle in pixels (optional for rectangle)
        #[arg(long)]
        width: Option<u32>,

        /// Height of the rectangle in pixels (optional for rectangle)
        #[arg(long)]
        height: Option<u32>,

        /// Size of the mask figure in pixels
        #[arg(long)]
        size: Option<u32>,

        /// Minimum space in millimeters between a figure and the others surrounding it
        #[arg(long, default_value_t = 2.0)]
        min_space: f64,

        /// Stroke thickness of the figure outline in millimeters
        #[arg(long, default_value_t = 1.0)]
        stroke_thickness: f64,

        /// Output file path for the PDF
        #[arg(short, long, default_value = "composed.pdf")]
        output: PathBuf,

        /// Algorithm to use for mask generation (basic, advanced, or curves)
        #[arg(long, value_enum, default_value = "advanced")]
        algorithm: MaskAlgorithmType,

        /// Optimization level for RDP simplification (1 = low, 5 = high)
        #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u8).range(1..=5))]
        rdp_level: u8,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let dpi = cli.dpi;
    let force = cli.force;
    let verbose = cli.verbose;

    // Validate DPI values
    if dpi != 100 && dpi != 200 && dpi != 300 && dpi != 600 {
        return Err("DPI must be one of: 100, 200, 300, 600".into());
    }

    match cli.command {
        Commands::Bake {
            figure,
            diameter,
            side,
            width,
            height,
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
            let (w, h) = match figure {
                FigureType::Circle => {
                    if side.is_some() || width.is_some() || height.is_some() {
                        return Err("Error: Cannot specify --side, --width, or --height for a circle figure. Use --diameter instead.".into());
                    }
                    if let Some(d) = diameter {
                        (d, d)
                    } else {
                        return Err("Error: Missing required option --diameter for circle figure.".into());
                    }
                }
                FigureType::Square => {
                    if diameter.is_some() || width.is_some() || height.is_some() {
                        return Err("Error: Cannot specify --diameter, --width, or --height for a square figure. Use --side instead.".into());
                    }
                    if let Some(s) = side {
                        (s, s)
                    } else {
                        return Err("Error: Missing required option --side for square figure.".into());
                    }
                }
                FigureType::Rectangle => {
                    if diameter.is_some() || side.is_some() {
                        return Err("Error: Cannot specify --diameter or --side for a rectangle figure. Use --width and --height instead.".into());
                    }
                    match (width, height) {
                        (Some(w), Some(h)) => (w, h),
                        _ => return Err("Error: Missing required options --width and --height for rectangle figure.".into()),
                    }
                }
                FigureType::Mask => {
                    return Err("The 'mask' figure type requires an input image and is not supported in the bake subcommand.".into());
                }
            };
            bake_grid(figure, w, h, dpi, min_space, stroke_thickness, output, verbose)?;
        }
        Commands::Compose {
            figure,
            input,
            diameter,
            side,
            width,
            height,
            size,
            min_space,
            stroke_thickness,
            output,
            algorithm,
            rdp_level,
        } => {
            if output.exists() && !force {
                return Err(format!(
                    "Output file '{}' already exists. Use --force to overwrite.",
                    output.display()
                )
                .into());
            }
            let (resolved_w, resolved_h) = match figure {
                FigureType::Circle => {
                    if side.is_some() || width.is_some() || height.is_some() || size.is_some() {
                        return Err("Error: Cannot specify --side, --width, --height, or --size for a circle figure. Use --diameter instead.".into());
                    }
                    (diameter, diameter)
                }
                FigureType::Square => {
                    if diameter.is_some() || width.is_some() || height.is_some() || size.is_some() {
                        return Err("Error: Cannot specify --diameter, --width, --height, or --size for a square figure. Use --side instead.".into());
                    }
                    (side, side)
                }
                FigureType::Rectangle => {
                    if diameter.is_some() || side.is_some() || size.is_some() {
                        return Err("Error: Cannot specify --diameter, --side, or --size for a rectangle figure. Use --width and --height instead.".into());
                    }
                    match (width, height) {
                        (None, None) => (None, None),
                        (Some(w), Some(h)) => (Some(w), Some(h)),
                        _ => return Err("Error: For a rectangle figure, either specify both --width and --height, or specify neither.".into()),
                    }
                }
                FigureType::Mask => {
                    if diameter.is_some() || side.is_some() || width.is_some() || height.is_some() {
                        return Err("Error: Cannot specify --diameter, --side, --width, or --height for a mask figure. Use --size instead.".into());
                    }
                    (size, size)
                }
            };
            compose_grid(
                figure,
                input,
                resolved_w,
                resolved_h,
                dpi,
                min_space,
                stroke_thickness,
                output,
                verbose,
                algorithm,
                rdp_level,
            )?;
        }
    }

    Ok(())
}
