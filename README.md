# Rusticker

`rusticker` is a command-line tool written in Rust for generating A4 grid layouts of stickers in PDF format, offering precise control over grid dimensions, DPI, and spacing. It allows you to create PDF documents with two layers: a raster layer with the image to be printed, and a vector layer with the outline for die-cutting.

## Features

- **High-Precision Layouts**: Generates A4 PDF pages with customized grid alignments.
- **Support for Shapes**: Supports drawing squares or circles as base figures, or dynamically tracing a custom `mask` outline around the foreground.
- **Image Composition**: Repeats and center-crops input images (PNG/JPEG) into shapes within the grid layout.
- **Adjustable Parameters**:
  - Customize DPI resolution (100, 200, 300, 600).
  - Customize figure size, stroke outline thickness, and minimum spacing in millimeters.
- **Background Removal (`stickerize`)**: Erase image background using an AI model (U2-Netp by default) to create transparent PNG stickers.
- **Safety Safeguard**: Prevents accidental overwriting of output files with a global `--force` flag.

## Building and Running

Ensure you have [Rust and Cargo](https://rustup.rs/) installed, then clone the repository and run:

```bash
cargo build --release
```

The compiled binaries will be located at:
- `target/release/rusticker`: Main tool containing the layout commands (`bake`, `compose`, `batch-compose`).
- `target/release/stickerize`: Background removal neural network tool.

### Packaging Scripts

For convenience, cross-platform packaging scripts are provided to build and bundle the executables:

- **macOS**:
  - Run [packager/macos/package.sh](file:///Users/drkbugs/repos/rusticker/packager/macos/package.sh) to compile and generate a universal macOS installer package: `packager/macos/target/rusticker-1.1.6.pkg`.
  - Use the `--native` flag to compile only for your host's native architecture (Apple Silicon or Intel) instead of a universal binary.
- **Windows**:
  - Run the Zsh script [packager/windows/package.sh](file:///Users/drkbugs/repos/rusticker/packager/windows/package.sh) on Unix-like hosts to cross-compile the MSVC target using `cargo-xwin` and generate the ZIP archive: `packager/windows/target/rusticker-v1.1.6-windows-x64.zip`.
  - Run the PowerShell script [packager/windows/package.ps1](file:///Users/drkbugs/repos/rusticker/packager/windows/package.ps1) to compile and package directly on a Windows host.

## Installation via WinGet

Windows users can install `rusticker` and `stickerize` using the [WinGet](https://github.com/microsoft/winget-cli) package manager:

```bash
winget install drkbugs.rusticker
```

### Local Manifest Verification & Testing

If you are developing or want to test the manifests locally before submitting to the WinGet community repository, run:

```bash
# Validate the manifest files structure and contents
winget validate packager/windows/winget/1.1.6

# Test the installation locally using the manifests
winget install --manifest packager/windows/winget/1.1.6
```

## Usage

The project is split into two standalone executables.

---

### 1. `rusticker`

`rusticker` provides commands to generate sticker grid layouts in PDF format.

```bash
rusticker [GLOBAL_OPTIONS] <SUBCOMMAND>
```

### Global Options

- `--dpi <DPI>`: Resolution of the application in DPI (dots per inch). Allowed values: `100`, `200`, `300`, `600` [default: `300`].
- `--margin <MARGIN>`: Page margin on the A4 page in millimeters [default: `5.0`].
- `--force`: Required to overwrite the output PDF file if it already exists.
- `--unsafe`: Disable some guardrails (like vertices and loops limits for the `mask` figure type).
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
- `--stroke-thickness <STROKE_THICKNESS>`: Stroke thickness of the figure outline in millimeters (e.g. `2.25`) [default: `0.25`].
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
- `--stroke-thickness <STROKE_THICKNESS>`: Stroke thickness of the outline in millimeters [default: `0.25`].
- `--algorithm <ALGORITHM>`: Algorithm to use for mask generation (`basic`, `advanced`, or `curves`). Only used when `--figure mask` is selected [default: `advanced`]:
  - `basic`: Traces exact pixel-stepped straight lines around the mask boundary.
  - `advanced`: Simplifies the outline using the Ramer-Douglas-Peucker (RDP) algorithm to drastically reduce vector segments.
  - `curves`: Converts the RDP simplified outline into smooth cubic Bézier curves (quadratic B-splines) for optimal vinyl plotter cutting.
- `--rdp-level <RDP_LEVEL>`: Aggressiveness of RDP segment reduction. Accepts a value from `1` (least reduction, more segments) to `5` (most reduction, fewest segments) [default: `3`].
- `--resize-outline <RESIZE_OUTLINE>`: Resize factor for the contour outline (greater than `0.5` and less than `1.5`). Only applied when `--figure mask` is selected [default: `1.0`].
- `-o, --output <OUTPUT>`: Output PDF file path [default: `composed.pdf`].

#### `batch-compose`

Composes stickers from multiple input images, quantities, and customized settings specified in a CSV file, laying them out dynamically across A4 pages.

```bash
rusticker batch-compose [OPTIONS] --input <INPUT>
```

- `--input <INPUT>`: Path to the input CSV configuration file.
- `-o, --output <OUTPUT>`: Output PDF file path [default: `batch_composed.pdf`].

##### CSV File Format

The CSV configuration file contains one entry per line, using the format:
```csv
<image_path>, <quantity>, <command_line_arguments_for_compose>
```

- `<image_path>`: The path to the image file (PNG/JPEG). Can be optionally enclosed in double quotes if the path contains commas or spaces.
- `<quantity>`: An integer specifying how many stickers to print for this image.
- `<command_line_arguments_for_compose>`: Space-separated command-line arguments corresponding to options in the `compose` subcommand (e.g. `--figure <FIGURE>`, size options like `--diameter`, `--side`, `--width`/`--height`, or outline settings like `--stroke-thickness` or `--algorithm`).

Example CSV content (`stickers.csv`):
```csv
"C:\path with space\future.png", 4, --figure mask
C:\stickers\button.png, 6, --figure circle --diameter 200
C:\stickers\dice.png, 3, --figure square --side 230 --stroke-thickness 0.5
```

##### Validation and Layout

- **Pre-Validation**: `rusticker` validates the CSV first. If any image file is missing, any quantity is invalid, or any command line argument fails parsing, the command aborts immediately before generating any output.
- **Mixed Layouts**: The layout engine dynamically places stickers of different sizes side-by-side using a **Row-by-Row Flow Layout**. When a row is full, it wraps to the next row, and automatically adds new A4 pages as needed.

---

### 2. `stickerize`

`stickerize` is a standalone tool that erases the background of an input image (PNG, JPEG, or WEBP) using a neural network model, saving the transparent output as a PNG.

```bash
stickerize [OPTIONS] --input <INPUT> --output <OUTPUT>
```

- `--input <INPUT>`: Path to the input image file (PNG, JPEG, or WEBP).
- `-o, --output <OUTPUT>`: Output transparent PNG file path.
- `--model <MODEL>`: The neural network model to use for background removal (`u2netp`, `rmbg`, or `birefnet`) [default: `birefnet`].
  - `birefnet`: General-purpose BiRefNet model (~224 MB) [default]. If not locally cached, it downloads automatically from GitHub Releases to `~/.rusticker/models/birefnet.onnx`.
  - `u2netp`: A lightweight, fast pre-trained model (~4.7 MB). If not locally cached, it downloads automatically from GitHub Releases to `~/.rusticker/models/u2netp.onnx`.
  - `rmbg`: Bria AI's high-quality background removal model (~176 MB). If not locally cached, it downloads automatically from Hugging Face to `~/.rusticker/models/rmbg.onnx`.
- `--cuda`: Use CUDA GPU acceleration for inference if specified (CPU execution is used by default).
- `-q, --quiet`: Do not output any logs or progress indicators to stdout (errors will still be printed).
- `-V, --version`: Prints version information, the background removal tool build legend (`Background removal tool build with Rust by drkbugs`), and supported models along with their download URLs.
- `--border <BORDER>`: (Optional) Border thickness in pixels to add around the foreground object after background removal.
- `--border-color <BORDER_COLOR>`: Border color in hexadecimal format (e.g. `#22AA5E` or `22AA5E`, case-insensitive) [default: `#FFFFFF`].
- `--antialiasing <true|false>`: Enable or disable antialiasing for the outer part of the border outline [default: `true`].
- `--format <png|webp>`: Output image format to save background-removed stickers [default: `png`].


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
- More loops than permitted:
  - **`basic`**: 500 loops
  - **`advanced`**: 1,500 loops
  - **`curves`**: 3,000 loops
- More vertices than permitted in the raw outline:
  - **`basic`**: 1,000,000 vertices
  - **`advanced`**: 2,000,000 vertices
  - **`curves`**: 4,000,000 vertices

The tool will abort with an error message detailing the complexity. For noisy images, clean up the background to a solid color before processing.

> [!NOTE]
> You can bypass these complexity limits entirely by using the global option `--unsafe` (e.g. `rusticker --unsafe compose --figure mask ...`).

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

### Compose stickers from multiple images via CSV configuration
```bash
cargo run -- batch-compose --input stickers.csv -o mixed_stickers.pdf
```

### Erase the background of an image to create a transparent sticker
```bash
cargo run --bin stickerize -- --input my_sticker.jpg -o my_sticker_transparent.png
```

### Erase the background using the high-quality Bria RMBG-1.4 model
```bash
cargo run --bin stickerize -- --model rmbg --input my_sticker.jpg -o my_sticker_transparent.png
```

### Erase the background using the BiRefNet model
```bash
cargo run --bin stickerize -- --model birefnet --input my_sticker.jpg -o my_sticker_transparent.png
```

### Erase the background and add a customized border
```bash
cargo run --bin stickerize -- --input my_sticker.jpg -o my_sticker_transparent.png --border 10 --border-color "#22AA5E"
```

### Erase the background and add an antialiased border (enabled by default)
```bash
cargo run --bin stickerize -- --input my_sticker.jpg -o my_sticker_transparent.png --border 10 --border-color "#22AA5E"
```

### Erase the background and add a non-antialiased (sharp) border
```bash
cargo run --bin stickerize -- --input my_sticker.jpg -o my_sticker_transparent.png --border 10 --border-color "#22AA5E" --antialiasing false
```

### Erase the background and save as transparent WebP format
```bash
cargo run --bin stickerize -- --input my_sticker.jpg -o my_sticker_transparent.webp --format webp
```
