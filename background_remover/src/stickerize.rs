use std::path::{Path, PathBuf};
use std::fs::File;
use std::collections::HashMap;
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use tract_onnx::prelude::*;
use tract_onnx::prelude::tract_ndarray::Array;
use tract_core::ops::konst::Const;
use tract_core::ops::array::GatherNd;
use tract_core::internal::*;
use tract_hir::infer::{InferenceOp, InferenceNode, ShapeFactoid, Factoid};
use crate::{ModelType, OutputFormat};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MyGatherNd {
    pub batch_dims: usize,
}

impl Op for MyGatherNd {
    fn name(&self) -> StaticName {
        "MyGatherNd".into()
    }
    
    fn as_typed(&self) -> Option<&dyn TypedOp> {
        None
    }
}

impl EvalOp for MyGatherNd {
    fn is_stateless(&self) -> bool {
        true
    }
    
    fn eval(&self, _inputs: TVec<TValue>) -> TractResult<TVec<TValue>> {
        bail!("MyGatherNd is an inference op, should be typed before execution")
    }
}

impl InferenceOp for MyGatherNd {
    fn infer_facts(
        &mut self,
        inputs: TVec<&InferenceFact>,
        outputs: TVec<&InferenceFact>,
        observed: TVec<&InferenceFact>,
    ) -> TractResult<(TVec<InferenceFact>, TVec<InferenceFact>, TVec<InferenceFact>)> {
        let data = inputs[0];
        let indices = inputs[1];
        
        let mut output = outputs[0].clone();
        output.datum_type = data.datum_type;

        if !data.shape.is_open() && !indices.shape.is_open() {
            let data_dims = data.shape.dims().cloned().collect::<TVec<_>>();
            let indices_dims = indices.shape.dims().cloned().collect::<TVec<_>>();
            let q = indices_dims.len();
            let r = data_dims.len();
            let b = self.batch_dims;
            
            if q > 0 && r > b {
                let k_fact = &indices_dims[q - 1];
                if let Some(k_dim) = k_fact.concretize() {
                    if let Ok(k) = k_dim.to_usize() {
                        if r >= b + k {
                            let mut dims = tvec![];
                            for i in 0..(q - 1) {
                                dims.push(indices_dims[i].clone());
                            }
                            for i in (b + k)..r {
                                dims.push(data_dims[i].clone());
                            }
                            output.shape = ShapeFactoid::closed(dims);
                        }
                    }
                }
            }
        }
        
        Ok((
            tvec![data.clone(), indices.clone()],
            tvec![output],
            observed.into_iter().cloned().collect(),
        ))
    }

    fn as_op(&self) -> &dyn Op {
        self
    }

    fn as_op_mut(&mut self) -> &mut dyn Op {
        self
    }

    fn to_typed(
        &self,
        _source: &InferenceModel,
        node: &InferenceNode,
        target: &mut TypedModel,
        mapping: &HashMap<OutletId, OutletId>,
    ) -> TractResult<TVec<OutletId>> {
        let op = GatherNd::new(self.batch_dims);
        let inputs = node.inputs.iter().map(|i| mapping[i]).collect::<TVec<_>>();
        target.wire_node(&node.name, op, &inputs)
    }
}


fn get_model_path(model_type: ModelType) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "Could not find home directory environment variable (USERPROFILE or HOME).")?;
    let mut path = PathBuf::from(home);
    path.push(".rusticker");
    path.push("models");
    std::fs::create_dir_all(&path)?;
    let filename = match model_type {
        ModelType::U2netp => "u2netp.onnx",
        ModelType::Rmbg => "rmbg.onnx",
        ModelType::Birefnet => "birefnet.onnx",
    };
    path.push(filename);
    Ok(path)
}

