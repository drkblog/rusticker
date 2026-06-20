# Rusticker

`rusticker` is a command-line tool written in Rust for generating A4 grid layouts of stickers in PDF format, offering precise control over grid dimensions, DPI, and spacing. It allows you to create PDF documents with two layers: a raster layer with the image to be printed, and a vector layer with the outline for die-cutting.

## Features

- **High-Precision Layouts**: Generates A4 PDF pages with customized grid alignments.
- **Support for Shapes**: Supports drawing squares or circles as base figures, or dynamically tracing a custom `mask` outline around the foreground.
- **Image Composition**: Repeats and center-crops input images (PNG/JPEG) into shapes within the grid layout.
- **Adjustable Parameters**:
  - Customize DPI resolution (100, 200, 300, 600).
  - Customize figure size, stroke outline thickness, and minimum spacing in millimeters.
- **Safety Safeguard**: Prevents accidental overwriting of output files with a global `--force` flag.

## Building and Running

Ensure you have [Rust and Cargo](https://rustup.rs/) installed, then clone the repository and run:

```bash
cargo build --release
```

The compiled binary will be located at `target/release/rusticker`.

## Usage

```bash
rusticker [GLOBAL_OPTIONS] <SUBCOMMAND>
```

### Global Options

- `--dpi <DPI>`: Resolution of the application in DPI (dots per inch). Allowed values: `100`, `200`, `300`, `600` [default: `300`].
- `--force`: Required to overwrite the output PDF file if it already exists.
- `-v, --verbose`: Show verbose logs on stdout describing layout calculations, cropping dimensions, grid slots, and mask statistics.
- `-h, --help`: Prints help information.
- `-V, --version`: Prints version information.

### Subcommands

#### `bake`

Generates blank shapes (square, circle, or rectangle outlines) in an A4 grid.

```bash
rusticker bake [OPTIONS] --figure <FIGURE> [--diameter <DIAMETER> | --side <SIDE> | --width <WIDTH> --height <HEIGHT>]
```

- `--figure <FIGURE>`: Type of figure to bake (`square`, `circle`, or `rectangle` - `mask` is not supported for baking).
- `--diameter <DIAMETER>`: Diameter of the circle in pixels (required for circle).
- `--side <SIDE>`: Side length of the square in pixels (required for square).
- `--width <WIDTH>`: Width of the rectangle in pixels (required for rectangle).
- `--height <HEIGHT>`: Height of the rectangle in pixels (required for rectangle).
- `--min-space <MIN_SPACE>`: Minimum spacing in millimeters between adjacent figures [default: `2.0`].
- `--stroke-thickness <STROKE_THICKNESS>`: Stroke thickness of the figure outline in millimeters (e.g. `2.25`) [default: `1.0`].
- `-o, --output <OUTPUT>`: Output PDF file path [default: `baked.pdf`].

#### `compose`

Generates grid shapes populated with a center-cropped repeat of an input image.

```bash
rusticker compose [OPTIONS] --figure <FIGURE> --input <INPUT>
```

- `--figure <FIGURE>`: Type of figure (`square`, `circle`, `rectangle`, or `mask`). The `mask` option dynamically detects background pixels (matching the color at `(0, 0)`) and traces a custom outline around the foreground sticker.
- `--input <INPUT>`: Path to the input image file (PNG or JPEG).
- `--diameter <DIAMETER>`: (Optional) Diameter of the circle in pixels.
- `--side <SIDE>`: (Optional) Side length of the square in pixels.
- `--width <WIDTH>`: (Optional) Width of the rectangle in pixels.
- `--height <HEIGHT>`: (Optional) Height of the rectangle in pixels.
- `--size <SIZE>`: (Optional) Size of the mask figure in pixels. If not provided, no cropping is performed and the largest dimension of the input image is used as the base size.
- `--min-space <MIN_SPACE>`: Minimum spacing in millimeters between adjacent figures [default: `2.0`].
- `--stroke-thickness <STROKE_THICKNESS>`: Stroke thickness of the outline in millimeters [default: `1.0`].
- `--algorithm <ALGORITHM>`: Algorithm to use for mask generation (`basic`, `advanced`, or `curves`). Only used when `--figure mask` is selected [default: `advanced`]:
  - `basic`: Traces exact pixel-stepped straight lines around the mask boundary.
  - `advanced`: Simplifies the outline using the Ramer-Douglas-Peucker (RDP) algorithm to drastically reduce vector segments.
  - `curves`: Converts the RDP simplified outline into smooth cubic Bézier curves (quadratic B-splines) for optimal vinyl plotter cutting.
- `--rdp-level <RDP_LEVEL>`: Aggressiveness of RDP segment reduction. Accepts a value from `1` (least reduction, more segments) to `5` (most reduction, fewest segments) [default: `3`].
- `-o, --output <OUTPUT>`: Output PDF file path [default: `composed.pdf`].

### Mask Generation Algorithms

When using `--figure mask`, the tool automatically detects background pixels (matching the color at `(0, 0)`) and traces a custom outline around the foreground sticker. You can choose from three different algorithms to trace the outline:

- **`basic`**: Traces exact pixel-stepped straight lines around the mask boundary.
- **`advanced` (Default)**: Simplifies the outline using the Ramer-Douglas-Peucker (RDP) algorithm to drastically reduce vector segments.
- **`curves`**: Smooths the contour by converting the RDP-simplified outline into smooth cubic Bézier curves (quadratic B-splines) for optimal vinyl plotter cutting.

#### Optimization Levels (`--rdp-level`)

Controls the aggressiveness of the RDP segment reduction. It accepts a value from `1` to `5` [default: `3`]:
- **`1`**: Low optimization (more segments left, $\epsilon = 0.5$).
- **`2`**: Moderate-low optimization ($\epsilon = 1.0$).
- **`3`**: Medium optimization ($\epsilon = 1.5$).
- **`4`**: High optimization ($\epsilon = 2.0$).
- **`5`**: Maximum optimization (fewest segments, $\epsilon = 3.0$).

#### Complexity Limits & Safety Safeguards

To prevent hangs or extremely large output files on complex or noisy images, `rusticker` enforces complexity limits on mask generation. If an image generates:
- More than **5,000 vertices** in the raw outline, or
- More than **20 separate loops**

The tool will abort with an error message detailing the complexity. For noisy images, clean up the background to a solid color before processing.

---

## Examples

### Bake a square grid outline
```bash
cargo run -- bake --figure square --side 150 --stroke-thickness 1.5 -o grid_squares.pdf
```

### Bake a rectangle grid outline
```bash
cargo run -- bake --figure rectangle --width 150 --height 100 --stroke-thickness 1.5 -o grid_rects.pdf
```

### Force overwrite an existing composed circle grid using an image
```bash
cargo run -- --force compose --figure circle --input my_sticker.png --diameter 120 --stroke-thickness 2.0 -o output_composed.pdf
```

### Compose a composed rectangle grid using an image
```bash
cargo run -- compose --figure rectangle --input my_sticker.png --width 150 --height 100 --stroke-thickness 1.5 -o grid_rects_composed.pdf
```

### Compose smooth vectorial curves around a mask foreground
```bash
cargo run -- compose --figure mask --algorithm curves --rdp-level 4 --input my_sticker.png -o smooth_curves.pdf
```
