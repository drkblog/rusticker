# Rusticker

`rusticker` is a command-line tool written in Rust for generating A4 grid layouts of stickers/shapes as vector outlines or composed with images in a PDF format. It enables precise control over grid dimensions, DPI resolutions, and spacing.

## Features

- **High-Precision Layouts**: Generates A4 PDF pages with customized grid alignments.
- **Support for Shapes**: Supports drawing squares or circles as base figures.
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
- `-h, --help`: Prints help information.
- `-V, --version`: Prints version information.

### Subcommands

#### `bake`

Generates blank shapes (square or circle outlines) in an A4 grid.

```bash
rusticker bake [OPTIONS] --figure <FIGURE> --size <SIZE>
```

- `--figure <FIGURE>`: Type of figure to bake (`square` or `circle`).
- `--size <SIZE>`: Size of the figure in pixels (side length for square, diameter for circle).
- `--min-space <MIN_SPACE>`: Minimum spacing in millimeters between adjacent figures [default: `2.0`].
- `--stroke-thickness <STROKE_THICKNESS>`: Stroke thickness of the figure outline in millimeters (e.g. `2.25`) [default: `1.0`].
- `-o, --output <OUTPUT>`: Output PDF file path [default: `baked.pdf`].

#### `compose`

Generates grid shapes populated with a center-cropped repeat of an input image.

```bash
rusticker compose [OPTIONS] --figure <FIGURE> --input <INPUT> --size <SIZE>
```

- `--figure <FIGURE>`: Type of figure (`square` or `circle`).
- `--input <INPUT>`: Path to the input image file (PNG or JPEG).
- `--size <SIZE>`: Size of the figure in pixels.
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
