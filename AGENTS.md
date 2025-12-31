# Agents Guide (avif_converter_server)

This file summarizes project-specific conventions for AI agents and contributors.

## Quick facts
- Service: Rust Actix Web image conversion/compression server (AVIF-focused).
- Entry point: `src/main.rs` (routes, handlers, processing).
- API routes: `/`, `/up`, `/convert`.
- Core logic: `process_image_bytes` + `ResizeOptions` (crop/fit/bg/pattern/quality).
- External IO: HTTP fetch for source image and optional pattern image.
- Deployment: Docker (`Dockerfile`, `docker-compose.yml`), Fly (`fly.toml`).
- Port: `0.0.0.0:8080`.

## Conventions
- Keep handlers thin; put image logic in helper functions.
- Use `ResizeOptions` as the single config surface; add new query params there and update parsing in one place.
- Return 400 for invalid query params and 500 for processing failures; keep error messages clear and short.
- When adding query parameters or behavior changes, update `README.md`, `docs/changelog`, and `docs/logs`.
- For medium/large changes, write a Change Spec in `docs/specs/` using `docs/specs/TEMPLATE.md` before implementation.
- If `src/main.rs` grows, split into modules under `src/` rather than adding more large functions.
- Avoid removing existing test assets or fixtures unless necessary; tests skip when assets are missing.

## Commands
- Dev: `cargo run`
- Release: `cargo run --release`
- Build: `cargo build --release`
- Tests: `cargo test`
- Integration: `./test_server.sh`
- Docker: `docker compose up --build`

## Where to look first
- Request handlers and processing: `src/main.rs`
- API docs: `README.md`
- Integration flow: `test_server.sh`
- Change history: `docs/changelog`, `docs/logs`

## Common pitfalls
- EXIF orientation is applied before cropping; bbox values should target the oriented image.
- `bbox` values are clamped to bounds; empty crops are ignored.
- `image::DynamicImage::resize` preserves aspect ratio; use `fit=contain|cover|fill` for exact sizing.
- `ravif` encoding requires separate alpha handling; check `has_alpha` when adding formats.
- Fetching remote URLs has no size limits or timeouts; be cautious with untrusted inputs.

## Manual update guide
- Update Quick facts when routes, entry points, or deployment targets change.
- Update `SKILLS.md` when new modules or integrations are added.
- Update `SPEC.md` when API behavior or workflows change.
- Keep `README.md`, `docs/changelog`, and `docs/logs` aligned with user-visible changes.
