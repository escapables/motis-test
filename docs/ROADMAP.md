# Portable App Roadmap

This is the focused backlog for the portable IPC-first app.

## Priority A

- Complete validation of all remaining interactive controls in Svelte UI.
- Add/fix protocol passthrough for any missing endpoints found during validation.
- Reduce legacy handler stubs in `protocol.rs` once coverage is confirmed.
- Disable global Tauri/webview zoom + window scrolling behavior so zoom gestures only affect the map.

## Priority B

- Harden USB execution flow:
  - validate `/tmp` launcher behavior across more Linux environments.
  - improve process lifecycle checks (clean shutdown, crash handling).
- Improve user-facing diagnostics for missing data/config/startup failures.

## Priority C

- UI polish and reliability:
  - loading/error states.
  - small-screen behavior.
  - route details consistency.

## Completed Milestones

- IPC subprocess bridge (`motis-ipc`) operational.
- Svelte Tauri app operational (`motis-gui-svelte`).
- Vector tiles rendering in native app.
- Glyph label rendering in native app.
- Major API endpoint passthrough implemented.
- USB-root deployment workflow functional.

## Guardrails

- Do not reintroduce localhost/server fallback into the native Svelte app.
- Do not regress offline + firewall-restricted operation.
- Keep docs aligned with the actual shipped USB workflow.
