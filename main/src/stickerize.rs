use std::path::{Path, PathBuf};
use std::fs::File;
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use tract_onnx::prelude::*;
use tract_onnx::prelude::tract_ndarray::Array;

fn get_default_model_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "Could not find home directory environment variable (USERPROFILE or HOME).")?;
    let mut path = PathBuf::from(home);
    path.push(".rusticker");
    path.push("models");
    std::fs::create_dir_all(&path)?;
    path.push("u2netp.onnx");
    Ok(path)
}

fn download_default_model(dest: &Path, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2netp.onnx";
    if verbose {
        println!("[VERBOSE] Downloading default ONNX model from {} to {:?}", url, dest);
    } else {
        println!("Downloading default U2Netp ONNX model (~4.7 MB)...");
    }

    let response = ureq::get(url).call()?;
    if response.status() != 200 {
        return Err(format!("Failed to download model: HTTP {}", response.status()).into());
    }

    let mut file = File::create(dest)?;
    let mut reader = response.into_reader();
    std::io::copy(&mut reader, &mut file)?;

    println!("Model downloaded successfully!");
    Ok(())
}

fn prepare_input_tensor(img: &DynamicImage, model_w: u32, model_h: u32) -> Result<Tensor, Box<dyn std::error::Error>> {
    let resized = img.resize_exact(model_w, model_h, image::imageops::FilterType::Triangle);
    let rgb = resized.to_rgb8();
    let raw = rgb.as_raw();

    // Standard U2-Net normalization: (val - mean) / std
    let mean = [0.485, 0.456, 0.406];
    let std = [0.229, 0.224, 0.225];

    let mut flat_data = vec![0.0f32; 1 * 3 * model_h as usize * model_w as usize];
    for y in 0..model_h as usize {
        for x in 0..model_w as usize {
            for c in 0..3 {
                let pixel_val = raw[(y * model_w as usize + x) * 3 + c] as f32 / 255.0;
                let normalized = (pixel_val - mean[c]) / std[c];
                let index = c * (model_h as usize * model_w as usize) + y * model_w as usize + x;
                flat_data[index] = normalized;
            }
        }
    }

    let shape = (1, 3, model_h as usize, model_w as usize);
    let array = Array::from_shape_vec(shape, flat_data)
        .map_err(|e| format!("ndarray shape error: {}", e))?;
    let tensor: Tensor = array.into();
    Ok(tensor)
}

pub fn remove_background(
    input_path: PathBuf,
    output_path: PathBuf,
    model_path: Option<PathBuf>,
    force: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if output_path.exists() && !force {
        return Err(format!(
            "Output file '{}' already exists. Use --force to overwrite.",
            output_path.display()
        )
        .into());
    }

    // 1. Load image
    if verbose {
        println!("[VERBOSE] Step: Loading input image from {:?}", input_path);
    }
    let img = image::ImageReader::open(&input_path)?
        .with_guessed_format()?
        .decode()?;

    let (w_orig, h_orig) = img.dimensions();
    if verbose {
        println!("[VERBOSE] Original image dimensions: {}x{} px", w_orig, h_orig);
    }

    // 2. Determine and resolve ONNX model path
    let resolved_model_path = match model_path {
        Some(p) => {
            if verbose {
                println!("[VERBOSE] Using custom model path: {:?}", p);
            }
            p
        }
        None => {
            let default_path = get_default_model_path()?;
            if !default_path.exists() {
                download_default_model(&default_path, verbose)?;
            } else if verbose {
                println!("[VERBOSE] Using cached default model at {:?}", default_path);
            }
            default_path
        }
    };

    // 3. Load ONNX model using tract-onnx
    if verbose {
        println!("[VERBOSE] Step: Loading ONNX model into tract...");
    }
    let model = tract_onnx::onnx()
        .model_for_path(&resolved_model_path)?
        .into_optimized()?
        .into_runnable()?;

    // 4. Retrieve input shape dynamically if possible
    let mut model_w = 320;
    let mut model_h = 320;
    if let Some(fact) = model.model().input_fact(0).ok() {
        if let Some(concrete) = fact.shape.as_concrete() {
            if concrete.len() == 4 {
                model_h = concrete[2] as u32;
                model_w = concrete[3] as u32;
            }
        }
    }
    if verbose {
        println!(
            "[VERBOSE] Model expected input dimensions: {}x{} px",
            model_w, model_h
        );
    }

    // 5. Preprocess image and perform inference
    if verbose {
        println!("[VERBOSE] Step: Preprocessing image for inference...");
    }
    let input_tensor = prepare_input_tensor(&img, model_w, model_h)?;

    if verbose {
        println!("[VERBOSE] Step: Running model inference...");
    }
    let mut result = model.run(tvec![input_tensor.into()])?;

    if verbose {
        println!("[VERBOSE] Step: Model execution completed. Parsing output mask...");
    }
    let mask_tensor = result.remove(0).into_tensor();
    let mask_array = mask_tensor.to_plain_array_view::<f32>()?;

    // 6. Post-process: Map mask back to original size & save as transparent PNG
    if verbose {
        println!("[VERBOSE] Step: Applying transparency mask to original image...");
    }
    let mut out_img = RgbaImage::new(w_orig, h_orig);

    for y in 0..h_orig {
        for x in 0..w_orig {
            let pixel = img.get_pixel(x, y);
            // Map original coordinate (x, y) to mask coordinate (mx, my)
            let mx = (x as f64 / w_orig as f64 * model_w as f64).clamp(0.0, (model_w - 1) as f64) as usize;
            let my = (y as f64 / h_orig as f64 * model_h as f64).clamp(0.0, (model_h - 1) as f64) as usize;

            let prob = mask_array[[0, 0, my, mx]].clamp(0.0, 1.0);

            // Scale original alpha channel (or default to 255 if original didn't have alpha)
            let old_alpha = if pixel.0.len() > 3 { pixel[3] } else { 255 };
            let new_alpha = (old_alpha as f32 * prob) as u8;

            out_img.put_pixel(x, y, Rgba([pixel[0], pixel[1], pixel[2], new_alpha]));
        }
    }

    if verbose {
        println!("[VERBOSE] Saving output transparent PNG to {:?}", output_path);
    }
    out_img.save(&output_path)?;
    println!("Saved background-removed image to {:?}", output_path);

    Ok(())
}
