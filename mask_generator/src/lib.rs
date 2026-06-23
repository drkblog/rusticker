use image::{DynamicImage, GenericImageView};
use std::collections::{HashMap, VecDeque};

/// Trait representing an algorithm for tracing the contour mask of an image.
pub trait MaskAlgorithm {
    /// Traces the boundary outline loops of the mask from the input image.
    /// Returns a list of loops, where each loop is a list of coordinates ((x, y), is_bezier).
    fn trace_mask(
        &self,
        img: &DynamicImage,
        verbose: bool,
        is_unsafe: bool,
    ) -> Result<Vec<Vec<((f64, f64), bool)>>, Box<dyn std::error::Error>>;
}

/// A mask tracing implementation using a flood-fill algorithm to detect
/// background pixels and contour tracing to extract boundary loops.
pub struct BasicTracer;

impl BasicTracer {
    pub fn trace_raw_mask(
        img: &DynamicImage,
        verbose: bool,
    ) -> Result<Vec<Vec<((f64, f64), bool)>>, Box<dyn std::error::Error>> {
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
            let &start_pt = adj.keys().min().unwrap();
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

        let mapped_loops = loops
            .into_iter()
            .map(|lp| lp.into_iter().map(|p| ((p.0 as f64, p.1 as f64), false)).collect())
            .collect();
        Ok(mapped_loops)
    }
}

impl MaskAlgorithm for BasicTracer {
    fn trace_mask(
        &self,
        img: &DynamicImage,
        verbose: bool,
        is_unsafe: bool,
    ) -> Result<Vec<Vec<((f64, f64), bool)>>, Box<dyn std::error::Error>> {
        let raw_loops = BasicTracer::trace_raw_mask(img, verbose)?;

        let loops_count = raw_loops.len();
        let total_vertices: usize = raw_loops.iter().map(|lp| lp.len()).sum();

        if !is_unsafe && (total_vertices > 50000 || loops_count > 50) {
            return Err(format!(
                "Image is too complex: outline contains {} vertices and {} loops. The maximum supported limits are 50000 vertices and 50 loops.",
                total_vertices, loops_count
            )
            .into());
        }

        Ok(raw_loops)
    }
}
fn perpendicular_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0.0 {
        let px = p.0 - a.0;
        let py = p.1 - a.1;
        (px * px + py * py).sqrt()
    } else {
        let num = (dx * (p.1 - a.1) - dy * (p.0 - a.0)).abs();
        num / len_sq.sqrt()
    }
}

fn rdp_recursive(points: &[(f64, f64)], epsilon: f64, start: usize, end: usize, keep: &mut [bool]) {
    if end <= start + 1 {
        return;
    }

    let p_start = points[start];
    let p_end = points[end];

    let mut d_max = 0.0;
    let mut index = start;

    for i in (start + 1)..end {
        let p = points[i];
        let dist = perpendicular_distance(p, p_start, p_end);
        if dist > d_max {
            d_max = dist;
            index = i;
        }
    }

    if d_max > epsilon {
        keep[index] = true;
        rdp_recursive(points, epsilon, start, index, keep);
        rdp_recursive(points, epsilon, index, end, keep);
    }
}

pub fn simplify_rdp(points: &[(f64, f64)], epsilon: f64) -> Vec<(f64, f64)> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;

    rdp_recursive(points, epsilon, 0, points.len() - 1, &mut keep);

    let mut simplified = Vec::new();
    for i in 0..points.len() {
        if keep[i] {
            simplified.push(points[i]);
        }
    }
    simplified
}

/// An advanced mask tracing implementation that simplifies the traced outline.
pub struct AdvancedTracer {
    pub rdp_level: u8,
}

