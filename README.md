# Rust Image Conversion and Resizing Service 

This project is a web service that allows you to convert and resize images. It uses the Actix Web framework and the ravif library for AVIF image conversion.

## Installation

Make sure you have Rust and Cargo installed on your system. You can install them from [https://www.rust-lang.org/](https://www.rust-lang.org/).

You also need to have `nasm` installed on your path you can get it here https://www.nasm.us

Clone the repository to your local machine:

```bash
git clone https://github.com/adgsenpai/avif_converter_server
cd avif_converter_server
```

Build the project:

```bash
cargo build --release
```

## Usage

To start the image conversion and resizing service, run the following command:

```bash
cargo run --release
```

By default, the service will bind to `0.0.0.0:8080`. You can change the binding address and port in the `main` function of the `main.rs` file.

## API Endpoints

### Convert and Resize an Image

To convert and resize an image, make a GET request to `/convert` with the following query parameters:

- `url`: The URL of the image to convert.
- `width`: (Optional) The desired width of the image.
- `height`: (Optional) The desired height of the image.
- `fit`: (Optional) The resize mode. Options are `contain`, `cover`, `fill`.
  - `contain`: Preserves aspect ratio and adds padding if necessary to fit the dimensions. (Recommended for fixed sizes like 1280x960)
  - `cover`: Preserves aspect ratio and crops the image to fill the dimensions.
  - `fill`: Stretches the image to fill the dimensions (aspect ratio may be broken).
  - If omitted, default behavior is resizing to fit within dimensions (similar to `contain` but without padding).
- `bg`: (Optional) Background color for `contain` mode padding.
  - Named colors: `white`, `black`, `gray`.
  - Hex code: e.g., `ff0000` (for red).
  - Default is transparent.
- `pattern`: (Optional) URL of an image to use as a background pattern for `contain` mode padding.
- `quality`: (Optional) AVIF compression quality (1-100). Default is 80.

### Examples

**Basic Resize:**
```bash
curl "http://localhost:8080/convert?url=https://example.com/image.jpg&width=300&height=200"
```

**Fixed Size with White Padding (1280x960):**
```bash
curl "http://localhost:8080/convert?url=https://example.com/image.jpg&width=1280&height=960&fit=contain&bg=white"
```

**Fixed Size with Pattern Background:**
```bash
curl "http://localhost:8080/convert?url=https://example.com/image.jpg&width=1280&height=960&fit=contain&pattern=https://example.com/pattern.png"
```

## Dependencies

- [Actix Web](https://actix.rs/): A powerful and efficient web framework for Rust.
- [ravif](https://github.com/kornelski/rav1e): A Rust library for AVIF image encoding.
- [reqwest](https://github.com/seanmonstar/reqwest): An HTTP client for Rust.
- [image](https://github.com/image-rs/image): A crate for decoding and encoding various image formats.
- [rgb](https://github.com/linebender/rgb): A crate for working with RGB colors.
- [anyhow](https://github.com/dtolnay/anyhow): A Rust library for handling errors with ease.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
