# Changelog - 2025-12-18

## Added

- **New Query Parameters**:
    - `fit`: Controls resize mode (`contain`, `cover`, `fill`).
    - `bg`: Sets background color for `contain` mode (supports named colors and hex codes).
    - `pattern`: Sets a background pattern image URL for `contain` mode.
    - `quality`: Sets AVIF encoding quality (default: 80).
- **Functionality**:
    - Image padding support: Images can now be resized to a fixed dimension with a background color or pattern filling the empty space.
    - Background tiling: Pattern images are automatically tiled to fill the background.
    - Hex color support: Users can specify background colors using hex codes (e.g., `ff0000`).

## Changed

- **Refactoring**:
    - `process_image_bytes` now accepts `ResizeOptions` struct instead of individual `width` and `height` arguments.
    - Enhanced error handling and logic flow within the image processing pipeline.
- **Documentation**:
    - Updated `README.md` with comprehensive examples and parameter descriptions.

## Fixed

- Addressed potential unused import warnings in `src/main.rs`.