fn download_model(model_type: ModelType, dest: &Path, verbose: bool, quiet: bool) -> Result<(), Box<dyn std::error::Error>> {
    let (url, name, size_str) = match model_type {
        ModelType::U2netp => (
            "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2netp.onnx",
            "U2Netp",
            "~4.7 MB"
        ),
        ModelType::Rmbg => (
            "https://huggingface.co/briaai/RMBG-1.4/resolve/main/onnx/model.onnx",
            "RMBG-1.4",
            "~176 MB"
        ),
        ModelType::Birefnet => (
            "https://github.com/danielgatis/rembg/releases/download/v0.0.0/BiRefNet-general-bb_swin_v1_tiny-epoch_232.onnx",
            "BiRefNet",
            "~224 MB"
        ),
    };

    if !quiet {
        if verbose {
            println!("[VERBOSE] Downloading {} ONNX model from {} to {:?}", name, url, dest);
        } else {
            println!("Downloading {} ONNX model ({})... This may take a moment...", name, size_str);
        }
    }

    let response = ureq::get(url).call()?;
    if response.status() != 200 {
        return Err(format!("Failed to download model: HTTP {}", response.status()).into());
    }

    let mut file = File::create(dest)?;
    let mut reader = response.into_reader();
    std::io::copy(&mut reader, &mut file)?;

    if !quiet {
        println!("Model downloaded successfully!");
    }
    Ok(())
}

fn prepare_input_tensor(
    img: &DynamicImage,
    model_w: u32,
    model_h: u32,
    model_type: ModelType,
) -> Result<Tensor, Box<dyn std::error::Error>> {
    let resized = img.resize_exact(model_w, model_h, image::imageops::FilterType::Triangle);
    let rgb = resized.to_rgb8();
    let raw = rgb.as_raw();

    let mut flat_data = vec![0.0f32; 1 * 3 * model_h as usize * model_w as usize];

    match model_type {
        ModelType::U2netp | ModelType::Birefnet => {
            // Standard U2-Net / ImageNet normalization: (val - mean) / std
            let mean = [0.485, 0.456, 0.406];
            let std = [0.229, 0.224, 0.225];

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
        }
        ModelType::Rmbg => {
            // RMBG-1.4 normalization: val / 255.0 - 0.5
            for y in 0..model_h as usize {
                for x in 0..model_w as usize {
                    for c in 0..3 {
                        let pixel_val = raw[(y * model_w as usize + x) * 3 + c] as f32 / 255.0;
                        let normalized = pixel_val - 0.5;
                        let index = c * (model_h as usize * model_w as usize) + y * model_w as usize + x;
                        flat_data[index] = normalized;
                    }
                }
            }
        }
    }

    let shape = (1, 3, model_h as usize, model_w as usize);
    let array = Array::from_shape_vec(shape, flat_data)
        .map_err(|e| format!("ndarray shape error: {}", e))?;
    let tensor: Tensor = array.into();
    Ok(tensor)
}

fn load_and_optimize_model(
    model_path: &Path,
    model_w: u32,
    model_h: u32,
    model_type: ModelType,
    verbose: bool,
    quiet: bool,
) -> Result<TypedModel, Box<dyn std::error::Error>> {
    if verbose && !quiet {
        println!("[VERBOSE] Step: Loading ONNX model into tract and setting input fact...");
    }
    let model = if model_type == ModelType::Birefnet {
        let mut raw_model = tract_onnx::onnx().model_for_path(model_path)?;
        
        if verbose && !quiet {
            println!("[VERBOSE] Patching BiRefNet graph...");
        }
        
        let mut replaced_gather_count = 0;
        for node in &mut raw_model.nodes {
            if let Some(gather_nd_op) = node.op_as::<GatherNd>() {
                let batch_dims = gather_nd_op.batch_dims;
                let new_op = MyGatherNd { batch_dims };
                node.op = Box::new(new_op);
                replaced_gather_count += 1;
            }
        }
        if verbose && !quiet {
            println!("[VERBOSE] Replaced {} GatherNd nodes.", replaced_gather_count);
        }

        let mut const_node_indices = std::collections::HashSet::new();
        for node in &raw_model.nodes {
            if node.name.contains("atrous_conv/Clip") {
                const_node_indices.insert(node.inputs[1].node);
                const_node_indices.insert(node.inputs[2].node);
            }
        }
        
        let mut patched_count = 0;
        for idx in const_node_indices {
            let node = &mut raw_model.nodes[idx];
            if let Some(const_op) = node.op_as::<Const>() {
                let tensor = const_op.val();
                
                if tensor.datum_type() == DatumType::I64 {
                    let ints = unsafe { tensor.as_slice_unchecked::<i64>() };
                    let val = ints[0];
                    let tdim_tensor = tensor0(TDim::from(val));

                    let new_op = Const::new(std::sync::Arc::new(tdim_tensor))?;
                    node.op = Box::new(new_op);
                    patched_count += 1;
                }
            }
        }
        if verbose && !quiet {
            println!("[VERBOSE] Patched {} constant ops to TDim.", patched_count);
        }

        raw_model
            .with_input_fact(
                0,
                InferenceFact::dt_shape(f32::datum_type(), tvec!(1, 3, model_h as usize, model_w as usize))
            )?
            .into_optimized()?
    } else {
        tract_onnx::onnx()
            .model_for_path(model_path)?
            .with_input_fact(
                0,
                InferenceFact::dt_shape(f32::datum_type(), tvec!(1, 3, model_h as usize, model_w as usize))
            )?
            .into_optimized()?
    };

    Ok(model)
}

