use clap::ValueEnum;
use ::image::GenericImageView;
use printpdf::*;
use std::path::PathBuf;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FigureType {
    Square,
    Circle,
}

pub fn bake_grid(
    figure: FigureType,
    size_px: u32,
    dpi: u32,
    min_space_mm: f64,
    stroke_thickness_mm: f64,
    output_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    // A4 dimensions: 210mm x 297mm
    let page_width_mm = 210.0f32;
    let page_height_mm = 297.0f32;

    // Convert A4 dimensions to PDF points (1 inch = 25.4 mm, 1 inch = 72 points)
    let page_width_pt = (page_width_mm as f64) / 25.4 * 72.0;
    let page_height_pt = (page_height_mm as f64) / 25.4 * 72.0;

    // Convert figure size from pixels to PDF points
    let size_pt = (size_px as f64) / (dpi as f64) * 72.0;

    // Set layout parameters: 10mm margins, min_space_mm gap between figures
    let margin_mm = 10.0f64;
    let gap_mm = min_space_mm;

    let margin_pt = margin_mm / 25.4 * 72.0;
    let gap_pt = gap_mm / 25.4 * 72.0;

    let available_width = page_width_pt - 2.0 * margin_pt;
    let available_height = page_height_pt - 2.0 * margin_pt;

    if size_pt > available_width || size_pt > available_height {
        eprintln!(
            "Error: Figure size of {} pixels ({:.2} pt) at {} DPI is larger than available page area (width: {:.2} pt, height: {:.2} pt).",
            size_px, size_pt, dpi, available_width, available_height
        );
        return Err("Figure size exceeds available page space.".into());
    }

    // Number of columns and rows
    let cols = ((available_width + gap_pt) / (size_pt + gap_pt)).floor() as usize;
    let rows = ((available_height + gap_pt) / (size_pt + gap_pt)).floor() as usize;

    if cols == 0 || rows == 0 {
        return Err(
            "No figures could fit on the page under the current margins and spacing.".into(),
        );
    }

    println!(
        "Baking grid of {}x{} = {} figures (size: {} px / {:.2} pt, DPI: {}) to {:?}",
        cols,
        rows,
        cols * rows,
        size_px,
        size_pt,
        dpi,
        output_path
    );

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

    for r in 0..rows {
        for c in 0..cols {
            // Calculate coordinates for top-left aligned placement of bounding box
            let x = margin_pt + (c as f64) * (size_pt + gap_pt);
            let y = page_height_pt - margin_pt - size_pt - (r as f64) * (size_pt + gap_pt);

            match figure {
                FigureType::Square => {
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
                                x: Pt((x + size_pt) as f32),
                                y: Pt(y as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((x + size_pt) as f32),
                                y: Pt((y + size_pt) as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt(x as f32),
                                y: Pt((y + size_pt) as f32),
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
                    let cx = x + size_pt / 2.0;
                    let cy = y + size_pt / 2.0;
                    let radius = size_pt / 2.0;
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
            }
        }
    }

    // End the layer
    ops.push(Op::EndLayer);

    // Create the page with A4 dimensions (Portrait)
    let page = PdfPage::new(Mm(page_width_mm), Mm(page_height_mm), ops);

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

    std::fs::write(output_path, pdf_bytes)?;

    Ok(())
}

pub fn compose_grid(
    figure: FigureType,
    input_path: PathBuf,
    size_px: u32,
    dpi: u32,
    min_space_mm: f64,
    stroke_thickness_mm: f64,
    output_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Open the image
    let img = ::image::ImageReader::open(&input_path)?
        .with_guessed_format()?
        .decode()?;
    
    // 2. Validate dimensions
    let (width, height) = img.dimensions();
    if width < size_px || height < size_px {
        return Err("Input image is smaller than the specified size".into());
    }

    // 3. Center crop the image to size_px x size_px
    let cropped = if width == size_px && height == size_px {
        img
    } else {
        let x = (width - size_px) / 2;
        let y = (height - size_px) / 2;
        img.crop_imm(x, y, size_px, size_px)
    };

    // 4. Encode cropped image to PNG bytes in-memory
    let mut png_bytes: Vec<u8> = Vec::new();
    cropped.write_to(&mut std::io::Cursor::new(&mut png_bytes), ::image::ImageFormat::Png)?;

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
    let size_pt = (size_px as f64) / (dpi as f64) * 72.0;

    // Set layout parameters: 10mm margins, min_space_mm gap between figures
    let margin_mm = 10.0f64;
    let gap_mm = min_space_mm;

    let margin_pt = margin_mm / 25.4 * 72.0;
    let gap_pt = gap_mm / 25.4 * 72.0;

    let available_width = page_width_pt - 2.0 * margin_pt;
    let available_height = page_height_pt - 2.0 * margin_pt;

    if size_pt > available_width || size_pt > available_height {
        eprintln!(
            "Error: Figure size of {} pixels ({:.2} pt) at {} DPI is larger than available page area (width: {:.2} pt, height: {:.2} pt).",
            size_px, size_pt, dpi, available_width, available_height
        );
        return Err("Figure size exceeds available page space.".into());
    }

    // Number of columns and rows
    let cols = ((available_width + gap_pt) / (size_pt + gap_pt)).floor() as usize;
    let rows = ((available_height + gap_pt) / (size_pt + gap_pt)).floor() as usize;

    if cols == 0 || rows == 0 {
        return Err(
            "No figures could fit on the page under the current margins and spacing.".into(),
        );
    }

    println!(
        "Composing grid of {}x{} = {} figures (size: {} px / {:.2} pt, DPI: {}) to {:?}",
        cols,
        rows,
        cols * rows,
        size_px,
        size_pt,
        dpi,
        output_path
    );

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
            let x = margin_pt + (c as f64) * (size_pt + gap_pt);
            let y = page_height_pt - margin_pt - size_pt - (r as f64) * (size_pt + gap_pt);

            match figure {
                FigureType::Square => {
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
                                x: Pt((x + size_pt) as f32),
                                y: Pt(y as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt((x + size_pt) as f32),
                                y: Pt((y + size_pt) as f32),
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Pt(x as f32),
                                y: Pt((y + size_pt) as f32),
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
                    let cx = x + size_pt / 2.0;
                    let cy = y + size_pt / 2.0;
                    let radius = size_pt / 2.0;
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
            }
        }
    }

    // End Vector Layer
    ops.push(Op::EndLayer);

    // 2. Render the raster layer
    ops.push(Op::BeginLayer {
        layer_id: raster_layer_id,
    });

    let scale_factor = (300.0 / (dpi as f64)) as f32;

    for r in 0..rows {
        for c in 0..cols {
            // Calculate coordinates for top-left aligned placement of image
            let x = margin_pt + (c as f64) * (size_pt + gap_pt);
            let y = page_height_pt - margin_pt - size_pt - (r as f64) * (size_pt + gap_pt);

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

    std::fs::write(output_path, pdf_bytes)?;

    Ok(())
}

