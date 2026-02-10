# Portable App Reference

This document is the repository-owned source of truth for the portable desktop runtime.

Use this as the long-term technical context for the fork, especially if `AGENTS.md` is moved out of the repository.

## Scope

- Linux-only portable desktop runtime.
- Primary app: `motis-gui-svelte` (`gui-svelte/`).
- Transport for native runtime: `motis://` custom protocol.
- Backend transport: `motis-ipc` over stdin/stdout JSON.
- Deployment target: `usb-bundle/` with FAT32-safe launcher (`RUN.sh`).

## Runtime Contract

- Native app runtime is IPC-only.
- No localhost HTTP fallback in native runtime.
- Browser/server localhost workflows are development-only and outside native runtime guarantees.

If this contract changes, update this file first.

## Architecture (Runtime Path)

1. Svelte UI executes `fetch()` against `motis://...` URLs.
2. Tauri protocol handler in `gui-svelte/src-tauri/src/protocol.rs` classifies path and routes request.
3. Native bridge in `gui-svelte/src-tauri/src/native.rs` sends JSON commands to `motis-ipc`.
4. `motis-ipc` (`native/example_ipc.cc`) calls native MOTIS API (`native/api.cc` / MOTIS core in `src/`).
5. Response returns through protocol handler to the UI.

## Key Files

- `gui-svelte/src-tauri/src/protocol.rs`: request routing for `motis://`.
- `gui-svelte/src-tauri/src/native.rs`: IPC process lifecycle, request/response bridge, startup diagnostics.
- `native/example_ipc.cc`: IPC command dispatcher.
- `native/api.cc`: C++ native API wrapper.
- `gui-svelte/build-usb.sh`: build + bundle assembly.
- `usb-bundle/RUN.sh`: production launcher for USB/FAT32 constraints.

## Supported Protocol Surface

The native protocol handler currently supports:

- Core API passthrough:
  - `/api/v1/geocode`, `/api/v5/geocode`
  - `/api/v1/reverse-geocode`, `/api/v5/reverse-geocode`
  - `/api/v1/plan`, `/api/v5/plan`
  - `/api/v1/trip`, `/api/v5/trip`
  - `/api/v1/stoptimes`, `/api/v4/stoptimes`, `/api/v5/stoptimes`
  - `/api/v1/map/trips`, `/api/v4/map/trips`, `/api/v5/map/trips`
  - `/api/v1/map/initial`, `/api/v1/map/stops`, `/api/v1/map/levels`
  - `/api/v1/one-to-many`, `/api/v1/one-to-all`, `/api/experimental/one-to-all`
  - `/api/v1/rentals`, `/api/v1/map/rentals`
- Tiles:
  - `/api/v1/tiles/...`, `/api/v5/tiles/...`, `/tiles/*.mvt`
- Glyphs:
  - `/tiles/glyphs/...`
- Debug:
  - `/api/debug/transfers`

Unsupported debug routes return explicit "unsupported protocol endpoint" errors.

For exact current behavior, always verify `classify_path()` in `gui-svelte/src-tauri/src/protocol.rs`.

## USB/FAT32 Launcher Behavior

`usb-bundle/RUN.sh` is the recommended entrypoint.

What it does:

1. Validates bundle layout and `data/config.yml`.
2. Chooses an executable temp directory (`$XDG_RUNTIME_DIR` or `/tmp`) and verifies mount executability.
3. Copies `motis-gui-svelte` and `motis-ipc` into temp dir, sets executable bits.
4. Exports:
   - `MOTIS_DATA_PATH`
   - `MOTIS_IPC_PATH`
5. Launches GUI with `--data-path`.
6. Cleans temp artifacts on exit (unless `--launcher-keep-tmp` is set).

Useful launcher flags:

- `--launcher-self-test`
- `--launcher-keep-tmp`

## Build and Bundle

Reference workflow:

```bash
./gui-svelte/build-usb.sh
```

Script behavior (`gui-svelte/build-usb.sh`):

1. Builds UI (`ui/`) with pnpm.
2. Ensures native binaries (`motis`, `motis-ipc`).
3. Builds Tauri app via `cargo tauri build`.
4. Assembles `usb-bundle/` from template + binaries + UI assets.
5. Sets executable permissions.

Supported reuse modes:

```bash
./gui-svelte/build-usb.sh --offline
./gui-svelte/build-usb.sh --skip-pnpm-install
```

## Data and Startup Expectations

Expected runtime files in `usb-bundle/`:

- `motis-gui-svelte`
- `motis-ipc`
- `motis`
- `RUN.sh`
- `motis-import.sh`
- `data/` (contains `config.yml` after import)

Import command:

```bash
cd usb-bundle
./motis-import.sh /path/to/gtfs.zip /path/to/osm.pbf
```

## Troubleshooting

- "Permission denied" on USB/FAT32: use `./RUN.sh`.
- Missing data/config errors: ensure `usb-bundle/data/config.yml` exists (run import first).
- IPC initialization failures: verify `MOTIS_IPC_PATH` and `MOTIS_DATA_PATH` values in launcher logs.
- Endpoint not found/unsupported: confirm path is covered by `classify_path()`.

## Change Playbooks

### Add a new protocol endpoint

1. Add path mapping in `classify_path()` in `gui-svelte/src-tauri/src/protocol.rs`.
2. Route to existing handler or add a handler.
3. If passthrough, ensure query bytes are preserved (do not normalize semantics-sensitive IDs).
4. Add/adjust unit tests in `protocol.rs` test module.

### Add a new IPC command

1. Add command handling in `native/example_ipc.cc`.
2. Add native bridge function in `gui-svelte/src-tauri/src/native.rs`.
3. Wire through protocol handler if exposed via `motis://`.
4. Validate via app flow launched through `RUN.sh`.

## Guardrails

- Do not reintroduce localhost/server fallback into native Svelte runtime.
- Do not regress offline/firewall-restricted operation.
- Keep this file synchronized with actual shipped behavior and scripts.

## Freshness Checklist

Update this file whenever any of the following changes:

- `classify_path()` route coverage in `protocol.rs`
- startup/auto-init contract in `native.rs`
- launcher semantics in `usb-bundle/RUN.sh`
- build/bundle process in `gui-svelte/build-usb.sh`
- runtime contract (IPC-only vs fallback behavior)