pub fn parse_hex_color(hex: &str) -> Result<Rgba<u8>, String> {
    let hex = hex.trim();
    let hex = if hex.starts_with('#') { &hex[1..] } else { hex };

    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16)
                .map_err(|_| format!("Invalid hex digit: '{}'", &hex[0..1]))?;
            let g = u8::from_str_radix(&hex[1..2], 16)
                .map_err(|_| format!("Invalid hex digit: '{}'", &hex[1..2]))?;
            let b = u8::from_str_radix(&hex[2..3], 16)
                .map_err(|_| format!("Invalid hex digit: '{}'", &hex[2..3]))?;
            Ok(Rgba([r * 17, g * 17, b * 17, 255]))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| format!("Invalid hex byte: '{}'", &hex[0..2]))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| format!("Invalid hex byte: '{}'", &hex[2..4]))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| format!("Invalid hex byte: '{}'", &hex[4..6]))?;
            Ok(Rgba([r, g, b, 255]))
        }
        _ => Err(format!(
            "Invalid hex color length (expected 3 or 6 hex digits, got '{}')",
            hex
        )),
    }
}

pub fn add_border(
    img: &RgbaImage,
    thickness: u32,
    color: Rgba<u8>,
    antialiasing: bool,
) -> RgbaImage {
    let (width, height) = img.dimensions();
    let border_img = img.clone();

    if thickness == 0 {
        return border_img;
    }

    // We consider a pixel to be part of the "subject" if its alpha is >= 10.
    let alpha_threshold = 10u8;

    // Find the bounding box of the subject pixels to optimize the search space.
    let mut min_x = width;
    let mut max_x = 0;
    let mut min_y = height;
    let mut max_y = 0;
    let mut has_subject = false;

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            if pixel[3] >= alpha_threshold {
                if x < min_x { min_x = x; }
                if x > max_x { max_x = x; }
                if y < min_y { min_y = y; }
                if y > max_y { max_y = y; }
                has_subject = true;
            }
        }
    }

    if !has_subject {
        return border_img;
    }

    // Expand the bounding box by the thickness of the border.
    // If antialiasing is active, expand by thickness + 1 to account for fractional pixel coverage.
    let expansion = if antialiasing { thickness + 1 } else { thickness };
    let start_x = if min_x > expansion { min_x - expansion } else { 0 };
    let end_x = if max_x + expansion < width { max_x + expansion } else { width - 1 };
    let start_y = if min_y > expansion { min_y - expansion } else { 0 };
    let end_y = if max_y + expansion < height { max_y + expansion } else { height - 1 };

    // Create a new image to store the final result.
    let mut result_img = RgbaImage::new(width, height);

    // For each pixel in the expanded bounding box, check if it lies within `thickness` distance of any subject pixel.
    for y in start_y..=end_y {
        for x in start_x..=end_x {
            let original_pixel = img.get_pixel(x, y);

            // If the pixel is already fully opaque subject, we don't need to check neighbors.
            if original_pixel[3] >= 255 {
                result_img.put_pixel(x, y, *original_pixel);
                continue;
            }

            // Search the neighborhood for the minimum distance to a subject pixel.
            let mut min_d = f64::INFINITY;
            let thickness_i = thickness as i32;

            if original_pixel[3] >= alpha_threshold {
                min_d = 0.0;
            } else {
                let limit = if antialiasing { thickness_i + 1 } else { thickness_i };
                let r2 = (limit * limit) as i32;

                for dy in -limit..=limit {
                    let ny = y as i32 + dy;
                    if ny < 0 || ny >= height as i32 {
                        continue;
                    }

                    let dy2 = dy * dy;
                    if dy2 > r2 {
                        continue;
                    }

                    for dx in -limit..=limit {
                        let nx = x as i32 + dx;
                        if nx < 0 || nx >= width as i32 {
                            continue;
                        }

                        let dx2 = dx * dx;
                        let dist2 = dx2 + dy2;
                        if dist2 <= r2 {
                            let neighbor_pixel = img.get_pixel(nx as u32, ny as u32);
                            if neighbor_pixel[3] >= alpha_threshold {
                                let d = (dist2 as f64).sqrt();
                                if d < min_d {
                                    min_d = d;
                                }
                            }
                        }
                    }
                }
            }

            let mut weight = 0.0;
            if min_d != f64::INFINITY {
                if antialiasing {
                    let t = thickness as f64;
                    if min_d <= t - 0.5 {
                        weight = 1.0;
                    } else if min_d < t + 0.5 {
                        weight = t + 0.5 - min_d;
                    } else {
                        weight = 0.0;
                    }
                } else {
                    if min_d <= thickness as f64 {
                        weight = 1.0;
                    } else {
                        weight = 0.0;
                    }
                }
            }

            if weight > 0.0 {
                // Blend the original pixel over the border color, scaled by border opacity weight.
                let alpha_s = original_pixel[3] as f32 / 255.0;
                let alpha_border = weight as f32;
                let alpha_final = alpha_s + (1.0 - alpha_s) * alpha_border;

                if alpha_final > 0.0 {
                    let r = ((alpha_s * original_pixel[0] as f32 + (1.0 - alpha_s) * alpha_border * color[0] as f32) / alpha_final) as u8;
                    let g = ((alpha_s * original_pixel[1] as f32 + (1.0 - alpha_s) * alpha_border * color[1] as f32) / alpha_final) as u8;
                    let b = ((alpha_s * original_pixel[2] as f32 + (1.0 - alpha_s) * alpha_border * color[2] as f32) / alpha_final) as u8;
                    let a = (alpha_final * 255.0) as u8;

                    result_img.put_pixel(x, y, Rgba([r, g, b, a]));
                } else {
                    result_img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                }
            } else {
                // Not in the border, keep original transparent / empty pixel.
                result_img.put_pixel(x, y, *original_pixel);
            }
        }
    }

    // Copy any pixels outside the bounding box that weren't processed (they are transparent).
    for y in 0..height {
        for x in 0..width {
            if x < start_x || x > end_x || y < start_y || y > end_y {
                result_img.put_pixel(x, y, *img.get_pixel(x, y));
            }
        }
    }

    result_img
}

