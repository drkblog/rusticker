use clap::{Args, Parser, ValueEnum};
use ::image::GenericImageView;
use printpdf::*;
use std::path::PathBuf;
use mask_generator::{MaskAlgorithm, BasicTracer, AdvancedTracer, CurvesTracer};

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FigureType {
    Square,
    Circle,
    Rectangle,
    Mask,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskAlgorithmType {
    Basic,
    Advanced,
    Curves,
}

#[derive(Args, Debug, Clone)]
pub struct BatchComposeLineArgs {
    /// Type of figure to bake
    #[arg(long, value_enum)]
    pub figure: FigureType,

    /// Diameter of the circle in pixels (optional for circle)
    #[arg(long)]
    pub diameter: Option<u32>,

    /// Side length of the square in pixels (optional for square)
    #[arg(long)]
    pub side: Option<u32>,

    /// Width of the rectangle in pixels (optional for rectangle)
    #[arg(long)]
    pub width: Option<u32>,

    /// Height of the rectangle in pixels (optional for rectangle)
    #[arg(long)]
    pub height: Option<u32>,

    /// Size of the mask figure in pixels
    #[arg(long)]
    pub size: Option<u32>,

    /// Minimum space in millimeters between a figure and the others surrounding it
    #[arg(long, default_value_t = 2.0)]
    pub min_space: f64,

    /// Stroke thickness of the figure outline in millimeters
    #[arg(long, default_value_t = 1.0)]
    pub stroke_thickness: f64,

    /// Algorithm to use for mask generation (basic, advanced, or curves)
    #[arg(long, value_enum, default_value = "advanced")]
    pub algorithm: MaskAlgorithmType,

    /// Optimization level for RDP simplification (1 = low, 5 = high)
    #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u8).range(1..=5))]
    pub rdp_level: u8,
}

#[derive(Parser, Debug)]
#[command(no_binary_name = true)]
pub struct BatchComposeLineParser {
    #[command(flatten)]
    pub args: BatchComposeLineArgs,
}

impl BatchComposeLineArgs {
    pub fn resolve_dimensions(&self) -> Result<(Option<u32>, Option<u32>), String> {
        match self.figure {
            FigureType::Circle => {
                if self.side.is_some() || self.width.is_some() || self.height.is_some() || self.size.is_some() {
                    return Err("Error: Cannot specify --side, --width, --height, or --size for a circle figure. Use --diameter instead.".to_string());
                }
                Ok((self.diameter, self.diameter))
            }
            FigureType::Square => {
                if self.diameter.is_some() || self.width.is_some() || self.height.is_some() || self.size.is_some() {
                    return Err("Error: Cannot specify --diameter, --width, --height, or --size for a square figure. Use --side instead.".to_string());
                }
                Ok((self.side, self.side))
            }
            FigureType::Rectangle => {
                if self.diameter.is_some() || self.side.is_some() || self.size.is_some() {
                    return Err("Error: Cannot specify --diameter, --side, or --size for a rectangle figure. Use --width and --height instead.".to_string());
                }
                match (self.width, self.height) {
                    (None, None) => Ok((None, None)),
                    (Some(w), Some(h)) => Ok((Some(w), Some(h))),
                    _ => Err("Error: For a rectangle figure, either specify both --width and --height, or specify neither.".to_string()),
                }
            }
            FigureType::Mask => {
                if self.diameter.is_some() || self.side.is_some() || self.width.is_some() || self.height.is_some() {
                    return Err("Error: Cannot specify --diameter, --side, --width, or --height for a mask figure. Use --size instead.".to_string());
                }
                Ok((self.size, self.size))
            }
        }
    }
}


pub fn bake_grid(
    figure: FigureType,
    width_px: u32,
    height_px: u32,
    dpi: u32,
    min_space_mm: f64,
    stroke_thickness_mm: f64,
    output_path: PathBuf,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if figure == FigureType::Mask {
        return Err("The 'mask' figure type requires an input image and is not supported in the bake subcommand.".into());
    }
    if verbose {
        println!("[VERBOSE] Step: Initializing layout and calculating grid cells...");
    }
    // A4 dimensions: 210mm x 297mm
    let page_width_mm = 210.0f32;
    let page_height_mm = 297.0f32;

    // Convert A4 dimensions to PDF points (1 inch = 25.4 mm, 1 inch = 72 points)
    let page_width_pt = (page_width_mm as f64) / 25.4 * 72.0;
    let page_height_pt = (page_height_mm as f64) / 25.4 * 72.0;

    // Convert figure size from pixels to PDF points
    let width_pt = (width_px as f64) / (dpi as f64) * 72.0;
    let height_pt = (height_px as f64) / (dpi as f64) * 72.0;

    // Set layout parameters: 10mm margins, min_space_mm gap between figures
    let margin_mm = 10.0f64;
    let gap_mm = min_space_mm;

    let margin_pt = margin_mm / 25.4 * 72.0;
    let gap_pt = gap_mm / 25.4 * 72.0;

    if verbose {
        println!("[VERBOSE] Grid details: Page size: {:.2}x{:.2} pt | Margins: {:.2} pt | Spacing (gap): {:.2} pt | Figure dimensions: {:.2}x{:.2} pt",
                 page_width_pt, page_height_pt, margin_pt, gap_pt, width_pt, height_pt);
    }

    let available_width = page_width_pt - 2.0 * margin_pt;
    let available_height = page_height_pt - 2.0 * margin_pt;

    if width_pt > available_width || height_pt > available_height {
        eprintln!(
            "Error: Figure size of {}x{} pixels ({:.2}x{:.2} pt) at {} DPI is larger than available page area (width: {:.2} pt, height: {:.2} pt).",
            width_px, height_px, width_pt, height_pt, dpi, available_width, available_height
        );
        return Err("Figure size exceeds available page space.".into());
    }

    // Number of columns and rows
    let cols = ((available_width + gap_pt) / (width_pt + gap_pt)).floor() as usize;
    let rows = ((available_height + gap_pt) / (height_pt + gap_pt)).floor() as usize;

    if cols == 0 || rows == 0 {
        return Err(
            "No figures could fit on the page under the current margins and spacing.".into(),
        );
    }

    if verbose {
        println!("[VERBOSE] Calculated grid layout: {} columns, {} rows (Total: {} figures)", cols, rows, cols * rows);
    }

    println!(
        "Baking grid of {}x{} = {} figures (size: {}x{} px / {:.2}x{:.2} pt, DPI: {}) to {:?}",
        cols,
        rows,
        cols * rows,
        width_px,
        height_px,
        width_pt,
        height_pt,
        dpi,
        output_path
    );

    if verbose {
        println!("[VERBOSE] Step: Initializing PDF document...");
    }
    // Initialize the PDF Document
    let mut doc = PdfDocument::new("Baked Grid");

    // Create a vector layer
    let graphics_layer = Layer {
        name: "Baked Layer".to_string(),
        creator: "rusticker".to_string(),
        intent: LayerIntent::Design,
        usage: LayerSubtype::Artwork,
    };
    let graphics_layer_id = doc.add_layer(&graphics_layer);

    let mut ops = Vec::new();

    // Start the vector layer
    ops.push(Op::BeginLayer {
        layer_id: graphics_layer_id,
    });

    // Set outline/stroke style: black color, custom thickness
    ops.push(Op::SetOutlineColor {
        col: Color::Rgb(Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            icc_profile: None,
        }),
    });
    let stroke_thickness_pt = stroke_thickness_mm / 25.4 * 72.0;
    ops.push(Op::SetOutlineThickness { pt: Pt(stroke_thickness_pt as f32) });

    if verbose {
        println!("[VERBOSE] Step: Drawing vector outlines for figures...");
    }

    for r in 0..rows {
        for c in 0..cols {
            // Calculate coordinates for top-left aligned placement of bounding box
            let x = margin_pt + (c as f64) * (width_pt + gap_pt);
            let y = page_height_pt - margin_pt - height_pt - (r as f64) * (height_pt + gap_pt);

            match figure {
                FigureType::Square | FigureType::Rectangle => {
                    let points = vec![
                        LinePoint {
                            p: Point {
                                x: Pt(x as f32),
                                y: Pt(y as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((x + width_pt) as f32),
                                y: Pt(y as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((x + width_pt) as f32),
                                y: Pt((y + height_pt) as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt(x as f32),
                                y: Pt((y + height_pt) as f32),
                            },
                            bezier: false,
                        },
                    ];
                    let line = Line {
                        points,
                        is_closed: true,
                    };
                    ops.push(Op::DrawLine { line });
                }
                FigureType::Circle => {
                    let cx = x + width_pt / 2.0;
                    let cy = y + height_pt / 2.0;
                    let radius = width_pt / 2.0;
                    let k = 0.55228474983; // Cubic Bézier circle approximation constant

                    let points = vec![
                        LinePoint {
                            p: Point {
                                x: Pt((cx + radius) as f32),
                                y: Pt(cy as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx + radius) as f32),
                                y: Pt((cy + radius * k) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx + radius * k) as f32),
                                y: Pt((cy + radius) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt(cx as f32),
                                y: Pt((cy + radius) as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx - radius * k) as f32),
                                y: Pt((cy + radius) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx - radius) as f32),
                                y: Pt((cy + radius * k) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx - radius) as f32),
                                y: Pt(cy as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx - radius) as f32),
                                y: Pt((cy - radius * k) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx - radius * k) as f32),
                                y: Pt((cy - radius) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt(cx as f32),
                                y: Pt((cy - radius) as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx + radius * k) as f32),
                                y: Pt((cy - radius) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx + radius) as f32),
                                y: Pt((cy - radius * k) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx + radius) as f32),
                                y: Pt(cy as f32),
                            },
                            bezier: false,
                        },
                    ];
                    let line = Line {
                        points,
                        is_closed: true,
                    };
                    ops.push(Op::DrawLine { line });
                }
                FigureType::Mask => unreachable!(),
            }
        }
    }

    // End the layer
    ops.push(Op::EndLayer);

    // Create the page with A4 dimensions (Portrait)
    let page = PdfPage::new(Mm(page_width_mm), Mm(page_height_mm), ops);

    if verbose {
        println!("[VERBOSE] Step: Encoding and saving PDF file...");
    }
    // Save page and document bytes
    let mut warnings = Vec::new();
    let pdf_bytes = doc
        .with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut warnings);

    if !warnings.is_empty() {
        for w in warnings {
            eprintln!("PDF Warning: {:?}", w);
        }
    }

    std::fs::write(&output_path, pdf_bytes)?;
    if verbose {
        println!("[VERBOSE] Step: Successfully wrote PDF to {:?}", output_path);
    }

    Ok(())
}

