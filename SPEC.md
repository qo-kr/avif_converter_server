# AVIF Converter Server Spec (project snapshot)

This is a high-level spec derived from code and docs. Update it when behavior changes.

## Purpose
Provide a stateless image compression and resizing service for internal use, producing AVIF output with optional cropping, fit modes, background padding, and quality control.

## System overview
- Actix Web server with three routes.
- `/convert` fetches a remote image (and optional pattern), applies EXIF orientation, optional crop, resize/fit, optional background fill, encodes to AVIF, and returns bytes.
- No persistent storage or database; single binary deployment via Docker/Fly.

## Key components
- Entry point: `src/main.rs`
- Core pipeline: `process_image_bytes`, `ResizeOptions`, `ResizeFit`, `CropUnit`
- Helpers: `parse_bbox_values`, `clamp_crop_rect`, `create_background`
- Integration validation: `test_server.sh`

## External services
- Remote image source URLs (HTTP/HTTPS).
- Optional pattern image URL for contain padding.

## API surface
- `GET /` - simple greeting
- `GET /up` - health check
- `GET /convert` - convert/resize
  - `url` (required)
  - `width`, `height` (optional)
  - `bbox` or `bbox_*` (optional)
  - `bbox_unit` (`px` default, `norm` for 0..1)
  - `fit` (`contain`, `cover`, `fill`)
  - `bg` (named color or hex)
  - `pattern` (pattern image URL)
  - `ext` (`avif`, `jpg`, `jpeg`, `png`, default `avif`; overrides `Accept`)
  - `quality` (1-100, default 80; PNG maps to compression hints)
  - `Accept` header (exact `image/avif`, `image/jpeg`, `image/png` when `ext` is omitted)

## Core workflows
- Convert: fetch -> decode -> EXIF orientation -> crop -> resize/fit -> background/pattern -> encode (AVIF/JPEG/PNG) -> respond.
- Crop: clamp to bounds; normalized values are relative to oriented image size.
- Contain: pad to target size with color or tiled pattern.
- JPEG output: if transparency remains and `bg` is missing, fall back to PNG to preserve alpha.

## Data model (conceptual)
Stateless; no DB. All inputs are query params and remote image bytes; outputs are AVIF/JPEG/PNG bytes.

## Non-functional requirements
- Performance: Lanczos3 resize; ravif speed 6; no caching.
- Reliability: returns 400 for invalid params, 500 for download/processing failures.
- Security: outbound HTTP fetch only; inputs should be validated if exposed publicly.

## Build and run
- Install/build: `cargo build --release`
- Run: `cargo run --release`
- Docker: `docker compose up --build`
- Tests: `cargo test`, `./test_server.sh`

## Documentation
- API usage: `README.md`
- Change history: `docs/changelog`, `docs/logs`

## Change Spec format
- Use `docs/specs/TEMPLATE.md` for medium/large changes (new endpoints, new processing stages, new dependencies).

## Manual update guide
- Update "API surface" when endpoints or parameters change.
- Update "Core workflows" when processing logic changes.
- Update "External services" when new dependencies or integrations are added.