impl MaskAlgorithm for AdvancedTracer {
    fn trace_mask(
        &self,
        img: &DynamicImage,
        verbose: bool,
        is_unsafe: bool,
    ) -> Result<Vec<Vec<((f64, f64), bool)>>, Box<dyn std::error::Error>> {
        // Use BasicTracer to get raw mask outline
        let raw_loops = BasicTracer::trace_raw_mask(img, false)?;

        let loops_count = raw_loops.len();
        let total_vertices: usize = raw_loops.iter().map(|lp| lp.len()).sum();

        if !is_unsafe && (total_vertices > 100000 || loops_count > 60) {
            return Err(format!(
                "Image is too complex: outline contains {} vertices and {} loops. The maximum supported limits are 100000 vertices and 60 loops.",
                total_vertices, loops_count
            )
            .into());
        }

        if verbose {
            println!(
                "[VERBOSE] Mask stats before simplification: Loops = {} | Contour/outline vertices = {}",
                loops_count, total_vertices
            );
        }

        let mut simplified_loops = Vec::new();
        // Map rdp_level to epsilon
        let epsilon = match self.rdp_level {
            1 => 0.5,
            2 => 1.0,
            3 => 1.5,
            4 => 2.0,
            5 => 3.0,
            _ => 1.5,
        };
        for lp in raw_loops {
            // Extract coordinates
            let pts: Vec<(f64, f64)> = lp.iter().map(|&(p, _)| p).collect();
            // Apply RDP simplification with mapped epsilon
            let simplified_pts = simplify_rdp(&pts, epsilon);
            if simplified_pts.len() >= 4 {
                // Reconstruct with bezier = false
                let simplified_lp: Vec<((f64, f64), bool)> = simplified_pts
                    .into_iter()
                    .map(|p| (p, false))
                    .collect();
                simplified_loops.push(simplified_lp);
            }
        }

        let simplified_vertices: usize = simplified_loops.iter().map(|lp| lp.len()).sum();

        if verbose {
            let reduction = if total_vertices > 0 {
                (total_vertices - simplified_vertices) as f64 / total_vertices as f64 * 100.0
            } else {
                0.0
            };
            println!(
                "[VERBOSE] Mask stats after simplification: Loops = {} | Contour/outline vertices = {} ({:.1}% reduction)",
                simplified_loops.len(), simplified_vertices, reduction
            );
        }

        Ok(simplified_loops)
    }
}

/// A mask tracing implementation that simplifies the traced outline and smooths it using quadratic B-splines.
pub struct CurvesTracer {
    pub rdp_level: u8,
}

