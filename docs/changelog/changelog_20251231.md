# Changelog - 2025-12-31

## Added

- Output format selection via `ext` query param (`avif`, `jpg`, `jpeg`, `png`) and exact `Accept` header matching.
- JPEG and PNG encoding support with correct `Content-Type` responses.
- PNG compression mapping driven by the existing `quality` parameter.

## Changed

- Image processing now yields a processed image before format-specific encoding.
- JPEG output falls back to PNG when transparency remains and no `bg` is provided.
