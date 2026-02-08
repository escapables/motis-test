<p align="center"><img src="logo.svg" width="196" height="196"></p>

# MOTIS Portable Offline Fork (`escapables/motis-portable`)

> [!WARNING]
> Experimental fork. APIs and behavior can change quickly.

This fork focuses on running MOTIS as a **portable, offline Linux desktop app** with an **IPC-first architecture**:

- No browser required.
- No localhost required for primary operation.
- Works from USB storage (including FAT32 via `/tmp` copy launcher).
- Uses the Svelte UI in a Tauri app (`gui-svelte/`) with a `motis://` custom protocol.
- Uses IPC-only backend mode in the native Svelte app.

## Primary Goal

Run MOTIS from a USB stick on Linux in restricted environments where loopback/network access may be blocked, while preserving the web UI experience.

## Current Status

- Native IPC bridge (`motis-ipc`) working.
- Svelte Tauri app (`motis-gui-svelte`) working.
- Vector tiles rendering in MapLibre working.
- Glyph rendering for labels working.
- Major interactive endpoints routed through IPC protocol passthrough.

## Architecture (IPC-first)

```text
Svelte UI (fetch motis://...)
        |
        v
Tauri protocol handler (Rust)
        |
        v
native.rs sync wrappers
        |
        v
motis-ipc (C++ JSON over stdin/stdout)
        |
        v
MOTIS core + GTFS/OSM data
```

HTTP/localhost is available only for legacy browser/server workflows.

Native Svelte runtime is IPC-only; localhost/browser workflows are legacy development paths.

## Quick Start (Portable Bundle Workflow)

### 1. Build core + IPC

```bash
mkdir -p build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --target motis motis-native motis-ipc -j"$(nproc)"
```

### 2. Build Svelte Tauri app

```bash
cd ../ui
pnpm install
pnpm build

cd ../gui-svelte/src-tauri
cargo tauri build
```

### 3. Assemble/copy to USB root

Minimum runtime files at USB root:

- `motis-gui-svelte`
- `motis-ipc`
- `RUN.sh`
- `data/` (your imported MOTIS dataset)

### 4. Run

```bash
./RUN.sh
```

## Localhost Workflows

Localhost/browser workflows are treated as legacy development tooling and are not part of the native portable runtime contract.

## Repository Guide

- `native/` C++ native API + IPC bridge.
- `gui-svelte/` primary desktop app (Tauri + Svelte UI).
- `gui/` simple HTML Tauri app (secondary/debug UI path).
- `ui/` Svelte web UI source.
- `docs/` setup guides and project-specific design/roadmap docs.

## Project Docs

- `docs/PORTABLE_APP.md` architecture, deployment model, implementation notes.
- `docs/ROADMAP.md` focused backlog and milestones.
- `docs/decisions/localhost-mode-deprecation.md` decision memo and migration checklist for issue `#19`.
- `gui-svelte/README.md` Svelte bundle build and run details.
- `gui/README.md` simple GUI notes.
- Upstream dev setup docs remain under `docs/linux-dev-setup.md`, `docs/windows-dev-setup.md`, `docs/macos-dev-setup.md`.

## Upstream Project

Original MOTIS project:

- https://github.com/motis-project/motis