impl MaskAlgorithm for CurvesTracer {
    fn trace_mask(
        &self,
        img: &DynamicImage,
        verbose: bool,
        is_unsafe: bool,
    ) -> Result<Vec<Vec<((f64, f64), bool)>>, Box<dyn std::error::Error>> {
        // Use BasicTracer to get raw mask outline
        let raw_loops = BasicTracer::trace_raw_mask(img, false)?;

        let loops_count = raw_loops.len();
        let total_vertices: usize = raw_loops.iter().map(|lp| lp.len()).sum();

        if !is_unsafe && (total_vertices > 250000 || loops_count > 100) {
            return Err(format!(
                "Image is too complex: outline contains {} vertices and {} loops. The maximum supported limits are 250000 vertices and 100 loops.",
                total_vertices, loops_count
            )
            .into());
        }

        if verbose {
            println!(
                "[VERBOSE] Curves algorithm: Mask stats before simplification: Loops = {} | Contour/outline vertices = {}",
                loops_count, total_vertices
            );
        }

        let mut curve_loops = Vec::new();
        let epsilon = match self.rdp_level {
            1 => 0.5,
            2 => 1.0,
            3 => 1.5,
            4 => 2.0,
            5 => 3.0,
            _ => 1.5,
        };

        for lp in raw_loops {
            // Extract coordinates
            let pts: Vec<(f64, f64)> = lp.iter().map(|&(p, _)| p).collect();
            // Simplify using RDP
            let mut simplified = simplify_rdp(&pts, epsilon);
            
            if simplified.len() >= 4 {
                // Remove duplicate start/end point to get unique vertices
                if simplified.first() == simplified.last() {
                    simplified.pop();
                }

                let m = simplified.len();
                if m >= 3 {
                    let mut b_spline_points = Vec::new();

                    for i in 0..m {
                        let prev = simplified[(i + m - 1) % m];
                        let curr = simplified[i];
                        let next = simplified[(i + 1) % m];

                        // Calculate midpoints of adjacent edges
                        let m_in = ((prev.0 + curr.0) / 2.0, (prev.1 + curr.1) / 2.0);
                        let m_out = ((curr.0 + next.0) / 2.0, (curr.1 + next.1) / 2.0);

                        // Convert quadratic Bezier curve to cubic Bezier curve:
                        // Q1 = 1/3 m_in + 2/3 curr
                        // Q2 = 1/3 m_out + 2/3 curr
                        let q1 = (m_in.0 / 3.0 + 2.0 * curr.0 / 3.0, m_in.1 / 3.0 + 2.0 * curr.1 / 3.0);
                        let q2 = (m_out.0 / 3.0 + 2.0 * curr.0 / 3.0, m_out.1 / 3.0 + 2.0 * curr.1 / 3.0);

                        // If first corner, push the start point
                        if i == 0 {
                            b_spline_points.push((m_in, false));
                        }

                        // Push first control point
                        b_spline_points.push((q1, true));

                        // Push second control point
                        b_spline_points.push((q2, true));

                        // Push end endpoint of this segment
                        b_spline_points.push((m_out, false));
                    }

                    curve_loops.push(b_spline_points);
                }
            }
        }

        let curve_vertices: usize = curve_loops.iter().map(|lp| lp.len()).sum();

        if verbose {
            let reduction = if total_vertices > 0 {
                (total_vertices - curve_vertices) as f64 / total_vertices as f64 * 100.0
            } else {
                0.0
            };
            println!(
                "[VERBOSE] Curves algorithm: Mask stats after B-spline smoothing: Loops = {} | Contour/outline vertices = {} ({:.1}% reduction/change)",
                curve_loops.len(), curve_vertices, reduction
            );
        }

        Ok(curve_loops)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn test_rdp_simplification() {
        // A stair-step diagonal line from (0,0) to (4,4)
        let stair_steps = vec![
            (0.0, 0.0),
            (1.0, 0.0),
            (1.0, 1.0),
            (2.0, 1.0),
            (2.0, 2.0),
            (3.0, 2.0),
            (3.0, 3.0),
            (4.0, 3.0),
            (4.0, 4.0),
        ];
        // With epsilon = 1.5, it should simplify to a straight diagonal line
        let simplified = simplify_rdp(&stair_steps, 1.5);
        assert_eq!(simplified, vec![(0.0, 0.0), (4.0, 4.0)]);
    }

    #[test]
    fn test_advanced_tracer_simplifies() {
        let mut img = RgbaImage::new(20, 20);
        let white = Rgba([255, 255, 255, 255]);
        let black = Rgba([0, 0, 0, 255]);

        // Draw a 10x10 square
        for y in 0..20 {
            for x in 0..20 {
                if x >= 5 && x < 15 && y >= 5 && y < 15 {
                    img.put_pixel(x, y, black);
                } else {
                    img.put_pixel(x, y, white);
                }
            }
        }

        let dyn_img = DynamicImage::ImageRgba8(img);
        let basic = BasicTracer.trace_mask(&dyn_img, false, false).unwrap();
        let advanced = AdvancedTracer { rdp_level: 3 }.trace_mask(&dyn_img, false, false).unwrap();

        let basic_vertices: usize = basic.iter().map(|l| l.len()).sum();
        let advanced_vertices: usize = advanced.iter().map(|l| l.len()).sum();

        // Advanced should have fewer vertices due to simplification
        assert!(advanced_vertices < basic_vertices);
        assert!(advanced_vertices > 0);
    }

    #[test]
    fn test_advanced_tracer_complexity_limit() {
        let mut img = RgbaImage::new(250, 250);
        let black = Rgba([0, 0, 0, 255]);

        // Draw 70 disjoint checkerboard squares (each will create a loop)
        for i in 0..70 {
            let start_x = (i % 10) * 20 + 5;
            let start_y = (i / 10) * 20 + 5;
            for y in start_y..(start_y + 10) {
                for x in start_x..(start_x + 10) {
                    img.put_pixel(x, y, black);
                }
            }
        }

        let dyn_img = DynamicImage::ImageRgba8(img);
        let result = AdvancedTracer { rdp_level: 3 }.trace_mask(&dyn_img, false, false);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("too complex") || err_msg.contains("loops"));

        // With is_unsafe = true, it should succeed bypassing limits
        let result_unsafe = AdvancedTracer { rdp_level: 3 }.trace_mask(&dyn_img, false, true);
        assert!(result_unsafe.is_ok());
    }

