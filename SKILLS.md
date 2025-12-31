# Skills Map (avif_converter_server)

This file maps domain areas to code locations and entry points.

## HTTP server and routing
- Routes and handlers: `src/main.rs` (`index`, `health_check`, `convert_and_resize_image`)

## Query parsing and validation
- `ResizeFit`, `CropUnit`, `parse_bbox_values` in `src/main.rs`

## Image processing pipeline
- `process_image_bytes`, `ResizeOptions`, `clamp_crop_rect`, `create_background` in `src/main.rs`
- EXIF orientation handling in `process_image_bytes`

## External IO
- HTTP fetch via `reqwest::Client` in `convert_and_resize_image`

## Tests and validation
- Unit tests in `src/main.rs`
- Integration script `test_server.sh`
- Sample images in project root (used by tests/scripts)

## Deployment and ops
- Container build: `Dockerfile`
- Local container run: `docker-compose.yml`
- Fly config: `fly.toml`

## Manual update guide
- Update this file when new modules, endpoints, or external systems are added.
- Prefer pointing to directories once code is split into modules.
