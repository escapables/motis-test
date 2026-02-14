<p align="center"><img src="logo.svg" width="196" height="196"></p>

# (`escapables/motis-portable`)

This fork is a substantial refactor of upstream `motis`, focused on a Linux-only, portable offline desktop runtime (Tauri + IPC, no browser-localhost dependency in native mode).

Current divergence against upstream (`motis-project/motis` at `2c8e946f`, 2026-02-08):

- `95 files changed`
- `11,806 insertions`
- `924 deletions`
- `+10,882 net lines`

This quantifies the code-level migration from upstream server-first workflows toward the portable USB-first Linux application model used in this fork.

## Supported Runtime Model

- Primary app: `gui-svelte/` (Tauri + Svelte).
- Transport: `motis://` protocol in Tauri.
- Backend: `motis-ipc` over stdin/stdout JSON to MOTIS core.
- Deployment target: USB-friendly bundle (`usb-bundle/`), including FAT32-safe launcher (`RUN.sh`).

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

## Dev Gate

Run the local gate before handoff:

```bash
./bin/test-gate
```

Docs-only validation:

```bash
./bin/validate-docs
```

## USB Build

```bash
git clone git@github.com:escapables/motis-portable.git
cd motis-portable
./gui-svelte/build-usb.sh
```

What this script does:

1. Builds the Svelte UI.
2. Builds native targets (`motis`, `motis-ipc`) if missing.
3. Builds the Tauri app (`motis-gui-svelte`).
4. Assembles `usb-bundle/`.
5. Applies runtime permissions.

First-time builds require network access to download Rust crates, pnpm packages, and CMake-managed dependencies.

## Offline / Reuse Modes

```bash
./gui-svelte/build-usb.sh --offline
./gui-svelte/build-usb.sh --skip-pnpm-install
```

The `--offline` parameter is used for installation in an offline environment and requires previously populated `node_modules`.

## Run the Bundle

### 1. Import data (once per dataset)

```bash
cd usb-bundle
./motis-import.sh /path/to/gtfs.zip /path/to/osm.pbf
```

- `gtfs.zip`: Public transit feed in GTFS format.
  It provides agencies, routes, stops, trips, and timetables used for journey planning.
- `*.osm.pbf`: OpenStreetMap extract in PBF format.
  It provides street/path geometry for walking connections, street routing, reverse geocoding, and map data generation. See upstream project for examples and links.

Notes:

- Use files that cover the same geographic area.
- Keep GTFS as a zip file (do not extract it manually for this script).

### 2. Start app

```bash
./RUN.sh
```

## Bundle Contents

`usb-bundle/` contains:

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

## Upstream Project

- https://github.com/motis-project/motis