pub fn compose_grid(
    figure: FigureType,
    input_path: PathBuf,
    width_px: Option<u32>,
    height_px: Option<u32>,
    dpi: u32,
    min_space_mm: f64,
    stroke_thickness_mm: f64,
    output_path: PathBuf,
    verbose: bool,
    algorithm: MaskAlgorithmType,
    rdp_level: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("[VERBOSE] Step: Opening input image...");
    }
    // 1. Open the image
    let img = ::image::ImageReader::open(&input_path)?
        .with_guessed_format()?
        .decode()?;
    
    // 2. Determine target size and crop if needed
    let (width, height) = img.dimensions();
    if verbose {
        println!("[VERBOSE] Original image dimensions: {}x{} px", width, height);
    }

    let (cropped, actual_width_px, actual_height_px) = if width_px.is_some() || height_px.is_some() {
        let w = width_px.unwrap_or(width);
        let h = height_px.unwrap_or(height);
        if width < w || height < h {
            return Err("Input image is smaller than the specified size".into());
        }
        if verbose {
            println!("[VERBOSE] Step: Cropping image to {}x{} px...", w, h);
        }
        let cropped_img = if width == w && height == h {
            img
        } else {
            let x = (width - w) / 2;
            let y = (height - h) / 2;
            img.crop_imm(x, y, w, h)
        };
        (cropped_img, w, h)
    } else {
        if verbose {
            println!("[VERBOSE] Step: No cropping requested. Using original dimensions.");
        }
        (img, width, height)
    };

    let (cropped_width, cropped_height) = cropped.dimensions();
    if verbose {
        println!("[VERBOSE] Resulting image dimensions: {}x{} px", cropped_width, cropped_height);
    }

    let mut loops = Vec::new();
    if figure == FigureType::Mask {
        if verbose {
            println!("[VERBOSE] Step: Detecting background mask and tracing contour outline using {:?} algorithm...", algorithm);
        }
        match algorithm {
            MaskAlgorithmType::Basic => {
                let tracer = BasicTracer;
                loops = tracer.trace_mask(&cropped, verbose)?;
            }
            MaskAlgorithmType::Advanced => {
                let tracer = AdvancedTracer { rdp_level };
                loops = tracer.trace_mask(&cropped, verbose)?;
            }
            MaskAlgorithmType::Curves => {
                let tracer = CurvesTracer { rdp_level };
                loops = tracer.trace_mask(&cropped, verbose)?;
            }
        }
    }

    // 4. Encode cropped image to PNG bytes in-memory
    let mut png_bytes: Vec<u8> = Vec::new();
    cropped.write_to(&mut std::io::Cursor::new(&mut png_bytes), ::image::ImageFormat::Png)?;

    if verbose {
        println!("[VERBOSE] Step: Decoding image into printpdf RawImage...");
    }
    // 5. Decode into printpdf::RawImage
    let mut image_warnings = Vec::new();
    let pdf_image = RawImage::decode_from_bytes(&png_bytes, &mut image_warnings)
        .map_err(|e| format!("Failed to decode cropped image: {}", e))?;

    // A4 dimensions: 210mm x 297mm
    let page_width_mm = 210.0f32;
    let page_height_mm = 297.0f32;

    // Convert A4 dimensions to PDF points (1 inch = 25.4 mm, 1 inch = 72 points)
    let page_width_pt = (page_width_mm as f64) / 25.4 * 72.0;
    let page_height_pt = (page_height_mm as f64) / 25.4 * 72.0;

    // Convert figure size from pixels to PDF points
    let (width_pt, height_pt) = if figure == FigureType::Mask {
        (
            (cropped_width as f64) / (dpi as f64) * 72.0,
            (cropped_height as f64) / (dpi as f64) * 72.0,
        )
    } else {
        (
            (actual_width_px as f64) / (dpi as f64) * 72.0,
            (actual_height_px as f64) / (dpi as f64) * 72.0,
        )
    };

    // Set layout parameters: 10mm margins, min_space_mm gap between figures
    let margin_mm = 10.0f64;
    let gap_mm = min_space_mm;

    let margin_pt = margin_mm / 25.4 * 72.0;
    let gap_pt = gap_mm / 25.4 * 72.0;

    if verbose {
        println!("[VERBOSE] Grid details: Page size: {:.2}x{:.2} pt | Margins: {:.2} pt | Spacing (gap): {:.2} pt | Figure dimensions: {:.2}x{:.2} pt",
                 page_width_pt, page_height_pt, margin_pt, gap_pt, width_pt, height_pt);
    }

    let available_width = page_width_pt - 2.0 * margin_pt;
    let available_height = page_height_pt - 2.0 * margin_pt;

    if width_pt > available_width || height_pt > available_height {
        eprintln!(
            "Error: Figure size of {}x{} pixels ({:.2}x{:.2} pt) at {} DPI is larger than available page area (width: {:.2} pt, height: {:.2} pt).",
            cropped_width, cropped_height, width_pt, height_pt, dpi, available_width, available_height
        );
        return Err("Figure size exceeds available page space.".into());
    }

    // Number of columns and rows
    let cols = ((available_width + gap_pt) / (width_pt + gap_pt)).floor() as usize;
    let rows = ((available_height + gap_pt) / (height_pt + gap_pt)).floor() as usize;

    if cols == 0 || rows == 0 {
        return Err(
            "No figures could fit on the page under the current margins and spacing.".into(),
        );
    }

    if verbose {
        println!("[VERBOSE] Calculated grid layout: {} columns, {} rows (Total: {} figures)", cols, rows, cols * rows);
    }

    if figure == FigureType::Mask {
        println!(
            "Composing grid of {}x{} = {} figures (size: {}x{} px / {:.2}x{:.2} pt, DPI: {}) to {:?}",
            cols,
            rows,
            cols * rows,
            cropped_width,
            cropped_height,
            width_pt,
            height_pt,
            dpi,
            output_path
        );
    } else {
        println!(
            "Composing grid of {}x{} = {} figures (size: {}x{} px / {:.2}x{:.2} pt, DPI: {}) to {:?}",
            cols,
            rows,
            cols * rows,
            actual_width_px,
            actual_height_px,
            width_pt,
            height_pt,
            dpi,
            output_path
        );
    }

    if verbose {
        println!("[VERBOSE] Step: Initializing PDF document...");
    }
    // Initialize the PDF Document
    let mut doc = PdfDocument::new("Composed Grid");

    // Add image as XObject to the document
    let image_xobject_id = doc.add_image(&pdf_image);

    // Create a vector layer
    let graphics_layer = Layer {
        name: "Vector Layer".to_string(),
        creator: "rusticker".to_string(),
        intent: LayerIntent::Design,
        usage: LayerSubtype::Artwork,
    };
    let graphics_layer_id = doc.add_layer(&graphics_layer);

    if verbose {
        println!("[VERBOSE] Step: Drawing vector outlines for figures...");
    }

    // Create a raster layer on top
    let raster_layer = Layer {
        name: "Raster Layer".to_string(),
        creator: "rusticker".to_string(),
        intent: LayerIntent::Design,
        usage: LayerSubtype::Artwork,
    };
    let raster_layer_id = doc.add_layer(&raster_layer);

    let mut ops = Vec::new();

    // 1. Render the vector layer
    ops.push(Op::BeginLayer {
        layer_id: graphics_layer_id,
    });

    // Set outline/stroke style: black color, custom thickness
    ops.push(Op::SetOutlineColor {
        col: Color::Rgb(Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            icc_profile: None,
        }),
    });
    let stroke_thickness_pt = stroke_thickness_mm / 25.4 * 72.0;
    ops.push(Op::SetOutlineThickness { pt: Pt(stroke_thickness_pt as f32) });

    for r in 0..rows {
        for c in 0..cols {
            // Calculate coordinates for top-left aligned placement of bounding box
            let x = margin_pt + (c as f64) * (width_pt + gap_pt);
            let y = page_height_pt - margin_pt - height_pt - (r as f64) * (height_pt + gap_pt);

            match figure {
                FigureType::Square | FigureType::Rectangle => {
                    let points = vec![
                        LinePoint {
                            p: Point {
                                x: Pt(x as f32),
                                y: Pt(y as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((x + width_pt) as f32),
                                y: Pt(y as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((x + width_pt) as f32),
                                y: Pt((y + height_pt) as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt(x as f32),
                                y: Pt((y + height_pt) as f32),
                            },
                            bezier: false,
                        },
                    ];
                    let line = Line {
                        points,
                        is_closed: true,
                    };
                    ops.push(Op::DrawLine { line });
                }
                FigureType::Circle => {
                    let cx = x + width_pt / 2.0;
                    let cy = y + height_pt / 2.0;
                    let radius = width_pt / 2.0;
                    let k = 0.55228474983; // Cubic Bézier circle approximation constant

                    let points = vec![
                        LinePoint {
                            p: Point {
                                x: Pt((cx + radius) as f32),
                                y: Pt(cy as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx + radius) as f32),
                                y: Pt((cy + radius * k) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx + radius * k) as f32),
                                y: Pt((cy + radius) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt(cx as f32),
                                y: Pt((cy + radius) as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx - radius * k) as f32),
                                y: Pt((cy + radius) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx - radius) as f32),
                                y: Pt((cy + radius * k) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx - radius) as f32),
                                y: Pt(cy as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx - radius) as f32),
                                y: Pt((cy - radius * k) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx - radius * k) as f32),
                                y: Pt((cy - radius) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt(cx as f32),
                                y: Pt((cy - radius) as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx + radius * k) as f32),
                                y: Pt((cy - radius) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx + radius) as f32),
                                y: Pt((cy - radius * k) as f32),
                            },
                            bezier: true,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((cx + radius) as f32),
                                y: Pt(cy as f32),
                            },
                            bezier: false,
                        },
                    ];
                    let line = Line {
                        points,
                        is_closed: true,
                    };
                    ops.push(Op::DrawLine { line });
                }
                FigureType::Mask => {
                    let scale = 72.0 / (dpi as f64);
                    for lp in &loops {
                        let mut points = Vec::new();
                        for &((cx, cy), is_bezier) in lp {
                            let lx = x + (cx - 1.0) * scale;
                            let ly = y + (cropped_height as f64 - (cy - 1.0)) * scale;
                            points.push(LinePoint {
                                p: Point {
                                    x: Pt(lx as f32),
                                    y: Pt(ly as f32),
                                },
                                bezier: is_bezier,
                            });
                        }
                        let line = Line {
                            points,
                            is_closed: true,
                        };
                        ops.push(Op::DrawLine { line });
                    }
                }
            }
        }
    }

    // End Vector Layer
    ops.push(Op::EndLayer);

    if verbose {
        println!("[VERBOSE] Step: Drawing raster layer images...");
    }
    // 2. Render the raster layer
    ops.push(Op::BeginLayer {
        layer_id: raster_layer_id,
    });

    let scale_factor = (300.0 / (dpi as f64)) as f32;

    for r in 0..rows {
        for c in 0..cols {
            // Calculate coordinates for top-left aligned placement of image
            let x = margin_pt + (c as f64) * (width_pt + gap_pt);
            let y = page_height_pt - margin_pt - height_pt - (r as f64) * (height_pt + gap_pt);

            ops.push(Op::UseXobject {
                id: image_xobject_id.clone(),
                transform: XObjectTransform {
                    translate_x: Some(Pt(x as f32)),
                    translate_y: Some(Pt(y as f32)),
                    scale_x: Some(scale_factor),
                    scale_y: Some(scale_factor),
                    ..Default::default()
                },
            });
        }
    }

    // End Raster Layer
    ops.push(Op::EndLayer);

    // Create the page with A4 dimensions (Portrait)
    let page = PdfPage::new(Mm(page_width_mm), Mm(page_height_mm), ops);

    if verbose {
        println!("[VERBOSE] Step: Encoding and saving PDF file...");
    }
    // Save page and document bytes
    let mut warnings = Vec::new();
    let pdf_bytes = doc
        .with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut warnings);

    if !warnings.is_empty() {
        for w in warnings {
            eprintln!("PDF Warning: {:?}", w);
        }
    }

    std::fs::write(&output_path, pdf_bytes)?;
    if verbose {
        println!("[VERBOSE] Step: Successfully wrote PDF to {:?}", output_path);
    }

    Ok(())
}

