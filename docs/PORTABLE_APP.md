# Portable App Architecture

This document is the fork-specific technical reference for the portable offline app.

## Objectives

- Primary runtime path must not depend on localhost.
- Run from USB on Linux (including FAT32 media).
- Preserve Svelte UI behavior from the web app.
- Keep native runtime IPC-only.

## Runtime Mode

1. `IPC mode` (native app)
- `motis-gui-svelte` serves UI and intercepts API requests via `motis://` protocol.
- Rust protocol handler calls synchronous IPC wrappers in `native.rs`.
- IPC wrappers send JSON commands to `motis-ipc` via stdin/stdout.
- `motis-ipc` calls native MOTIS C++ APIs directly.

Legacy localhost/browser workflows remain available as separate development tooling, not as part of the native app fallback path.

## Main Components

- `gui-svelte/src-tauri/src/protocol.rs`
- `gui-svelte/src-tauri/src/native.rs`
- `native/example_ipc.cc`
- `native/api.cc`
- `ui/src/...` (Svelte frontend)

## Data + USB Model

Expected runtime files in USB root:

- `motis-gui-svelte`
- `motis-ipc`
- `RUN.sh`
- `data/`

`RUN.sh` handles FAT32 execution limitations by copying binaries to `/tmp`, setting execute bits, and running there while keeping data on USB.

## API Routing Status

Implemented through protocol passthrough/native API:

- `plan`
- `geocode`
- `reverse-geocode`
- `trip`
- `stoptimes` (`v1/v4/v5`)
- `map/trips` (`v1/v4/v5`)
- `map/initial`
- `map/stops`
- `map/levels`
- `one-to-many`
- `one-to-all`
- `rentals`
- vector `tiles`
- `glyphs`

## Notable Fixes Landed

- Tile rendering fixed by zlib inflate handling before returning vector tile payload to MapLibre.
- Label glyph rendering fixed by wiring `/tiles/glyphs/...` to embedded font resources via IPC.
- Build/deploy flow standardized around `cargo tauri build` for portable assets.

## Build Notes

- Use `cargo tauri build` for Tauri apps (not plain `cargo build --release`) when distributing portable binaries.
- `gui-svelte/build-usb.sh` is the reference script for Svelte bundle assembly.

## Validation Checklist

- App launches via `./RUN.sh` from USB root.
- Data path resolves and initialization succeeds.
- Map tiles render.
- Labels render (no white square placeholders).
- Route planning + core map interactions work without localhost.