    #[test]
    fn test_advanced_tracer_rdp_levels() {
        let mut img = RgbaImage::new(40, 40);
        let white = Rgba([255, 255, 255, 255]);
        let black = Rgba([0, 0, 0, 255]);

        // Create a jagged/pixelated shape (a stair-step circle/diamond)
        for y in 0..40 {
            for x in 0..40 {
                let dx = (x as i32 - 20).abs();
                let dy = (y as i32 - 20).abs();
                if dx + dy < 15 {
                    img.put_pixel(x, y, black);
                } else {
                    img.put_pixel(x, y, white);
                }
            }
        }

        let dyn_img = DynamicImage::ImageRgba8(img);
        let level1 = AdvancedTracer { rdp_level: 1 }.trace_mask(&dyn_img, false, false).unwrap();
        let level5 = AdvancedTracer { rdp_level: 5 }.trace_mask(&dyn_img, false, false).unwrap();

        let l1_vertices: usize = level1.iter().map(|l| l.len()).sum();
        let l5_vertices: usize = level5.iter().map(|l| l.len()).sum();

        // Higher rdp_level should optimize more aggressively, resulting in fewer vertices
        assert!(l5_vertices < l1_vertices);
    }

    #[test]
    fn test_curves_tracer() {
        let mut img = RgbaImage::new(20, 20);
        let white = Rgba([255, 255, 255, 255]);
        let black = Rgba([0, 0, 0, 255]);

        // Draw a 10x10 square
        for y in 0..20 {
            for x in 0..20 {
                if x >= 5 && x < 15 && y >= 5 && y < 15 {
                    img.put_pixel(x, y, black);
                } else {
                    img.put_pixel(x, y, white);
                }
            }
        }

        let dyn_img = DynamicImage::ImageRgba8(img);
        let curves = CurvesTracer { rdp_level: 3 }.trace_mask(&dyn_img, false, false).unwrap();

        assert!(!curves.is_empty());
        let loop0 = &curves[0];
        assert_eq!(loop0.len(), 13); // 3 * 4 unique vertices + 1 start point = 13

        // Check starting and ending points are equal
        assert_eq!(loop0.first().unwrap().0, loop0.last().unwrap().0);

        // Check alternating cubic bezier flags: false, true, true, false, true, true, false...
        assert_eq!(loop0.first().unwrap().1, false);
        assert_eq!(loop0.last().unwrap().1, false);

        let bezier_flags: Vec<bool> = loop0.iter().map(|&(_, b)| b).collect();
        assert_eq!(bezier_flags[0], false);
        assert_eq!(bezier_flags[1], true);
        assert_eq!(bezier_flags[2], true);
        assert_eq!(bezier_flags[3], false);
        assert_eq!(bezier_flags[4], true);
        assert_eq!(bezier_flags[5], true);
        assert_eq!(bezier_flags[6], false);
    }
}




