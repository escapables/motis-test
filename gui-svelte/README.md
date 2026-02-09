# MOTIS Transit - Svelte GUI (Tauri)

Primary desktop app target for this portable offline fork.

## Runtime Model

- Frontend requests use `motis://`.
- Rust protocol layer routes requests to native IPC calls.
- `motis-ipc` communicates with MOTIS core using JSON over stdin/stdout.
- Native app runtime is IPC-first; localhost is not required.

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

### Required Linux system libraries

- GTK3 development package
- WebKit2GTK development package
- libsoup3 development package
- JavaScriptCoreGTK development package
- OpenSSL development package

Package names differ by distro. Install equivalent `-dev`/`-devel` packages.

## Recommended Build (Full USB Bundle)

Validated in a clean clone on **February 8, 2026**:

```bash
cd gui-svelte
./build-usb.sh
```

What this does:

1. Builds the Svelte UI.
2. Builds native targets (`motis`, `motis-ipc`) if missing.
3. Builds `motis-gui-svelte` with Tauri.
4. Assembles `usb-bundle/`.

No manual patching or manual intervention is required when prerequisites are installed.
First-time builds require network access for Rust crates, pnpm packages, and CMake-managed dependency downloads.

### Offline/reuse flags

```bash
./build-usb.sh --offline
./build-usb.sh --skip-pnpm-install
```

`--offline` requires existing `node_modules`.

## Manual Build (Developer Path)

```bash
cd ui
pnpm install
pnpm build

cd ../gui-svelte/src-tauri
cargo tauri build
```

## Run

From a prepared bundle with imported `data/`:

```bash
./RUN.sh
```

Direct launch (when executable permissions are available):

```bash
./motis-gui-svelte --data-path ./data
```

## Key Files

- `src-tauri/src/main.rs`
- `src-tauri/src/native.rs`
- `src-tauri/src/protocol.rs`
- `src-tauri/tauri.conf.json`
- `build-usb.sh`