pub struct BatchStickerInput {
    pub figure: FigureType,
    pub input_path: PathBuf,
    pub width_px: Option<u32>,
    pub height_px: Option<u32>,
    pub min_space_mm: f64,
    pub stroke_thickness_mm: f64,
    pub algorithm: MaskAlgorithmType,
    pub rdp_level: u8,
    pub quantity: u32,
}

struct PreprocessedSticker {
    figure: FigureType,
    raw_image: RawImage,
    loops: Vec<Vec<((f64, f64), bool)>>,
    width_pt: f64,
    height_pt: f64,
    cropped_height_px: u32,
    min_space_mm: f64,
    stroke_thickness_mm: f64,
    quantity: u32,
}

struct PlacedInstance {
    sticker_index: usize,
    x_pt: f64,
    y_pt: f64,
}

pub fn batch_compose_grid(
    stickers: Vec<BatchStickerInput>,
    dpi: u32,
    output_path: PathBuf,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut preprocessed = Vec::new();
    for sticker in &stickers {
        if verbose {
            println!("[VERBOSE] Preprocessing sticker: {:?}", sticker.input_path);
        }
        let img = ::image::ImageReader::open(&sticker.input_path)?
            .with_guessed_format()?
            .decode()?;
        
        let (width, height) = img.dimensions();
        let (cropped, actual_width_px, actual_height_px) = if sticker.width_px.is_some() || sticker.height_px.is_some() {
            let w = sticker.width_px.unwrap_or(width);
            let h = sticker.height_px.unwrap_or(height);
            if width < w || height < h {
                return Err(format!("Input image '{}' is smaller than the specified size {}x{}", sticker.input_path.display(), w, h).into());
            }
            let cropped_img = if width == w && height == h {
                img
            } else {
                let x = (width - w) / 2;
                let y = (height - h) / 2;
                img.crop_imm(x, y, w, h)
            };
            (cropped_img, w, h)
        } else {
            (img, width, height)
        };

        let (cropped_width, cropped_height) = cropped.dimensions();
        let mut loops = Vec::new();
        if sticker.figure == FigureType::Mask {
            match sticker.algorithm {
                MaskAlgorithmType::Basic => {
                    loops = BasicTracer.trace_mask(&cropped, verbose)?;
                }
                MaskAlgorithmType::Advanced => {
                    loops = AdvancedTracer { rdp_level: sticker.rdp_level }.trace_mask(&cropped, verbose)?;
                }
                MaskAlgorithmType::Curves => {
                    loops = CurvesTracer { rdp_level: sticker.rdp_level }.trace_mask(&cropped, verbose)?;
                }
            }
        }

        let mut png_bytes = Vec::new();
        cropped.write_to(&mut std::io::Cursor::new(&mut png_bytes), ::image::ImageFormat::Png)?;

        let mut image_warnings = Vec::new();
        let raw_image = RawImage::decode_from_bytes(&png_bytes, &mut image_warnings)
            .map_err(|e| format!("Failed to decode image {:?}: {}", sticker.input_path, e))?;

        let (width_pt, height_pt) = if sticker.figure == FigureType::Mask {
            (
                (cropped_width as f64) / (dpi as f64) * 72.0,
                (cropped_height as f64) / (dpi as f64) * 72.0,
            )
        } else {
            (
                (actual_width_px as f64) / (dpi as f64) * 72.0,
                (actual_height_px as f64) / (dpi as f64) * 72.0,
            )
        };

        // Validate that this sticker can fit on an A4 page
        let page_width_mm = 210.0f32;
        let page_height_mm = 297.0f32;
        let page_width_pt = (page_width_mm as f64) / 25.4 * 72.0;
        let page_height_pt = (page_height_mm as f64) / 25.4 * 72.0;
        let margin_pt = 10.0 / 25.4 * 72.0;
        let available_width = page_width_pt - 2.0 * margin_pt;
        let available_height = page_height_pt - 2.0 * margin_pt;

        if width_pt > available_width || height_pt > available_height {
            return Err(format!(
                "Error: Figure size of {:.2}x{:.2} pt for image '{}' is larger than available page area ({:.2}x{:.2} pt).",
                width_pt, height_pt, sticker.input_path.display(), available_width, available_height
            ).into());
        }

        preprocessed.push(PreprocessedSticker {
            figure: sticker.figure,
            raw_image,
            loops,
            width_pt,
            height_pt,
            cropped_height_px: cropped_height,
            min_space_mm: sticker.min_space_mm,
            stroke_thickness_mm: sticker.stroke_thickness_mm,
            quantity: sticker.quantity,
        });
    }

    let page_width_mm = 210.0f32;
    let page_height_mm = 297.0f32;
    let page_width_pt = (page_width_mm as f64) / 25.4 * 72.0;
    let page_height_pt = (page_height_mm as f64) / 25.4 * 72.0;
    let margin_pt = 10.0 / 25.4 * 72.0;

    let mut pages_placements: Vec<Vec<PlacedInstance>> = Vec::new();
    let mut current_page: Vec<PlacedInstance> = Vec::new();
    let mut current_x = margin_pt;
    let mut current_y_top = page_height_pt - margin_pt;
    let mut row_max_height = 0.0;

    for (sticker_idx, sticker) in preprocessed.iter().enumerate() {
        let w = sticker.width_pt;
        let h = sticker.height_pt;
        let gap = sticker.min_space_mm / 25.4 * 72.0;

        for _ in 0..sticker.quantity {
            // Check if it fits horizontally on the current row
            if current_x > margin_pt && current_x + w > page_width_pt - margin_pt {
                // Wrap to next row
                current_y_top -= row_max_height;
                current_x = margin_pt;
                row_max_height = 0.0;
            }

            // Check if it fits vertically on the current page
            if current_y_top - h < margin_pt {
                // Doesn't fit on this page, start a new page
                pages_placements.push(current_page);
                current_page = Vec::new();
                current_x = margin_pt;
                current_y_top = page_height_pt - margin_pt;
                row_max_height = 0.0;
            }

            // Place the sticker
            let x = current_x;
            let y = current_y_top - h;
            current_page.push(PlacedInstance {
                sticker_index: sticker_idx,
                x_pt: x,
                y_pt: y,
            });

            // Update row height and move horizontal cursor
            row_max_height = row_max_height.max(h + gap);
            current_x += w + gap;
        }
    }

    if !current_page.is_empty() {
        pages_placements.push(current_page);
    }

    if verbose {
        println!("[VERBOSE] Initializing PDF document...");
    }
    let mut doc = PdfDocument::new("Batch Composed Grid");

    // Add unique images
    let mut image_xobject_ids = Vec::new();
    for sticker in &preprocessed {
        image_xobject_ids.push(doc.add_image(&sticker.raw_image));
    }

    let graphics_layer = Layer {
        name: "Vector Layer".to_string(),
        creator: "rusticker".to_string(),
        intent: LayerIntent::Design,
        usage: LayerSubtype::Artwork,
    };
    let graphics_layer_id = doc.add_layer(&graphics_layer);

    let raster_layer = Layer {
        name: "Raster Layer".to_string(),
        creator: "rusticker".to_string(),
        intent: LayerIntent::Design,
        usage: LayerSubtype::Artwork,
    };
    let raster_layer_id = doc.add_layer(&raster_layer);

    let mut pdf_pages = Vec::new();

    for (page_idx, placements) in pages_placements.iter().enumerate() {
        if verbose {
            println!("[VERBOSE] Rendering page {}/{}", page_idx + 1, pages_placements.len());
        }
        let mut ops = Vec::new();

        // 1. Draw Vector Outlines
        ops.push(Op::BeginLayer {
            layer_id: graphics_layer_id.clone(),
        });
        
        ops.push(Op::SetOutlineColor {
            col: Color::Rgb(Rgb {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                icc_profile: None,
            }),
        });

        for placement in placements {
            let sticker = &preprocessed[placement.sticker_index];
            let x = placement.x_pt;
            let y = placement.y_pt;
            let w = sticker.width_pt;
            let h = sticker.height_pt;

            let stroke_thickness_pt = sticker.stroke_thickness_mm / 25.4 * 72.0;
            ops.push(Op::SetOutlineThickness { pt: Pt(stroke_thickness_pt as f32) });

            match sticker.figure {
                FigureType::Square | FigureType::Rectangle => {
                    let points = vec![
                        LinePoint { p: Point { x: Pt(x as f32), y: Pt(y as f32) }, bezier: false },
                        LinePoint { p: Point { x: Pt((x + w) as f32), y: Pt(y as f32) }, bezier: false },
                        LinePoint { p: Point { x: Pt((x + w) as f32), y: Pt((y + h) as f32) }, bezier: false },
                        LinePoint { p: Point { x: Pt(x as f32), y: Pt((y + h) as f32) }, bezier: false },
                    ];
                    ops.push(Op::DrawLine { line: Line { points, is_closed: true } });
                }
                FigureType::Circle => {
                    let cx = x + w / 2.0;
                    let cy = y + h / 2.0;
                    let radius = w / 2.0;
                    let k = 0.55228474983;

                    let points = vec![
                        LinePoint { p: Point { x: Pt((cx + radius) as f32), y: Pt(cy as f32) }, bezier: false },
                        LinePoint { p: Point { x: Pt((cx + radius) as f32), y: Pt((cy + radius * k) as f32) }, bezier: true },
                        LinePoint { p: Point { x: Pt((cx + radius * k) as f32), y: Pt((cy + radius) as f32) }, bezier: true },
                        LinePoint { p: Point { x: Pt(cx as f32), y: Pt((cy + radius) as f32) }, bezier: false },
                        LinePoint { p: Point { x: Pt((cx - radius * k) as f32), y: Pt((cy + radius) as f32) }, bezier: true },
                        LinePoint { p: Point { x: Pt((cx - radius) as f32), y: Pt((cy + radius * k) as f32) }, bezier: true },
                        LinePoint { p: Point { x: Pt((cx - radius) as f32), y: Pt(cy as f32) }, bezier: false },
                        LinePoint { p: Point { x: Pt((cx - radius) as f32), y: Pt((cy - radius * k) as f32) }, bezier: true },
                        LinePoint { p: Point { x: Pt((cx - radius * k) as f32), y: Pt((cy - radius) as f32) }, bezier: true },
                        LinePoint { p: Point { x: Pt(cx as f32), y: Pt((cy - radius) as f32) }, bezier: false },
                        LinePoint { p: Point { x: Pt((cx + radius * k) as f32), y: Pt((cy - radius) as f32) }, bezier: true },
                        LinePoint { p: Point { x: Pt((cx + radius) as f32), y: Pt((cy - radius * k) as f32) }, bezier: true },
                        LinePoint { p: Point { x: Pt((cx + radius) as f32), y: Pt(cy as f32) }, bezier: false },
                    ];
                    ops.push(Op::DrawLine { line: Line { points, is_closed: true } });
                }
                FigureType::Mask => {
                    let scale = 72.0 / (dpi as f64);
                    for lp in &sticker.loops {
                        let mut points = Vec::new();
                        for &((cx, cy), is_bezier) in lp {
                            let lx = x + (cx - 1.0) * scale;
                            let ly = y + (sticker.cropped_height_px as f64 - (cy - 1.0)) * scale;
                            points.push(LinePoint {
                                p: Point {
                                    x: Pt(lx as f32),
                                    y: Pt(ly as f32),
                                },
                                bezier: is_bezier,
                            });
                        }
                        ops.push(Op::DrawLine { line: Line { points, is_closed: true } });
                    }
                }
            }
        }
        ops.push(Op::EndLayer);

        // 2. Draw Raster Images
        ops.push(Op::BeginLayer {
            layer_id: raster_layer_id.clone(),
        });

        let scale_factor = (300.0 / (dpi as f64)) as f32;

        for placement in placements {
            let x = placement.x_pt;
            let y = placement.y_pt;
            let image_xobject_id = &image_xobject_ids[placement.sticker_index];

            ops.push(Op::UseXobject {
                id: image_xobject_id.clone(),
                transform: XObjectTransform {
                    translate_x: Some(Pt(x as f32)),
                    translate_y: Some(Pt(y as f32)),
                    scale_x: Some(scale_factor),
                    scale_y: Some(scale_factor),
                    ..Default::default()
                },
            });
        }
        ops.push(Op::EndLayer);

        let page = PdfPage::new(Mm(page_width_mm), Mm(page_height_mm), ops);
        pdf_pages.push(page);
    }

    if verbose {
        println!("[VERBOSE] Encoding and saving PDF file...");
    }
    let mut warnings = Vec::new();
    let pdf_bytes = doc
        .with_pages(pdf_pages)
        .save(&PdfSaveOptions::default(), &mut warnings);

    if !warnings.is_empty() {
        for w in warnings {
            eprintln!("PDF Warning: {:?}", w);
        }
    }

    std::fs::write(&output_path, pdf_bytes)?;
    if verbose {
        println!("[VERBOSE] Successfully wrote PDF to {:?}", output_path);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_dimensions_circle_valid() {
        let args = BatchComposeLineArgs {
            figure: FigureType::Circle,
            diameter: Some(120),
            side: None,
            width: None,
            height: None,
            size: None,
            min_space: 2.0,
            stroke_thickness: 1.0,
            algorithm: MaskAlgorithmType::Advanced,
            rdp_level: 3,
        };
        let dims = args.resolve_dimensions().unwrap();
        assert_eq!(dims, (Some(120), Some(120)));
    }

    #[test]
    fn test_resolve_dimensions_circle_invalid() {
        let args = BatchComposeLineArgs {
            figure: FigureType::Circle,
            diameter: Some(120),
            side: Some(150),
            width: None,
            height: None,
            size: None,
            min_space: 2.0,
            stroke_thickness: 1.0,
            algorithm: MaskAlgorithmType::Advanced,
            rdp_level: 3,
        };
        assert!(args.resolve_dimensions().is_err());
    }

    #[test]
    fn test_resolve_dimensions_rectangle_valid() {
        let args = BatchComposeLineArgs {
            figure: FigureType::Rectangle,
            diameter: None,
            side: None,
            width: Some(150),
            height: Some(100),
            size: None,
            min_space: 2.0,
            stroke_thickness: 1.0,
            algorithm: MaskAlgorithmType::Advanced,
            rdp_level: 3,
        };
        let dims = args.resolve_dimensions().unwrap();
        assert_eq!(dims, (Some(150), Some(100)));
    }

    #[test]
    fn test_resolve_dimensions_rectangle_invalid() {
        let args = BatchComposeLineArgs {
            figure: FigureType::Rectangle,
            diameter: None,
            side: None,
            width: Some(150),
            height: None,
            size: None,
            min_space: 2.0,
            stroke_thickness: 1.0,
            algorithm: MaskAlgorithmType::Advanced,
            rdp_level: 3,
        };
        assert!(args.resolve_dimensions().is_err());
    }
}


