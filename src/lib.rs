use clap::ValueEnum;
use ::image::GenericImageView;
use printpdf::*;
use std::path::PathBuf;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FigureType {
    Square,
    Circle,
    Mask,
}

pub fn bake_grid(
    figure: FigureType,
    size_px: u32,
    dpi: u32,
    min_space_mm: f64,
    stroke_thickness_mm: f64,
    output_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    if figure == FigureType::Mask {
        return Err("The 'mask' figure type requires an input image and is not supported in the bake subcommand.".into());
    }
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
                FigureType::Mask => unreachable!(),
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
    size_px: Option<u32>,
    dpi: u32,
    min_space_mm: f64,
    stroke_thickness_mm: f64,
    output_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Open the image
    let img = ::image::ImageReader::open(&input_path)?
        .with_guessed_format()?
        .decode()?;
    
    // 2. Determine target size and crop if needed
    let (width, height) = img.dimensions();
    let (cropped, actual_size_px) = if let Some(s) = size_px {
        if width < s || height < s {
            return Err("Input image is smaller than the specified size".into());
        }
        let cropped_img = if width == s && height == s {
            img
        } else {
            let x = (width - s) / 2;
            let y = (height - s) / 2;
            img.crop_imm(x, y, s, s)
        };
        (cropped_img, s)
    } else {
        (img, std::cmp::max(width, height))
    };

    let (cropped_width, cropped_height) = cropped.dimensions();

    let mut loops = Vec::new();
    if figure == FigureType::Mask {
        let bg_color = cropped.get_pixel(0, 0);
        let mut visited = vec![vec![false; cropped_width as usize]; cropped_height as usize];
        let mut q = std::collections::VecDeque::new();
        if cropped_width > 0 && cropped_height > 0 {
            q.push_back((0, 0));
            visited[0][0] = true;
        }
        while let Some((cx, cy)) = q.pop_front() {
            for &(nx, ny) in &[
                (cx as i32 + 1, cy as i32),
                (cx as i32 - 1, cy as i32),
                (cx as i32, cy as i32 + 1),
                (cx as i32, cy as i32 - 1),
            ] {
                if nx >= 0 && nx < cropped_width as i32 && ny >= 0 && ny < cropped_height as i32 {
                    let ux = nx as usize;
                    let uy = ny as usize;
                    if !visited[uy][ux] && cropped.get_pixel(nx as u32, ny as u32) == bg_color {
                        visited[uy][ux] = true;
                        q.push_back((nx as u32, ny as u32));
                    }
                }
            }
        }

        let grid_w = cropped_width as i32 + 2;
        let grid_h = cropped_height as i32 + 2;
        let mut is_bg = vec![vec![false; grid_w as usize]; grid_h as usize];
        for sy in 0..grid_h {
            for sx in 0..grid_w {
                let x = sx - 1;
                let y = sy - 1;
                is_bg[sy as usize][sx as usize] = x < 0 || x >= cropped_width as i32 || y < 0 || y >= cropped_height as i32 || visited[y as usize][x as usize];
            }
        }

        let mut segments = Vec::new();
        let get_bg = |sx: i32, sy: i32| -> bool {
            if sx < 0 || sx >= grid_w || sy < 0 || sy >= grid_h {
                true
            } else {
                is_bg[sy as usize][sx as usize]
            }
        };

        for sy in 0..grid_h {
            for sx in 0..grid_w {
                if get_bg(sx, sy) {
                    if !get_bg(sx + 1, sy) {
                        segments.push(((sx + 1, sy), (sx + 1, sy + 1)));
                    }
                    if !get_bg(sx - 1, sy) {
                        segments.push(((sx, sy + 1), (sx, sy)));
                    }
                    if !get_bg(sx, sy + 1) {
                        segments.push(((sx + 1, sy + 1), (sx, sy + 1)));
                    }
                    if !get_bg(sx, sy - 1) {
                        segments.push(((sx, sy), (sx + 1, sy)));
                    }
                }
            }
        }

        use std::collections::HashMap;
        let mut adj: HashMap<(i32, i32), Vec<(i32, i32)>> = HashMap::new();
        for &(start, end) in &segments {
            adj.entry(start).or_default().push(end);
        }

        while !adj.is_empty() {
            let &start_pt = adj.keys().next().unwrap();
            if adj.get(&start_pt).map_or(true, |v| v.is_empty()) {
                adj.remove(&start_pt);
                continue;
            }
            
            let mut curr = start_pt;
            let mut path = vec![curr];
            let mut success = false;
            
            while let Some(options) = adj.get_mut(&curr) {
                if options.is_empty() {
                    break;
                }
                let next_pt = options.pop().unwrap();
                if options.is_empty() {
                    adj.remove(&curr);
                }
                curr = next_pt;
                path.push(curr);
                if curr == start_pt {
                    success = true;
                    break;
                }
            }
            
            if success && path.len() > 1 {
                loops.push(path);
            } else {
                adj.remove(&start_pt);
            }
        }
    }

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
    let (width_pt, height_pt) = if figure == FigureType::Mask {
        (
            (cropped_width as f64) / (dpi as f64) * 72.0,
            (cropped_height as f64) / (dpi as f64) * 72.0,
        )
    } else {
        let size_pt = (actual_size_px as f64) / (dpi as f64) * 72.0;
        (size_pt, size_pt)
    };

    // Set layout parameters: 10mm margins, min_space_mm gap between figures
    let margin_mm = 10.0f64;
    let gap_mm = min_space_mm;

    let margin_pt = margin_mm / 25.4 * 72.0;
    let gap_pt = gap_mm / 25.4 * 72.0;

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
            "Composing grid of {}x{} = {} figures (size: {} px / {:.2} pt, DPI: {}) to {:?}",
            cols,
            rows,
            cols * rows,
            actual_size_px,
            width_pt,
            dpi,
            output_path
        );
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
                        for &(cx, cy) in lp {
                            let lx = x + (cx - 1) as f64 * scale;
                            let ly = y + (cropped_height as f64 - (cy - 1) as f64) * scale;
                            points.push(LinePoint {
                                p: Point {
                                    x: Pt(lx as f32),
                                    y: Pt(ly as f32),
                                },
                                bezier: false,
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