pub fn remove_background(
    input_path: PathBuf,
    output_path: PathBuf,
    model_type: ModelType,
    force: bool,
    verbose: bool,
    use_cuda: bool,
    quiet: bool,
    border: Option<u32>,
    border_color: Option<String>,
    antialiasing: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    if output_path.exists() && !force {
        return Err(format!(
            "Output file '{}' already exists. Use --force to overwrite.",
            output_path.display()
        )
        .into());
    }

    // 1. Load image
    if !quiet {
        if verbose {
            println!("[VERBOSE] Step: Loading input image from {:?}", input_path);
        } else {
            println!("Step 1/6: Loading input image from {:?}", input_path);
        }
    }
    let img = image::ImageReader::open(&input_path)?
        .with_guessed_format()?
        .decode()?;

    let (w_orig, h_orig) = img.dimensions();
    if verbose && !quiet {
        println!("[VERBOSE] Original image dimensions: {}x{} px", w_orig, h_orig);
    }

    // 2. Determine and resolve ONNX model path
    if !quiet {
        if verbose {
            println!("[VERBOSE] Step: Resolving ONNX model path...");
        } else {
            println!("Step 2/6: Loading neural network model ({:?})...", model_type);
        }
    }
    let resolved_model_path = get_model_path(model_type)?;
    if !resolved_model_path.exists() {
        download_model(model_type, &resolved_model_path, verbose, quiet)?;
    } else if verbose && !quiet {
        println!("[VERBOSE] Using cached model at {:?}", resolved_model_path);
    }

    // 3. Set input dimensions based on model type
    let (model_w, model_h) = match model_type {
        ModelType::U2netp => (320, 320),
        ModelType::Rmbg | ModelType::Birefnet => (1024, 1024),
    };
    if verbose && !quiet {
        println!(
            "[VERBOSE] Model expected input dimensions: {}x{} px",
            model_w, model_h
        );
    }

    // 4. Resolve runtime (CUDA with fallback to CPU if requested, otherwise CPU) and prepare runnable model
    let (model, device_name) = if use_cuda {
        if let Ok(Some(cuda_runtime)) = tract_onnx::prelude::runtime_for_name("cuda") {
            if verbose && !quiet {
                println!("[VERBOSE] CUDA runtime found in registry. Attempting compilation...");
            }
            let model = load_and_optimize_model(&resolved_model_path, model_w, model_h, model_type, verbose, quiet)?;
            match cuda_runtime.prepare(model) {
                Ok(runnable_model) => (runnable_model, "CUDA"),
                Err(e) => {
                    if verbose && !quiet {
                        println!("[VERBOSE] Failed to prepare model for CUDA runtime: {}. Falling back to CPU.", e);
                    }
                    let model = load_and_optimize_model(&resolved_model_path, model_w, model_h, model_type, verbose, quiet)?;
                    let cpu_runtime = tract_onnx::prelude::runtime_for_name("cpu")
                        .or_else(|_| tract_onnx::prelude::runtime_for_name("default"))?
                        .ok_or_else(|| "No CPU/default runtime found in tract registry")?;
                    (cpu_runtime.prepare(model)?, "CPU (CUDA failed)")
                }
            }
        } else {
            if verbose && !quiet {
                println!("[VERBOSE] CUDA runtime requested but not found in registry. Using CPU.");
            }
            let model = load_and_optimize_model(&resolved_model_path, model_w, model_h, model_type, verbose, quiet)?;
            let cpu_runtime = tract_onnx::prelude::runtime_for_name("cpu")
                .or_else(|_| tract_onnx::prelude::runtime_for_name("default"))?
                .ok_or_else(|| "No CPU/default runtime found in tract registry")?;
            (cpu_runtime.prepare(model)?, "CPU")
        }
    } else {
        if verbose && !quiet {
            println!("[VERBOSE] CUDA runtime not requested. Using CPU.");
        }
        let model = load_and_optimize_model(&resolved_model_path, model_w, model_h, model_type, verbose, quiet)?;
        let cpu_runtime = tract_onnx::prelude::runtime_for_name("cpu")
            .or_else(|_| tract_onnx::prelude::runtime_for_name("default"))?
            .ok_or_else(|| "No CPU/default runtime found in tract registry")?;
        (cpu_runtime.prepare(model)?, "CPU")
    };

    if !quiet {
        if verbose {
            println!("[VERBOSE] Image processing will be done with: {}", device_name);
        } else {
            println!("Step 3/6: Preparing execution device (using {})...", device_name);
        }
    }

    // 5. Preprocess image and perform inference
    if !quiet {
        if verbose {
            println!("[VERBOSE] Step: Preprocessing image for inference...");
        } else {
            println!("Step 4/6: Preprocessing image for inference...");
        }
    }
    let input_tensor = prepare_input_tensor(&img, model_w, model_h, model_type)?;

    if !quiet {
        if verbose {
            println!("[VERBOSE] Step: Running model inference...");
        } else {
            println!("Step 5/6: Running model inference (this may take a few seconds)...");
        }
    }
    let mut result = model.run(tvec![input_tensor.into()])?;

    if verbose && !quiet {
        println!("[VERBOSE] Step: Model execution completed. Parsing output mask...");
    }
    let mask_tensor = result.remove(0).into_tensor();
    let mask_array = mask_tensor.to_plain_array_view::<f32>()?;

    // 6. Post-process: Map mask back to original size & save as transparent PNG
    if !quiet {
        if verbose {
            println!("[VERBOSE] Step: Applying transparency mask to original image...");
        } else {
            println!("Step 6/6: Applying transparency mask...");
        }
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

    // 7. Apply border if present
    let final_img = if let Some(thickness) = border {
        if thickness > 0 {
            if !quiet {
                if verbose {
                    println!("[VERBOSE] Step: Adding border of thickness {} px...", thickness);
                } else {
                    println!("Adding border of thickness {} px...", thickness);
                }
            }
            let color_str = border_color.as_deref().unwrap_or("#FFFFFF");
            let color = parse_hex_color(color_str)?;
            add_border(&out_img, thickness, color, antialiasing)
        } else {
            out_img
        }
    } else {
        out_img
    };

    let format_desc = match format {
        OutputFormat::Png => "PNG",
        OutputFormat::Webp => "WebP",
    };
    if verbose && !quiet {
        println!("[VERBOSE] Saving output transparent {} to {:?}", format_desc, output_path);
    }
    
    let img_format = match format {
        OutputFormat::Png => image::ImageFormat::Png,
        OutputFormat::Webp => image::ImageFormat::WebP,
    };
    final_img.save_with_format(&output_path, img_format)?;
    
    if !quiet {
        println!("Saved background-removed image to {:?}", output_path);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_resolution() {
        // CPU runtime should always be available
        let cpu_runtime = tract_onnx::prelude::runtime_for_name("cpu")
            .or_else(|_| tract_onnx::prelude::runtime_for_name("default"))
            .unwrap();
        assert!(cpu_runtime.is_some());

        // Check cuda runtime (might be None or Some depending on the system, but shouldn't panic)
        let cuda_runtime = tract_onnx::prelude::runtime_for_name("cuda");
        assert!(cuda_runtime.is_ok());
    }

    #[test]
    fn test_parse_hex_color() {
        // Standard 6 digits with and without #
        assert_eq!(parse_hex_color("#FFFFFF").unwrap(), Rgba([255, 255, 255, 255]));
        assert_eq!(parse_hex_color("ffffff").unwrap(), Rgba([255, 255, 255, 255]));
        assert_eq!(parse_hex_color("#22AA5E").unwrap(), Rgba([0x22, 0xAA, 0x5E, 255]));
        assert_eq!(parse_hex_color("22aa5e").unwrap(), Rgba([0x22, 0xAA, 0x5E, 255]));

        // Shorthand 3 digits with and without #
        assert_eq!(parse_hex_color("#FFF").unwrap(), Rgba([255, 255, 255, 255]));
        assert_eq!(parse_hex_color("000").unwrap(), Rgba([0, 0, 0, 255]));
        assert_eq!(parse_hex_color("#123").unwrap(), Rgba([17, 34, 51, 255]));

        // Invalid inputs
        assert!(parse_hex_color("white").is_err());
        assert!(parse_hex_color("#FFFFF").is_err());
        assert!(parse_hex_color("#22AA5G").is_err()); // 'G' is invalid hex
    }

    #[test]
    fn test_add_border() {
        // Create a 5x5 transparent image with one opaque pixel in the center (2, 2)
        let mut img = RgbaImage::new(5, 5);
        img.put_pixel(2, 2, Rgba([255, 0, 0, 255]));

        // Add 1px white border
        let bordered = add_border(&img, 1, Rgba([255, 255, 255, 255]), false);

        // Center pixel should remain unchanged
        assert_eq!(bordered.get_pixel(2, 2), &Rgba([255, 0, 0, 255]));

        // Adjacent pixels (2+1, 2) should be white border
        assert_eq!(bordered.get_pixel(3, 2), &Rgba([255, 255, 255, 255]));
        assert_eq!(bordered.get_pixel(1, 2), &Rgba([255, 255, 255, 255]));
        assert_eq!(bordered.get_pixel(2, 3), &Rgba([255, 255, 255, 255]));
        assert_eq!(bordered.get_pixel(2, 1), &Rgba([255, 255, 255, 255]));

        // Corners (3, 3) are at distance sqrt(2) = 1.414.
        // Since thickness is 1, a circle of radius 1 does not cover the corners.
        assert_eq!(bordered.get_pixel(3, 3), &Rgba([0, 0, 0, 0]));
    }

    #[test]
    fn test_add_border_antialiasing() {
        // Create a 5x5 transparent image with one opaque pixel in the center (2, 2)
        let mut img = RgbaImage::new(5, 5);
        img.put_pixel(2, 2, Rgba([255, 0, 0, 255]));

        // Add 1px white border with antialiasing enabled
        let bordered = add_border(&img, 1, Rgba([255, 255, 255, 255]), true);

        // Center pixel should remain unchanged
        assert_eq!(bordered.get_pixel(2, 2), &Rgba([255, 0, 0, 255]));

        // Adjacent pixels at distance 1.0: weight = 1.5 - 1.0 = 0.5.
        // Alpha = 0.5 * 255 = 127.
        assert_eq!(bordered.get_pixel(3, 2)[3], 127);
        assert_eq!(bordered.get_pixel(1, 2)[3], 127);
        assert_eq!(bordered.get_pixel(2, 3)[3], 127);
        assert_eq!(bordered.get_pixel(2, 1)[3], 127);

        // Diagonal pixels (3, 3) are at distance sqrt(2) = 1.4142.
        // weight = 1.5 - 1.4142 = 0.0858.
        // Alpha = 0.0858 * 255 = 21.
        assert_eq!(bordered.get_pixel(3, 3)[3], 21);
    }
}

