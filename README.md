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

Generates blank shapes (square or circle outlines) in an A4 grid.

```bash
rusticker bake [OPTIONS] --figure <FIGURE> --size <SIZE>
```

- `--figure <FIGURE>`: Type of figure to bake (`square` or `circle` - `mask` is not supported for baking).
- `--size <SIZE>`: Size of the figure in pixels (side length for square, diameter for circle).
- `--min-space <MIN_SPACE>`: Minimum spacing in millimeters between adjacent figures [default: `2.0`].
- `--stroke-thickness <STROKE_THICKNESS>`: Stroke thickness of the figure outline in millimeters (e.g. `2.25`) [default: `1.0`].
- `-o, --output <OUTPUT>`: Output PDF file path [default: `baked.pdf`].

#### `compose`

Generates grid shapes populated with a center-cropped repeat of an input image.

```bash
rusticker compose [OPTIONS] --figure <FIGURE> --input <INPUT>
```

- `--figure <FIGURE>`: Type of figure (`square`, `circle`, or `mask`). The `mask` option dynamically detects background pixels (matching the color at `(0, 0)`) and traces a custom outline around the foreground sticker.
- `--input <INPUT>`: Path to the input image file (PNG or JPEG).
- `--size <SIZE>`: (Optional) Size of the figure in pixels. If not provided, no cropping is performed and the largest dimension of the input image is used as the base size.
- `--min-space <MIN_SPACE>`: Minimum spacing in millimeters between adjacent figures [default: `2.0`].
- `--stroke-thickness <STROKE_THICKNESS>`: Stroke thickness of the outline in millimeters [default: `1.0`].
- `-o, --output <OUTPUT>`: Output PDF file path [default: `composed.pdf`].

---

## Examples

### Bake a square grid outline
```bash
cargo run -- bake --figure square --size 150 --stroke-thickness 1.5 -o grid_squares.pdf
```

### Force overwrite an existing composed circle grid using an image
```bash
cargo run -- --force compose --figure circle --input my_sticker.png --size 120 --stroke-thickness 2.0 -o output_composed.pdf
```
