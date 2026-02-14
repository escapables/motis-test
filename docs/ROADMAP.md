---
summary: 'Prioritized roadmap and status tracking for the portable IPC-first app.'
read_when:
  - Planning upcoming work.
  - Reviewing completed vs remaining priorities.
---

# Portable App Roadmap

This is the focused roadmap for the portable IPC-first app.

## Current Status (2026-02-10)

- Issues `#1` through `#20` are closed.
- Native runtime is IPC-only (`motis://` + `motis-ipc`), with no localhost fallback in the desktop app.
- Protocol endpoint coverage and legacy stub cleanup are completed.
- USB/FAT32 launcher hardening and startup diagnostics are completed.
- UI reliability and via-stop regressions from the initial backlog are closed.
- Open GitHub issues in `escapables/motis-portable`: none.

## Remaining Priority A (Release Confidence)

- Complete validation of all remaining interactive controls in Svelte UI.
- Verify latest GitHub Actions run for `master` completes green (all required jobs).
- Run a clean-clone end-to-end build and bundle assembly on Linux.
- Re-validate startup and core runtime flows after clean build (import data + run app).
- Record retest findings and open follow-up issues only for reproducible regressions.

## Remaining Priority B (Behavior Quality)

- Create and maintain a mislabel inventory (wrong mode/icon/line classification).
- Execute one bulk normalization pass based on that inventory.
- Regression-check representative itineraries after normalization updates.

## Remaining Priority C (Maintenance)

- Periodically sync high-value upstream bugfixes into this fork and validate in USB runtime.
- Keep docs snapshots current when state changes (for example CI status snapshots and divergence stats).

## Completed Backlog (Issues `#1`-`#20`)

- `#1` Validate all Svelte interactive controls in IPC mode.
- `#2` Close remaining IPC endpoint passthrough gaps.
- `#3` Remove legacy protocol stubs after coverage is complete.
- `#4` Harden FAT32 USB launcher behavior across Linux environments.
- `#5` Improve `motis-ipc` lifecycle and crash recovery handling.
- `#6` Improve startup/data-path diagnostics for portable runs.
- `#7` Polish native UI reliability (loading, errors, small screens).
- `#8` Prevent global touchpad pinch-zoom in Tauri webview.
- `#9` Fix STOP/place-id handling for plan endpoint in protocol mode.
- `#10` Fix global paste handler crash on non-JSON clipboard input.
- `#11` Remove offline-mode geolocation button.
- `#12` Fix ineffective Tauri Cargo target rustflags config.
- `#13` Fix protocol routing for debug panel endpoints.
- `#14` Guard missing-pointer cases in `native::api_get`.
- `#15` Remove `String::leak` in Svelte init path resolution.
- `#16` Fix direct date/time typing behavior.
- `#17` Add global language toggle (English/Swedish).
- `#18` Add city-specific tram/metro enrichment.
- `#19` Deprecate localhost fallback in native runtime.
- `#20` Fix via-stop selection usability regression.

## Guardrails

- Do not reintroduce localhost/server fallback into the native Svelte app.
- Do not regress offline + firewall-restricted operation.
- Keep docs aligned with the actually shipped USB workflow.
