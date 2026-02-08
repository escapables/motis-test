<p align="center"><img src="logo.svg" width="196" height="196"></p>

# MOTIS Portable Offline Fork (`escapables/motis-portable`)

This fork targets one primary outcome: a portable offline Linux desktop app that runs MOTIS without browser localhost access.

## Supported Runtime Model

- Primary app: `gui-svelte/` (Tauri + Svelte).
- Transport: `motis://` protocol in Tauri.
- Backend: `motis-ipc` over stdin/stdout JSON to MOTIS core.
- Deployment target: USB-friendly bundle (`usb-bundle-svelte/`), including FAT32-safe launcher (`RUN.sh`).

## Build Prerequisites

### Required tools

- `git`
- `cmake` (3.20+ recommended)
- C/C++ toolchain with C++23 support (`gcc`/`g++` or `clang`)
- `make` or `ninja`
- Rust toolchain (Rust 1.77+), `cargo`
- Tauri CLI (`cargo install tauri-cli`)
- `node` + `pnpm`
- `pkg-config`

### Required Linux system libraries (for Tauri/WebKit build)

- GTK3 development package
- WebKit2GTK development package
- libsoup3 development package
- JavaScriptCoreGTK development package
- OpenSSL development package

Package names vary by distro. Install the equivalent `-dev`/`-devel` packages for your distribution.

## Clean-Clone USB Build (Validated)

Validated in a clean clone on **February 8, 2026**:

```bash
git clone git@github.com:escapables/motis-portable.git
cd motis-portable
./gui-svelte/build-usb.sh
```

What this script does:

1. Builds the Svelte UI.
2. Builds native targets (`motis`, `motis-ipc`) if missing.
3. Builds the Tauri app (`motis-gui-svelte`).
4. Assembles `usb-bundle-svelte/`.
5. Applies runtime permissions.

No manual patching or manual build steps are required when prerequisites are installed.
First-time builds require network access to download Rust crates, pnpm packages, and CMake-managed dependencies.

## Offline / Reuse Modes

```bash
./gui-svelte/build-usb.sh --offline
./gui-svelte/build-usb.sh --skip-pnpm-install
```

`--offline` requires previously populated `node_modules`.

## Run the Bundle

### 1. Import data (once per dataset)

```bash
cd usb-bundle-svelte
./motis-import.sh /path/to/gtfs.zip /path/to/osm.pbf
```

### 2. Start app

```bash
./RUN.sh
```

## Bundle Contents

`usb-bundle-svelte/` contains:

- `motis-gui-svelte`
- `motis-ipc`
- `motis`
- `RUN.sh`
- `motis-import.sh`
- `sweden-route-fix.lua`
- `ui/`
- `data/` (imported by user)

## Repository Layout

- `gui-svelte/` primary desktop app and USB build tooling
- `native/` C++ native API and IPC bridge
- `ui/` Svelte frontend source
- `src/` MOTIS core logic
- `docs/` design and implementation notes

## Current Status

- Native IPC bridge working
- Svelte Tauri app working
- Route planning, geocoding, reverse geocoding working
- Vector tiles and glyph rendering working

## Upstream Project

- https://github.com/motis-project/motis
