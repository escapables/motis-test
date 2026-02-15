# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

- No changes yet.

## [2.9.0] - 2026-02-15

### Bug Fixes

- Fixed IPC startup error handling so `init` failures return structured errors instead of ambiguous failure output.
- Fixed native wrapper exception paths so route/geocode failures are surfaced as errors, not empty-success payloads.
- Fixed isochrones and connections debounce/tab-exit races by canceling stale work and ignoring out-of-generation responses.
- Fixed departures tab-switch regression by preserving `stopId` state and auto-resolving existing `STOP` selections.
- Fixed Linux debug coverage setup with POSIX-safe virtualenv commands and deterministic gcov/llvm-cov tool selection.

### Feature Implementations

- Added improved route-search controls: map-pick buttons in origin/destination inputs and centered swap control.
- Changed isochrones default last-leg time from 15 minutes to 5 minutes when no explicit value is set.
- Added release/test workflow helpers with stricter docs-validation and gate commands for repeatable session checks.
- Added and standardized workflow docs for release, handoff, and read-order guidance.
- Added explicit GitHub-only release policy in `docs/RELEASING.md` (review/remediation DONE, CI green, `gh release create` trigger).

### Optimization and Refactor

- Extracted shared UI request-state/query helpers and reused them across planner flows to reduce state-handling duplication.
- Split route UI logic into focused modules/components, reducing `+page.svelte` from `1071` LOC to `493` LOC.
- Split native Tauri logic into focused modules and deduplicated endpoint wrapper boilerplate with shared execution helpers.
- Reduced CI runtime variance by installing coverage tooling only on debug matrix jobs.

### Security Hardening

- Hardened IPC command validation with single-line enforcement and `64 KiB` payload limits before subprocess writes.
- Replaced predictable temp executable staging with private unique temp directories and strict executable permissions.
- Replaced manual geocode JSON interpolation with serializer-based command construction and control-char regression checks.
- Hardened CI workflow security with least-privilege permissions, SHA-pinned actions, and removal of publish job from CI.

### Testing and CI

- Added Rust IPC crash/restart recovery regression `native::tests::command_retries_after_ipc_crash_and_recovers` to guard backend restart + command retry behavior.
- Added Rust protocol/native regression coverage in CI and a Playwright `@regression` slice for state/race scenarios.
- Added native IPC integration regressions for startup init-throw JSON behavior and wrapper exception contracts.
- Revalidated post-refactor step-7 remediation and refreshed step-8 review deliverables with updated severity/file-line evidence.
- Completed step-9 post-refactor re-review gate: `./bin/test-gate` green plus rerun native C++/Rust and Playwright smoke+regression slices.
- Completed fresh clean-tree scenario-5 verification: rebuilt `build-todo-debug`, reran native wrapper regressions (`2/2`), and passed `import -> generate -> batch -> compare` with `consumed: 20` / `equal: 20`.
- Reset release-preflight checklist to concrete rerun steps (fresh clean-tree build, copied-path USB simulation, full gate/CI confirmation) before tag/release.
- Re-ran release-preflight technical checks on fresh artifacts: clean `build-release-check` compile, native wrapper regression filter (`2/2`), copied-path USB launcher self-test, and copied-path desktop manual route-query smoke.
- Reconfirmed pre-tag gate and CI status: fixed `docs/TODO.md` validation schema drift, reran `./bin/test-gate` (`26/26` unit tests), reran Playwright smoke (`1/1`) and regression (`2/2`), and verified latest `master` `CI` run remains green.
- Fixed GitHub Actions workflow push-trigger evaluation by guarding pull-request-only Docker job conditions with an explicit event check.
