use clap::{Parser, Subcommand};
use background_remover::{remove_background, ModelType};
use pdf_generator::{bake_grid, compose_grid, FigureType, BatchComposeLineArgs, BatchComposeLineParser, BatchStickerInput, PageSize};
use std::path::{Path, PathBuf};

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

    /// Page margin in millimeters
    #[arg(long, global = true, default_value_t = 5.0)]
    margin: f64,

    /// Page size for the output PDF
    #[arg(long, global = true, value_enum, default_value_t = PageSize::A4)]
    page_size: PageSize,

    /// Force overwrite of the output file if it already exists
    #[arg(long, global = true)]
    force: bool,

    /// Disable some guardrails (like vertices and loops limits for figure mask)
    #[arg(long, global = true)]
    r#unsafe: bool,

    /// Show verbose logs on stdout
    #[arg(short = 'v', long = "verbose", global = true)]
    verbose: bool,

    /// Print version information
    #[arg(short = 'V', long = "version", action = clap::ArgAction::SetTrue)]
    version: bool,

    #[command(subcommand)]
    command: Option<Commands>,
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
        #[arg(long, default_value_t = 0.25)]
        stroke_thickness: f64,

        /// Output file path for the PDF
        #[arg(short, long, default_value = "baked.pdf")]
        output: PathBuf,
    },
    /// Compose figures and repeat an input image into a PDF grid on an A4 page
    Compose {
        /// Path to the input image file (PNG or JPEG)
        #[arg(long)]
        input: PathBuf,

        /// Output file path for the PDF
        #[arg(short, long, default_value = "composed.pdf")]
        output: PathBuf,

        #[command(flatten)]
        args: BatchComposeLineArgs,
    },
    /// Compose stickers from a CSV configuration file into a PDF grid across A4 pages
    #[command(long_about = "Compose stickers from a CSV configuration file into a PDF grid across A4 pages.\n\n\
        The CSV file must contain the following columns per line:\n\
        <image_path>, <quantity>, <command_line_arguments_for_compose>\n\n\
        Example:\n\
        \"C:\\path\\to\\image.png\", 6, --figure circle --diameter 120 --stroke-thickness 1.5")]
    BatchCompose {
        /// Path to the input CSV file. Format: <image_path>, <quantity>, <command_line_arguments>
        #[arg(long)]
        input: PathBuf,

        /// Output file path for the PDF
        #[arg(short, long, default_value = "batch_composed.pdf")]
        output: PathBuf,
    },
    /// Erase the background of an image and save it as a transparent PNG.
    /// Pre-trained models are automatically downloaded to ~/.rusticker/models/
    Stickerize {
        /// Path to the input image file (PNG, JPEG, or WEBP)
        #[arg(long)]
        input: PathBuf,

        /// Output transparent PNG file path
        #[arg(short, long)]
        output: PathBuf,

        /// Model to use for background removal (models are downloaded to ~/.rusticker/models/)
        #[arg(long, value_enum, default_value = "u2netp")]
        model: ModelType,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let dpi = cli.dpi;
    let force = cli.force;
    let verbose = cli.verbose;
    let margin = cli.margin;
    let page_size = cli.page_size;
    let is_unsafe = cli.r#unsafe;

    if cli.version {
        let version_str = env!("CARGO_PKG_VERSION");
        println!("rusticker v{} - Sticker tool build with Rust by drkbugs", version_str);
        if verbose {
            println!("\nSupported background removal models:");
            println!("  - u2netp: https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2netp.onnx");
            println!("  - rmbg: https://huggingface.co/briaai/RMBG-1.4/resolve/main/onnx/model.onnx");
            println!("  - birefnet: https://github.com/danielgatis/rembg/releases/download/v0.0.0/BiRefNet-general-bb_swin_v1_tiny-epoch_232.onnx");
        }
        return Ok(());
    }

    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            let err = cmd.error(
                clap::error::ErrorKind::MissingSubcommand,
                "A subcommand is required but one was not provided.",
            );
            err.exit();
        }
    };

    // Validate DPI values
    if dpi != 100 && dpi != 200 && dpi != 300 && dpi != 600 {
        return Err("DPI must be one of: 100, 200, 300, 600".into());
    }

    match command {
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
            bake_grid(figure, w, h, dpi, margin, min_space, stroke_thickness, output, verbose, page_size)?;
        }
        Commands::Compose {
            input,
            output,
            args,
        } => {
            if output.exists() && !force {
                return Err(format!(
                    "Output file '{}' already exists. Use --force to overwrite.",
                    output.display()
                )
                .into());
            }
            let (resolved_w, resolved_h) = args.resolve_dimensions()?;
            compose_grid(
                args.figure,
                input,
                resolved_w,
                resolved_h,
                dpi,
                margin,
                args.min_space,
                args.stroke_thickness,
                output,
                verbose,
                args.algorithm,
                args.rdp_level,
                page_size,
                is_unsafe,
            )?;
        }
        Commands::BatchCompose {
            input,
            output,
        } => {
            if output.exists() && !force {
                return Err(format!(
                    "Output file '{}' already exists. Use --force to overwrite.",
                    output.display()
                )
                .into());
            }
            if verbose {
                println!("[VERBOSE] Validating CSV file: '{}'...", input.display());
            }
            let stickers = validate_and_parse_csv(&input, verbose)?;
            if verbose {
                println!("[VERBOSE] CSV validated successfully. Starting batch PDF composition...");
            }
            pdf_generator::batch_compose_grid(stickers, dpi, margin, output, verbose, page_size, is_unsafe)?;
        }
        Commands::Stickerize {
            input,
            output,
            model,
        } => {
            remove_background(input, output, model, force, verbose)?;
        }
    }

    Ok(())
}

