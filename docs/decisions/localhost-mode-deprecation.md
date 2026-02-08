# Decision: Deprecate Localhost Mode for Native App

Date: 2026-02-08  
Issue: `#19` (Evaluate deprecating localhost mode in favor of Svelte protocol mode)

## Decision

Use a single native runtime path for the desktop app:

- Keep `motis://` + IPC (`motis-ipc`) as the only backend path for `motis-gui-svelte`.
- Remove localhost/server-mode fallback code from the Svelte native backend.

Legacy browser/server workflows can still exist as separate tooling, but they are not part of the native app runtime contract.

## Why

- Dual mode increased maintenance and behavior drift risk.
- The shipped portable app is already designed around offline IPC startup (`RUN.sh`, `MOTIS_IPC_PATH`, `MOTIS_DATA_PATH`).
- Localhost fallback did not provide reliable value for the target USB/offline use case.

## Scenario Check

- Scenarios that still need localhost:
  - Browser-only workflows and server-centric local development.
  - These remain out of scope for native app runtime and should use dedicated web/server setup docs.
- Debug endpoint gaps:
  - Some debug-only endpoints are intentionally unsupported in protocol mode.
  - This is accepted for portable production use.

## Migration Plan

1. Remove server-mode code paths from `gui-svelte/src-tauri/src/native.rs`.
2. Keep protocol errors explicit for endpoints unsupported in portable IPC mode.
3. Update docs to state IPC-only native runtime.
4. Communicate that localhost workflows are legacy/development-only.

## Rollback Strategy

If a critical workflow is blocked after release:

1. Reintroduce server-mode branches in `gui-svelte/src-tauri/src/native.rs` from git history.
2. Restore fallback wording in docs.
3. Ship as a targeted hotfix and reopen follow-up issue with explicit owner/scope.

## Implementation Checklist

- [x] Code: remove localhost/server fallback branches from Svelte native backend.
- [x] Code: update protocol unsupported-endpoint messaging for IPC-only runtime.
- [x] Docs: update fork docs to reflect IPC-only native runtime.
- [x] User migration: state localhost workflows as legacy/development-only, not native runtime.
