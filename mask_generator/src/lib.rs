use image::{DynamicImage, GenericImageView};
use std::collections::{HashMap, VecDeque};

/// Trait representing an algorithm for tracing the contour mask of an image.
pub trait MaskAlgorithm {
    /// Traces the boundary outline loops of the mask from the input image.
    /// Returns a list of loops, where each loop is a list of coordinates (x, y).
    fn trace_mask(
        &self,
        img: &DynamicImage,
        verbose: bool,
    ) -> Result<Vec<Vec<(i32, i32)>>, Box<dyn std::error::Error>>;
}

/// A mask tracing implementation using a flood-fill algorithm to detect
/// background pixels and contour tracing to extract boundary loops.
pub struct FloodFillTracer;

impl MaskAlgorithm for FloodFillTracer {
    fn trace_mask(
        &self,
        img: &DynamicImage,
        verbose: bool,
    ) -> Result<Vec<Vec<(i32, i32)>>, Box<dyn std::error::Error>> {
        let (width, height) = img.dimensions();
        if width == 0 || height == 0 {
            return Ok(Vec::new());
        }

        let bg_color = img.get_pixel(0, 0);
        let mut visited = vec![vec![false; width as usize]; height as usize];
        let mut q = VecDeque::new();
        q.push_back((0, 0));
        visited[0][0] = true;

        while let Some((cx, cy)) = q.pop_front() {
            for &(nx, ny) in &[
                (cx as i32 + 1, cy as i32),
                (cx as i32 - 1, cy as i32),
                (cx as i32, cy as i32 + 1),
                (cx as i32, cy as i32 - 1),
            ] {
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let ux = nx as usize;
                    let uy = ny as usize;
                    if !visited[uy][ux] && img.get_pixel(nx as u32, ny as u32) == bg_color {
                        visited[uy][ux] = true;
                        q.push_back((nx as u32, ny as u32));
                    }
                }
            }
        }

        let grid_w = width as i32 + 2;
        let grid_h = height as i32 + 2;
        let mut is_bg = vec![vec![false; grid_w as usize]; grid_h as usize];
        for sy in 0..grid_h {
            for sx in 0..grid_w {
                let x = sx - 1;
                let y = sy - 1;
                is_bg[sy as usize][sx as usize] = x < 0 || x >= width as i32 || y < 0 || y >= height as i32 || visited[y as usize][x as usize];
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

        let mut adj: HashMap<(i32, i32), Vec<(i32, i32)>> = HashMap::new();
        for &(start, end) in &segments {
            adj.entry(start).or_default().push(end);
        }

        let mut loops = Vec::new();
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

        if verbose {
            let bg_pixel_count = visited.iter().map(|row| row.iter().filter(|&&v| v).count()).sum::<usize>();
            let contour_point_count = loops.iter().map(|l| l.len()).sum::<usize>();
            println!("[VERBOSE] Mask stats (for one figure): Background pixels = {} | Contour/outline vertices = {}", bg_pixel_count, contour_point_count);
        }

        Ok(loops)
    }
}
