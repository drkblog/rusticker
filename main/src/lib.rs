use clap::ValueEnum;
use ::image::GenericImageView;
use printpdf::*;
use std::path::PathBuf;
use mask_generator::{MaskAlgorithm, BasicTracer, AdvancedTracer, CurvesTracer};

pub mod stickerize;
pub use stickerize::remove_background;

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