fn parse_csv_line(line: &str) -> Result<(PathBuf, u32, String), String> {
    let line = line.trim();
    if line.is_empty() {
        return Err("Empty line".to_string());
    }
    
    let (path_str, rest) = if line.starts_with('"') {
        let closing_quote_idx = line[1..].find('"')
            .ok_or_else(|| "Mismatched double quotes in image path".to_string())?;
        let path = &line[1..1 + closing_quote_idx];
        let rest_after_quote = &line[1 + closing_quote_idx + 1..];
        let first_comma_idx = rest_after_quote.find(',')
            .ok_or_else(|| "Missing quantity and arguments after image path".to_string())?;
        (path, &rest_after_quote[first_comma_idx + 1..])
    } else {
        let first_comma_idx = line.find(',')
            .ok_or_else(|| "Missing comma after image path".to_string())?;
        (&line[..first_comma_idx], &line[first_comma_idx + 1..])
    };
    
    let (qty_str, args_str) = match rest.find(',') {
        Some(idx) => (&rest[..idx], &rest[idx + 1..]),
        None => (rest, ""),
    };
    
    let path = PathBuf::from(path_str.trim());
    let qty = qty_str.trim().parse::<u32>()
        .map_err(|_| format!("Invalid quantity: '{}'", qty_str.trim()))?;
    let args = args_str.trim().to_string();
    
    Ok((path, qty, args))
}

fn validate_and_parse_csv(
    path: &Path,
    _verbose: bool,
) -> Result<Vec<BatchStickerInput>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    use std::io::BufRead;
    
    let mut stickers = Vec::new();
    let mut row_idx = 0;
    
    for line_res in reader.lines() {
        row_idx += 1;
        let line = line_res?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        
        let (image_path, qty, args_str) = parse_csv_line(&line)
            .map_err(|e| format!("Row {} error: {}", row_idx, e))?;
            
        // Check image existence
        if !image_path.exists() {
            return Err(format!("Row {} error: Image file does not exist at '{}'", row_idx, image_path.display()).into());
        }
        if !image_path.is_file() {
            return Err(format!("Row {} error: Path '{}' is not a file", row_idx, image_path.display()).into());
        }
        
        // Parse arguments using clap
        let tokens = args_str.split_whitespace();
        
        let parser = BatchComposeLineParser::try_parse_from(tokens)
            .map_err(|e| format!("Row {} command-line arguments parsing error:\n{}", row_idx, e))?;
            
        let line_args = parser.args;
        
        // Resolve and validate shape dimensions
        let resolved_crop = line_args.resolve_dimensions()
            .map_err(|e| format!("Row {} validation error: {}", row_idx, e))?;
            
        stickers.push(BatchStickerInput {
            figure: line_args.figure,
            input_path: image_path,
            width_px: resolved_crop.0,
            height_px: resolved_crop.1,
            min_space_mm: line_args.min_space,
            stroke_thickness_mm: line_args.stroke_thickness,
            algorithm: line_args.algorithm,
            rdp_level: line_args.rdp_level,
            quantity: qty,
        });
    }
    
    if stickers.is_empty() {
        return Err("CSV file contains no sticker entries".into());
    }
    
    Ok(stickers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_line_unquoted() {
        let line = "path/to/image.png, 6, --figure circle --diameter 120";
        let (path, qty, args) = parse_csv_line(line).unwrap();
        assert_eq!(path, Path::new("path/to/image.png"));
        assert_eq!(qty, 6);
        assert_eq!(args, "--figure circle --diameter 120");
    }

    #[test]
    fn test_parse_csv_line_quoted_comma() {
        let line = "\"path,with,comma.png\", 3, --figure square --side 150";
        let (path, qty, args) = parse_csv_line(line).unwrap();
        assert_eq!(path, Path::new("path,with,comma.png"));
        assert_eq!(qty, 3);
        assert_eq!(args, "--figure square --side 150");
    }

    #[test]
    fn test_parse_csv_line_quoted_space() {
        let line = "\"path with space.png\", 2, --figure rectangle --width 100 --height 50";
        let (path, qty, args) = parse_csv_line(line).unwrap();
        assert_eq!(path, Path::new("path with space.png"));
        assert_eq!(qty, 2);
        assert_eq!(args, "--figure rectangle --width 100 --height 50");
    }

    #[test]
    fn test_parse_csv_line_no_args() {
        let line = "image.png, 10";
        let (path, qty, args) = parse_csv_line(line).unwrap();
        assert_eq!(path, Path::new("image.png"));
        assert_eq!(qty, 10);
        assert_eq!(args, "");
    }

    #[test]
    fn test_parse_csv_line_invalid_qty() {
        let line = "image.png, abc, --figure circle";
        assert!(parse_csv_line(line).is_err());
    }

    #[test]
    fn test_parse_csv_line_missing_fields() {
        let line = "image.png";
        assert!(parse_csv_line(line).is_err());
    }
}

